//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use std::io;

use axiom_cert::CertificateError;
use axiom_core::{
    Probability, ProbabilityError, TransitionMatrixError, TransitionRowError, TwoStateChain,
    TwoStateChainError,
};
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
    /// A matrix used by the test is not a transition matrix.
    #[error(transparent)]
    TransitionMatrix(#[from] TransitionMatrixError),
    /// A certificate used by the test could not be written or read.
    #[error(transparent)]
    Certificate(#[from] CertificateError),
    /// A chain used by the test has no fixed point or contraction
    /// certificate.
    #[error(transparent)]
    TwoStateChain(#[from] TwoStateChainError),
    /// A fixture could not be read.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A certificate that must be rejected was accepted.
    #[error("certificate was accepted but must be rejected")]
    Accepted,
    /// A line of the fixture reason table is malformed.
    #[error("fixture reason table line is malformed")]
    Fixture,
}

/// The golden certificate for the link model, written by hand.
pub const LINKS_MASS: &str = include_str!("../../../../fixtures/certificates/links-mass.json");

/// The golden version 2 certificate for the link model: `Mass` for both
/// rows, steady state 1/4 and contraction factor 3/5, written by hand.
pub const LINKS_FIXED_POINT: &str =
    include_str!("../../../../fixtures/certificates/links-fixed-point.json");

/// The exact rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The link model: failure probability 1/10, repair probability 3/10.
pub fn link_chain() -> Result<TwoStateChain, ProbabilityError> {
    Ok(TwoStateChain::new(
        Probability::try_from_fraction(1, 10)?,
        Probability::try_from_fraction(3, 10)?,
    ))
}
