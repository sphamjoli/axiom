//! The rational interface that proof 3 trusts.
//!
//! The covered engine functions in [`crate::kernel`] see rational numbers only
//! through [`ExactRational`]. The translation to Lean keeps every item of this
//! module opaque and models it in Lean's `ℚ`, so each method must stay a
//! one-line delegation to `num-rational` whose meaning on rationals is obvious.

use num_rational::BigRational;
use num_traits::{One, Zero};

/// An exact rational number, seen by the covered functions only through the
/// operations below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactRational(BigRational);

impl ExactRational {
    /// Wraps a copy of `value`.
    pub(crate) fn from_ref(value: &BigRational) -> Self {
        Self(value.clone())
    }

    /// The wrapped value.
    pub(crate) fn into_big(self) -> BigRational {
        self.0
    }

    /// The rational one.
    pub(crate) fn one() -> Self {
        Self(BigRational::one())
    }

    /// The sum `self + other`.
    pub(crate) fn add(&self, other: &Self) -> Self {
        Self(&self.0 + &other.0)
    }

    /// The difference `self - other`.
    pub(crate) fn sub(&self, other: &Self) -> Self {
        Self(&self.0 - &other.0)
    }

    /// The product `self * other`.
    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self(&self.0 * &other.0)
    }

    /// The quotient `self / other`; callers must ensure `other` is not zero.
    pub(crate) fn div(&self, other: &Self) -> Self {
        Self(&self.0 / &other.0)
    }

    /// Whether the value is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}
