//! Helpers shared by the integration tests.

use axiom_core::{Probability, ProbabilityError, TransitionRowError};
use num_bigint::BigInt;
use num_rational::BigRational;
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?` instead of
/// panicking on setup errors.
#[derive(Debug, Error)]
pub enum TestError {
    /// A fraction used by the test is not a probability.
    #[error(transparent)]
    Probability(#[from] ProbabilityError),
    /// A row used by the test is not a transition row.
    #[error(transparent)]
    TransitionRow(#[from] TransitionRowError),
    /// An iterator ended before the requested term.
    #[allow(
        dead_code,
        reason = "each test binary compiles this shared module; only some use this variant"
    )]
    #[error("sequence ended before term {0}")]
    MissingTerm(usize),
}

/// The exact rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The probability `numerator / denominator`.
pub fn probability(numerator: i64, denominator: i64) -> Result<Probability, ProbabilityError> {
    Probability::try_from_fraction(numerator, denominator)
}
