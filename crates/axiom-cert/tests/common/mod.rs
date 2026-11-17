//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use axiom_cert::CertificateError;
use axiom_core::{
    Probability, ProbabilityError, TransitionMatrixError, TransitionRowError, TwoStateChain,
};
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
}

/// The golden certificate for the link model, written by hand.
pub const LINKS_MASS: &str = include_str!("../fixtures/links-mass.json");

/// The link model: failure probability 1/10, repair probability 3/10.
pub fn link_chain() -> Result<TwoStateChain, ProbabilityError> {
    Ok(TwoStateChain::new(
        Probability::try_from_fraction(1, 10)?,
        Probability::try_from_fraction(3, 10)?,
    ))
}
