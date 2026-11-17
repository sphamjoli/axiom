//! Helpers shared by the integration tests.

use axiom_cert::CertificateError;
use axiom_core::{Probability, ProbabilityError, TwoStateChain};
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?` instead of
/// panicking on setup errors.
#[derive(Debug, Error)]
pub enum TestError {
    /// A fraction used by the test is not a probability.
    #[error(transparent)]
    Probability(#[from] ProbabilityError),
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
