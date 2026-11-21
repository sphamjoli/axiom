//! Seeded simulation of a model for the browser.
//!
//! [`crate::simulate`] reads a model and a JSON request, runs `axiom-sim` on the
//! model with the request's seed, and returns what the page plots. All the
//! sampling is done by `axiom-sim`; this module validates the request,
//! bounds the work and shapes the JSON.
//!
//! The caps below keep one call short enough that a browser tab cannot be
//! hung by it: at most 200 000 samples of at most 1000 steps or attempts
//! each, from a request of at most 1024 bytes and a model of at most 64 KiB.
//!
//! An estimate is reported only when the sample count meets the Hoeffding
//! bound for the requested ε and δ. A run with fewer samples still returns
//! its samples, with `estimate` null and the count the bound needs in
//! `requiredSamples`.

use axiom_core::{Probability, TransitionMatrix};
use axiom_model::CheckedModel;
use axiom_sim::{
    Certainty, Estimate, SimError, chain_matrix, required_samples, retry_probability, sample_chain,
    sample_paths, sample_retry,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Diagnostic, Fraction, two_state};

/// The longest request accepted, in bytes.
pub const MAX_REQUEST_BYTES: usize = 1024;

/// The longest model accepted, in bytes.
pub const MAX_SOURCE_BYTES: usize = 65_536;

/// The most samples one request may ask for.
pub const MAX_SAMPLES: u64 = 200_000;

/// The most steps of a chain trajectory, the same cap as a certificate's.
pub const MAX_STEPS: u64 = 1000;

/// The most attempts of one retry run.
pub const MAX_ATTEMPTS: u64 = 1000;

/// The sample paths returned for a chain.
pub const PATHS: usize = 5;

/// The most steps of each returned sample path.
pub const MAX_PATH_STEPS: u64 = 200;

/// The most points of the running fraction.
pub const MAX_RUNNING_POINTS: u64 = 200;

/// The most histogram buckets, one per attempt count from 1.
pub const MAX_HISTOGRAM_BUCKETS: u64 = 40;

/// The most decimal digits of each part of ε and δ.
const MAX_FRACTION_DIGITS: usize = 9;

/// The decimal digits of `u64::MAX`.
const MAX_SEED_DIGITS: usize = 20;

/// The default tolerance, 1/100.
const DEFAULT_EPSILON: (u32, u32) = (1, 100);

/// The default failure probability, 1/1000.
const DEFAULT_DELTA: (u32, u32) = (1, 1000);

/// A request the engine refuses before simulating. Every message is fixed
/// and never repeats the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RequestError {
    /// The request is longer than [`MAX_REQUEST_BYTES`].
    #[error("the request is longer than 1024 bytes")]
    RequestTooLarge,
    /// The model is longer than [`MAX_SOURCE_BYTES`].
    #[error("the model is longer than 65536 bytes")]
    SourceTooLarge,
    /// The request is not JSON of the expected shape.
    #[error("the request is not a valid simulation request")]
    Malformed,
    /// The seed is not a decimal integer that fits in 64 bits.
    #[error("the seed must be a decimal integer from 0 to 18446744073709551615")]
    InvalidSeed,
    /// The sample count is outside `1..=MAX_SAMPLES`.
    #[error("samples must lie between 1 and 200000")]
    SamplesOutOfRange,
    /// The step count is outside `1..=MAX_STEPS`.
    #[error("steps must lie between 1 and 1000")]
    StepsOutOfRange,
    /// The attempt limit is outside `1..=MAX_ATTEMPTS`.
    #[error("max attempts must lie between 1 and 1000")]
    AttemptsOutOfRange,
    /// ε or δ is not a fraction of short decimal integers.
    #[error("epsilon and delta must be fractions of decimal integers of at most 9 digits")]
    InvalidFraction,
}

impl RequestError {
    /// A stable kebab-case name for the error, the same on every machine.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestTooLarge => "request-too-large",
            Self::SourceTooLarge => "source-too-large",
            Self::Malformed => "malformed-request",
            Self::InvalidSeed => "invalid-seed",
            Self::SamplesOutOfRange => "samples-out-of-range",
            Self::StepsOutOfRange => "steps-out-of-range",
            Self::AttemptsOutOfRange => "attempts-out-of-range",
            Self::InvalidFraction => "invalid-fraction",
        }
    }
}

/// The request as the page sends it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawRequest {
    seed: String,
    samples: u64,
    steps: u64,
    max_attempts: u64,
    epsilon: Option<RawFraction>,
    delta: Option<RawFraction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFraction {
    num: String,
    den: String,
}

/// A validated request.
struct Request {
    seed: u64,
    samples: u64,
    steps: u64,
    max_attempts: u64,
    epsilon: BigRational,
    delta: BigRational,
}

fn in_range(value: u64, max: u64, error: RequestError) -> Result<u64, RequestError> {
    if (1..=max).contains(&value) {
        Ok(value)
    } else {
        Err(error)
    }
}

fn decimal(text: &str, max_digits: usize) -> Option<u64> {
    let valid = !text.is_empty()
        && text.len() <= max_digits
        && text.bytes().all(|byte| byte.is_ascii_digit());
    if valid { text.parse().ok() } else { None }
}

