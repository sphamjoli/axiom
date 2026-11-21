//! The `analyse` command: reads a model and writes its certificate.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use axiom_cert::{Certificate, End, Estimate, Interval, Model, Quantity};
use axiom_core::Probability;
use axiom_model::{AffineWeight, Bound, Chain, CheckedModel, MAX_INPUT_BYTES, Parameter};
use num_traits::{One, Zero};

use crate::cli::EstimateRequest;
use crate::error::{CliError, FileRole, IoFailure};

/// What `analyse` wrote, for the summary line.
#[derive(Debug)]
pub struct Summary {
    /// The model's description, ending with its separator: `chain links: 2
    /// states, 2 rows;`, `parametric chain links:` or `retry request:`.
    model: String,
    claims: usize,
    cert: String,
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let noun = if self.claims == 1 { "claim" } else { "claims" };
        write!(
            formatter,
            "{} wrote {} {noun} to {}",
            self.model, self.claims, self.cert
        )
    }
}

/// The attempt count of the `failure_after` part of a `Termination` claim,
/// and the attempts each sampled retry may make for a `success` estimate.
const FAILURE_ATTEMPTS: u64 = 20;

/// The steps of each sampled trajectory for a `steady-state` estimate. The
/// trajectories start in the first state; after this many steps the links
/// model is within `0.6^64 < 10^-14` of its steady state.
pub(crate) const STEADY_STATE_STEPS: usize = 64;

/// Reads the model at `model` and writes a certificate to `cert`: for a
/// chain, a `Mass` claim for every row, plus `FixedPoint`, `Contraction` and
/// `ConvergenceBound` claims when the chain has two states and settles; for
/// a two-state chain whose exit probabilities are its two parameters, a
/// `ParametricContraction` claim; for a retry loop, a `Termination` claim.
///
/// # Errors
///
/// Returns an error when the model cannot be read or is invalid, when it is
/// a parametric retry or a parametric chain of another shape, when a
/// parametric range includes a chain that does not contract, or when the
/// certificate cannot be written. With `estimate`, it also returns the
/// errors of sampling: `--epsilon` or `--delta` out of range, a parametric
/// model, a sample count above the format limit or a chain without two
/// states.
pub fn run(
    model: &Path,
    cert: &Path,
    estimate: Option<&EstimateRequest>,
) -> Result<Summary, CliError> {
    let source = read_model(model)?;
    let (description, certificate) = match axiom_model::parse(&source)? {
        CheckedModel::Chain(chain) => {
            if let Some(matrix) = &chain.matrix {
                let states = matrix.states();
                (
                    format!("chain {}: {states} states, {states} rows;", chain.name),
                    Certificate::strongest(matrix.clone()),
                )
            } else {
                let (a, b) = parametric_ranges(&chain).ok_or(CliError::ParametricChain)?;
                (
                    format!("parametric chain {}:", chain.name),
                    Certificate::parametric(a, b)?,
                )
            }
        }
        CheckedModel::Retry(retry) => {
            let success = retry.probability.ok_or(CliError::ParametricRetry)?;
            (
                format!("retry {}:", retry.name),
                Certificate::termination(success, FAILURE_ATTEMPTS)?,
            )
        }
    };
    let certificate = match estimate {
        Some(request) => estimated(certificate, request)?,
        None => certificate,
    };
    let text = certificate.to_json()?;
    std::fs::write(cert, text).map_err(|error| CliError::WriteFailed {
        file: FileRole::Certificate,
        failure: IoFailure::from(&error),
    })?;
    Ok(Summary {
        model: description,
        claims: certificate.claims().len(),
        cert: cert.display().to_string(),
    })
}

/// `certificate` with an `Estimate` claim appended: `steady-state` for a
/// two-state chain, from trajectories of [`STEADY_STATE_STEPS`] steps that
/// start in the first state, or `success` for a retry, from runs of at most
/// [`FAILURE_ATTEMPTS`] attempts. The sample count is the least that meets
/// Hoeffding's bound for the requested tolerance and failure probability,
/// and it is checked against the format's limit before any sampling.
///
/// # Errors
///
/// Returns the simulation error when `epsilon` or `delta` is not strictly
/// between zero and one or the model has parameters, and the certificate
/// error when the sample count is above the limit or the chain does not
/// have two states.
fn estimated(certificate: Certificate, request: &EstimateRequest) -> Result<Certificate, CliError> {
    let samples = sample_count(&certificate, request)?;
    let index = certificate.claims().len();
    let count = usize::try_from(samples).map_err(|_| {
        CliError::Certificate(axiom_cert::CertificateError::SamplesOutOfRange {
            claim: index,
            limit: axiom_cert::MAX_SAMPLES,
        })
    })?;
    let (quantity, hits) = match certificate.model() {
        Model::Chain(matrix) if matrix.states() == 2 => {
            let runs = axiom_sim::sample_chain(matrix, 0, STEADY_STATE_STEPS, count, request.seed)?;
            (Quantity::SteadyState, runs.count(1))
        }
        Model::Chain(matrix) => {
            return Err(CliError::Certificate(
                axiom_cert::CertificateError::NotTwoState {
                    claim: index,
                    states: matrix.states(),
                },
            ));
        }
        Model::Retry { success } => {
            let runs = axiom_sim::sample_retry(success, FAILURE_ATTEMPTS, count, request.seed);
            (Quantity::Success, runs.successes())
        }
        _ => return Err(CliError::Simulation(axiom_sim::SimError::Parametric)),
    };
    with_sampled_estimate(certificate, quantity, hits, samples, request)
}

