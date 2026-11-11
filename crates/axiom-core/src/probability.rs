use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use thiserror::Error;

/// An exact probability: a rational number between zero and one inclusive,
/// stored in lowest terms.
///
/// The value is fixed at construction, so every `Probability` in the program
/// satisfies the bound.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Probability(BigRational);

/// The reason a rational number is not a probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ProbabilityError {
    /// The fraction has a zero denominator.
    #[error("fraction has a zero denominator")]
    ZeroDenominator,
    /// The value is below zero.
    #[error("probability is below zero")]
    Negative,
    /// The value is above one.
    #[error("probability is above one")]
    AboveOne,
}

impl Probability {
    /// Validates that `value` lies between zero and one inclusive and stores
    /// it in lowest terms.
    ///
    /// The value is rebuilt from its numerator and denominator, so a fraction
    /// constructed without reduction, such as `2/4`, is stored as `1/2`.
    ///
    /// # Errors
    ///
    /// Returns [`ProbabilityError::ZeroDenominator`] for a zero denominator,
    /// and [`ProbabilityError::Negative`] or [`ProbabilityError::AboveOne`]
    /// when the value is out of range.
    pub fn try_new(value: &BigRational) -> Result<Self, ProbabilityError> {
        if value.denom().is_zero() {
            return Err(ProbabilityError::ZeroDenominator);
        }
        let reduced = BigRational::new(value.numer().clone(), value.denom().clone());
        if reduced < BigRational::zero() {
            return Err(ProbabilityError::Negative);
        }
        if reduced > BigRational::one() {
            return Err(ProbabilityError::AboveOne);
        }
        Ok(Self(reduced))
    }

    /// Builds the probability `numerator / denominator`.
    ///
    /// # Errors
    ///
    /// The errors of [`Probability::try_new`].
    pub fn try_from_fraction(numerator: i64, denominator: i64) -> Result<Self, ProbabilityError> {
        Self::try_new(&BigRational::new_raw(
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