fn fraction(raw: Option<RawFraction>, default: (u32, u32)) -> Result<BigRational, RequestError> {
    let Some(raw) = raw else {
        return Ok(BigRational::new(default.0.into(), default.1.into()));
    };
    let num = decimal(&raw.num, MAX_FRACTION_DIGITS).ok_or(RequestError::InvalidFraction)?;
    let den = decimal(&raw.den, MAX_FRACTION_DIGITS)
        .filter(|den| *den != 0)
        .ok_or(RequestError::InvalidFraction)?;
    Ok(BigRational::new(num.into(), den.into()))
}

fn parse_request(json: &str) -> Result<Request, RequestError> {
    if json.len() > MAX_REQUEST_BYTES {
        return Err(RequestError::RequestTooLarge);
    }
    let raw: RawRequest = serde_json::from_str(json).map_err(|_| RequestError::Malformed)?;
    Ok(Request {
        seed: decimal(&raw.seed, MAX_SEED_DIGITS).ok_or(RequestError::InvalidSeed)?,
        samples: in_range(raw.samples, MAX_SAMPLES, RequestError::SamplesOutOfRange)?,
        steps: in_range(raw.steps, MAX_STEPS, RequestError::StepsOutOfRange)?,
        max_attempts: in_range(
            raw.max_attempts,
            MAX_ATTEMPTS,
            RequestError::AttemptsOutOfRange,
        )?,
        epsilon: fraction(raw.epsilon, DEFAULT_EPSILON)?,
        delta: fraction(raw.delta, DEFAULT_DELTA)?,
    })
}

/// The outcome of [`crate::simulate`].
#[derive(Serialize)]
#[serde(untagged)]
enum SimulateResult {
    Ran {
        ok: bool,
        simulation: Box<Simulation>,
    },
    Refused {
        ok: bool,
        error: ErrorView,
    },
}

