//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use axiom_core::ProbabilityError;
use axiom_model::{Chain, CheckedModel, ModelError, Retry};
use axiom_sim::SimError;
use num_bigint::BigInt;
use num_rational::BigRational;
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?`.
#[derive(Debug, Error)]
pub enum TestError {
    /// A model the test expects to be accepted was rejected.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// A fraction used by the test is not a probability.
    #[error(transparent)]
    Probability(#[from] ProbabilityError),
    /// The simulator refused its input.
    #[error(transparent)]
    Sim(#[from] SimError),
    /// The model has a different form than the test expects.
    #[error("model has an unexpected form")]
    Form,
    /// A result differs from the expected one.
    #[error("{0}")]
    Mismatch(String),
}

/// Fails with `message` unless `condition` holds.
pub fn ensure(condition: bool, message: impl Into<String>) -> Result<(), TestError> {
    if condition {
        Ok(())
    } else {
        Err(TestError::Mismatch(message.into()))
    }
}

/// The rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// Alice's links: up fails with 1/10, down is repaired with 3/10.
pub const LINKS: &str = "chain links {
  states up, down
  from up:   up 9/10, down 1/10
  from down: up 3/10, down 7/10
}
";

/// A request that succeeds with probability 3/10.
pub const RETRY: &str = "retry request {\n  success 3/10\n}\n";

/// The checked links chain.
pub fn links() -> Result<Chain, TestError> {
    match axiom_model::parse(LINKS)? {
        CheckedModel::Chain(chain) => Ok(chain),
        CheckedModel::Retry(_) => Err(TestError::Form),
    }
}

/// The checked retry.
pub fn retry() -> Result<Retry, TestError> {
    match axiom_model::parse(RETRY)? {
        CheckedModel::Retry(retry) => Ok(retry),
        CheckedModel::Chain(_) => Err(TestError::Form),
    }
}