/// The least sample count that meets Hoeffding's bound for the request,
/// checked against the format's limit for the estimate that would be
/// appended to `certificate`, before any sampling.
///
/// # Errors
///
/// Returns the simulation error when `epsilon` or `delta` is not strictly
/// between zero and one, and the certificate error when the count is above
/// the limit.
pub(crate) fn sample_count(
    certificate: &Certificate,
    request: &EstimateRequest,
) -> Result<u64, CliError> {
    let samples = axiom_sim::required_samples(&request.epsilon, &request.delta)?;
    Estimate::check_samples(certificate.claims().len(), samples)?;
    Ok(samples)
}

/// `certificate` with the `Estimate` claim of `hits` successes out of
/// `samples` samples drawn with the request's seed appended.
///
/// # Errors
///
/// Returns the simulation error when the count does not meet the bound or
/// `hits` exceeds it, and the certificate error when the reader would
/// refuse the claim.
pub(crate) fn with_sampled_estimate(
    certificate: Certificate,
    quantity: Quantity,
    hits: u64,
    samples: u64,
    request: &EstimateRequest,
) -> Result<Certificate, CliError> {
    let mean = axiom_sim::Estimate::from_hits(
        hits,
        samples,
        request.epsilon.clone(),
        request.delta.clone(),
    )?;
    let value = Probability::try_new(&mean.value)
        .map_err(|_| CliError::Simulation(axiom_sim::SimError::HitsExceedSamples))?;
    Ok(certificate.with_estimate(Estimate {
        quantity,
        value,
        samples,
        epsilon: mean.epsilon,
        delta: mean.delta,
        seed: request.seed,
    })?)
}

/// The ranges of `a` and `b` for a two-state chain whose exit
/// probabilities are exactly two distinct parameters, `a` from the first
/// state and `b` from the second, or `None` for any other parametric chain.
/// The parameters are distinct because the model checker rejects a declared
/// parameter that is never used, and the diagonal needs no check because the
/// model checker has shown that every row sums to one.
fn parametric_ranges(chain: &Chain) -> Option<(Interval, Interval)> {
    let [first, second] = chain.rows.as_slice() else {
        return None;
    };
    let a = sole_parameter(first.get(1)?)?;
    let b = sole_parameter(second.first()?)?;
    if chain.parameters.len() != 2 {
        return None;
    }
    Some((
        interval(chain.parameters.get(a)?)?,
        interval(chain.parameters.get(b)?)?,
    ))
}

/// The index of the parameter a weight is exactly equal to, if any.
fn sole_parameter(weight: &AffineWeight) -> Option<usize> {
    if !weight.constant().is_zero() {
        return None;
    }
    let mut found = None;
    for (index, coefficient) in weight.coefficients().iter().enumerate() {
        if coefficient.is_one() && found.is_none() {
            found = Some(index);
        } else if !coefficient.is_zero() {
            return None;
        }
    }
    found
}

/// A parameter's range as a certificate interval.
fn interval(parameter: &Parameter) -> Option<Interval> {
    let end = |bound: &Bound| if bound.closed { End::Closed } else { End::Open };
    let probability = |bound: &Bound| Probability::try_new(&bound.value).ok();
    Interval::try_new(
        probability(&parameter.lower)?,
        end(&parameter.lower),
        probability(&parameter.upper)?,
        end(&parameter.upper),
    )
    .ok()
}

/// Reads a model file, refusing files larger than the model size limit
/// without reading more than one byte past it.
pub(crate) fn read_model(path: &Path) -> Result<String, CliError> {
    let io_error = |error: std::io::Error| CliError::ReadFailed {
        file: FileRole::Model,
        failure: IoFailure::from(&error),
    };
    let file = File::open(path).map_err(io_error)?;
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_INPUT_BYTES).unwrap_or(u64::MAX);
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::ModelTooLarge {
            limit: MAX_INPUT_BYTES,
        });
    }
    String::from_utf8(bytes).map_err(|_| CliError::ModelNotUtf8)
}