/// Why a simulation did not run: a stable code, a fixed message, and for a
/// model error the line and column.
#[derive(Serialize)]
struct ErrorView {
    code: &'static str,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

enum Failure {
    Request(RequestError),
    Model(Diagnostic),
    Sim(SimError),
}

impl From<RequestError> for Failure {
    fn from(error: RequestError) -> Self {
        Self::Request(error)
    }
}

impl From<SimError> for Failure {
    fn from(error: SimError) -> Self {
        Self::Sim(error)
    }
}

impl From<Failure> for ErrorView {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Request(error) => Self {
                code: error.code(),
                message: error.to_string(),
                line: None,
                column: None,
            },
            Failure::Sim(error) => Self {
                code: error.code(),
                message: error.to_string(),
                line: None,
                column: None,
            },
            Failure::Model(diagnostic) => Self {
                code: diagnostic.code,
                message: diagnostic.message,
                line: Some(diagnostic.line),
                column: Some(diagnostic.column),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Simulation {
    name: String,
    seed: String,
    samples: u64,
    required_samples: u64,
    epsilon: Fraction,
    delta: Fraction,
    estimate: Option<EstimateView>,
    chain: Option<ChainRun>,
    retry: Option<RetryRun>,
}

#[derive(Serialize)]
struct EstimateView {
    value: Fraction,
    approx: f64,
    samples: u64,
    epsilon: Fraction,
    delta: Fraction,
    certainty: &'static str,
}

#[derive(Serialize)]
struct RunningPoint {
    samples: u64,
    approx: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChainRun {
    states: Vec<String>,
    state: String,
    steps: u64,
    paths: Vec<Vec<usize>>,
    running: Vec<RunningPoint>,
    fraction: Fraction,
    steady_state: Option<Fraction>,
    steady_approx: Option<f64>,
}

#[derive(Serialize)]
struct Bucket {
    attempts: u64,
    count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RetryRun {
    max_attempts: u64,
    success: Fraction,
    histogram: Vec<Bucket>,
    beyond: u64,
    failures: u64,
    success_fraction: Fraction,
    mean_attempts: Option<Fraction>,
    mean_approx: Option<f64>,
    expected_attempts: Option<Fraction>,
    expected_approx: Option<f64>,
}

/// The JSON for [`crate::simulate`].
pub(crate) fn simulate(source: &str, request_json: &str) -> String {
    let result = match run(source, request_json) {
        Ok(simulation) => SimulateResult::Ran {
            ok: true,
            simulation: Box::new(simulation),
        },
        Err(failure) => SimulateResult::Refused {
            ok: false,
            error: ErrorView::from(failure),
        },
    };
    serde_json::to_string(&result).unwrap_or_else(|_| {
        String::from(
            r#"{"ok":false,"error":{"code":"internal","message":"the engine could not encode its result","line":null,"column":null}}"#,
        )
    })
}

fn run(source: &str, request_json: &str) -> Result<Simulation, Failure> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(RequestError::SourceTooLarge.into());
    }
    let request = parse_request(request_json)?;
    let model = axiom_model::parse(source)
        .map_err(|error| Failure::Model(Diagnostic::from_error(&error)))?;
    let required = required_samples(&request.epsilon, &request.delta)?;
    let (name, hits, chain, retry) = match &model {
        CheckedModel::Chain(chain) => {
            let (hits, run) = run_chain(&chain.states, chain_matrix(chain)?, &request)?;
            (chain.name.clone(), hits, Some(run), None)
        }
        CheckedModel::Retry(retry) => {
            let (hits, run) = run_retry(retry_probability(retry)?, &request);
            (retry.name.clone(), hits, None, Some(run))
        }
    };
    let estimate = match Estimate::from_hits(
        hits,
        request.samples,
        request.epsilon.clone(),
        request.delta.clone(),
    ) {
        Ok(estimate) => Some(estimate_view(&estimate)),
        Err(SimError::TooFewSamples { .. }) => None,
        Err(error) => return Err(error.into()),
    };
    Ok(Simulation {
        name,
        seed: request.seed.to_string(),
        samples: request.samples,
        required_samples: required,
        epsilon: Fraction::from(&request.epsilon),
        delta: Fraction::from(&request.delta),
        estimate,
        chain,
        retry,
    })
}

fn estimate_view(estimate: &Estimate) -> EstimateView {
    EstimateView {
        value: Fraction::from(&estimate.value),
        approx: approx(&estimate.value),
        samples: estimate.samples,
        epsilon: Fraction::from(&estimate.epsilon),
        delta: Fraction::from(&estimate.delta),
        certainty: match estimate.certainty() {
            Certainty::Proven => "proven",
            Certainty::Estimated => "estimated",
        },
    }
}

fn approx(value: &BigRational) -> f64 {
    value.to_f64().unwrap_or(f64::NAN)
}

fn ratio(num: u64, den: u64) -> BigRational {
    BigRational::new(BigInt::from(num), BigInt::from(den.max(1)))
}

fn index(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Samples the chain from its first state and counts the trajectories that
/// end in the second state, the state the steady state is stated for.
fn run_chain(
    states: &[String],
    matrix: &TransitionMatrix,
    request: &Request,
) -> Result<(u64, ChainRun), SimError> {
    let tracked = usize::from(states.len() > 1);
    let samples = sample_chain(
        matrix,
        0,
        index(request.steps),
        index(request.samples),
        request.seed,
    )?;
    let paths = sample_paths(
        matrix,
        0,
        index(request.steps.min(MAX_PATH_STEPS)),
        PATHS,
        request.seed,
    )?;
    let points = request.samples.min(MAX_RUNNING_POINTS);
    let mut running = Vec::with_capacity(index(points));
    let mut hits = 0_u64;
    let mut next = 1_u64;
    for (k, state) in (1_u64..).zip(&samples.final_states) {
        if *state == tracked {
            hits += 1;
        }
        // Point `next` of `points` falls at sample ⌈next · n / points⌉.
        if k * points >= next * request.samples {
            running.push(RunningPoint {
                samples: k,
                approx: approx(&ratio(hits, k)),
            });
            next += 1;
        }
    }
    let steady = two_state(matrix).and_then(|chain| {
        let settles = chain.contraction_factor().abs() < BigRational::one();
        chain.steady_state().ok().filter(|_| settles)
    });
    let run = ChainRun {
        states: states.to_vec(),
        state: states.get(tracked).cloned().unwrap_or_default(),
        steps: request.steps,
        paths,
        running,
        fraction: Fraction::from(&ratio(hits, request.samples)),
        steady_approx: steady.as_ref().map(|p| approx(p.value())),
        steady_state: steady.as_ref().map(|p| Fraction::from(p.value())),
    };
    Ok((hits, run))
}

/// Samples the retry and counts the runs that succeed on the first attempt,
/// whose fraction estimates the success probability per attempt.
fn run_retry(success: &Probability, request: &Request) -> (u64, RetryRun) {
    let samples = sample_retry(
        success,
        request.max_attempts,
        index(request.samples),
        request.seed,
    );
    let buckets = request.max_attempts.min(MAX_HISTOGRAM_BUCKETS);
    let mut counts = vec![0_u64; index(buckets)];
    for attempt in samples.attempts.iter().flatten() {
        if let Some(count) = counts.get_mut(index(attempt.saturating_sub(1))) {
            *count += 1;
        }
    }
    let successes = samples.successes();
    let within = samples.successes_within(buckets);
    let mean = (successes > 0).then(|| ratio(samples.total_attempts(), successes));
    let p = success.value();
    let expected = (!p.is_zero()).then(|| p.recip());
    let run = RetryRun {
        max_attempts: request.max_attempts,
        success: Fraction::from(p),
        histogram: (1_u64..)
            .zip(counts)
            .map(|(attempts, count)| Bucket { attempts, count })
            .collect(),
        beyond: successes - within,
        failures: request.samples - successes,
        success_fraction: Fraction::from(&ratio(successes, request.samples)),
        mean_approx: mean.as_ref().map(approx),
        mean_attempts: mean.as_ref().map(Fraction::from),
        expected_approx: expected.as_ref().map(approx),
        expected_attempts: expected.as_ref().map(Fraction::from),
    };
    (samples.successes_within(1), run)
}
