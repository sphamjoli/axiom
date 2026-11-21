//! The engine step the harness times: parse a model and build its
//! certificate, as `axiom analyse` does, with the convergence tolerance the
//! case asks for.

use axiom_cert::{Certificate, FAILURE_ATTEMPTS, Interval};
use axiom_core::{Probability, TwoStateChain};
use axiom_model::{Chain, CheckedModel, Parameter};
use num_rational::BigRational;

/// What the engine made of a model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "one value per case, moved once, so the larger size is paid once"
)]
pub enum Built {
    /// The certificate of the strongest claims the engine can make.
    Certificate(Certificate),
    /// The engine refused the model, with the refusal's stable code.
    Refused(&'static str),
}

/// Parses `source` and builds its certificate. A two-state chain with a
/// unique, attracting steady state gets a convergence bound from start one
/// to `tolerance`; any other chain gets [`Certificate::strongest`]; a
/// parametric two-state chain whose exits are its parameters gets
/// [`Certificate::parametric`]; a retry gets [`Certificate::termination`].
#[must_use]
pub fn build(source: &str, tolerance: &BigRational) -> Built {
    let model = match axiom_model::parse(source) {
        Ok(model) => model,
        Err(error) => return Built::Refused(error.code()),
    };
    match model {
        CheckedModel::Chain(chain) => match &chain.matrix {
            Some(matrix) => Built::Certificate(chain_certificate(matrix, tolerance)),
            None => match parametric_ranges(&chain) {
                Some((a, b)) => match Certificate::parametric(a, b) {
                    Ok(certificate) => Built::Certificate(certificate),
                    Err(error) => Built::Refused(error.code()),
                },
                None => Built::Refused("parametric-chain"),
            },
        },
        CheckedModel::Retry(retry) => match retry.probability {
            Some(success) => match Certificate::termination(success, FAILURE_ATTEMPTS) {
                Ok(certificate) => Built::Certificate(certificate),
                Err(error) => Built::Refused(error.code()),
            },
            None => Built::Refused("parametric-retry"),
        },
    }
}

fn chain_certificate(
    matrix: &axiom_core::TransitionMatrix,
    tolerance: &BigRational,
) -> Certificate {
    if let [first, second] = matrix.rows()
        && let (Some(a), Some(b)) = (first.probabilities().get(1), second.probabilities().first())
    {
        let chain = TwoStateChain::new(a.clone(), b.clone());
        if let Ok(certificate) =
            Certificate::two_state_with_bound(&chain, &Probability::one(), tolerance)
        {
            return certificate;
        }
    }
    Certificate::strongest(matrix.clone())
}

/// The ranges of `a` and `b` for a two-state chain whose exit
/// probabilities are exactly its two parameters, as `axiom analyse` reads
/// them.
fn parametric_ranges(chain: &Chain) -> Option<(Interval, Interval)> {
    let (a, b) = chain.two_state_exits()?;
    let interval = |p: &Parameter| {
        Interval::from_bounds(
            &p.lower.value,
            p.lower.closed,
            &p.upper.value,
            p.upper.closed,
        )
        .ok()
    };
    Some((interval(a)?, interval(b)?))
}
