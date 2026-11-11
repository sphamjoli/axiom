use std::fmt;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// An exact probability: a rational number between zero and one inclusive.
///
/// The value is fixed at construction, so every `Probability` in the program
/// satisfies the bound.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Probability(BigRational);

/// The reason a rational number is not a probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbabilityError {
    /// The fraction has a zero denominator.
    ZeroDenominator,
    /// The value is below zero.
    Negative,
    /// The value is above one.
    AboveOne,
}

impl Probability {
    /// Validates that `value` lies between zero and one inclusive.
    ///
    /// # Errors
    ///
    /// Returns [`ProbabilityError::Negative`] or [`ProbabilityError::AboveOne`]
    /// when the value is out of range.
    pub fn try_new(value: BigRational) -> Result<Self, ProbabilityError> {
        if value < BigRational::zero() {
            return Err(ProbabilityError::Negative);
        }
        if value > BigRational::one() {
            return Err(ProbabilityError::AboveOne);
        }
        Ok(Self(value))
    }

    /// Builds the probability `numerator / denominator`.
    ///
    /// # Errors
    ///
    /// Returns [`ProbabilityError::ZeroDenominator`] when `denominator` is zero,
    /// and the errors of [`Probability::try_new`] when the fraction is out of
    /// range.
    pub fn try_from_fraction(numerator: i64, denominator: i64) -> Result<Self, ProbabilityError> {
        if denominator == 0 {
            return Err(ProbabilityError::ZeroDenominator);
        }
        Self::try_new(BigRational::new(
            BigInt::from(numerator),
            BigInt::from(denominator),
        ))
    }

    /// The probability of the complementary event, `1 - p`.
    #[must_use]
    pub fn complement(&self) -> Self {
        Self(BigRational::one() - &self.0)
    }

    /// The exact value, in lowest terms.
    #[must_use]
    pub fn value(&self) -> &BigRational {
        &self.0
    }
}

impl fmt::Display for ProbabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroDenominator => "fraction has a zero denominator",
            Self::Negative => "probability is below zero",
            Self::AboveOne => "probability is above one",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProbabilityError {}
