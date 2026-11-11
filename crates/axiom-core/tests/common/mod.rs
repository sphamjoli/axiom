//! Helpers shared by the integration tests.

use axiom_core::Probability;
use num_bigint::BigInt;
use num_rational::BigRational;

/// The exact rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The probability `numerator / denominator`, for fractions known to be valid.
pub fn probability(numerator: i64, denominator: i64) -> Probability {
    Probability::try_from_fraction(numerator, denominator)
        .expect("test fractions are chosen between zero and one")
}
