use std::iter;

use num_rational::BigRational;

/// The update `x' = factor * x + offset`, applied once per step.
///
/// For a two-state chain, `x` is the probability of occupying the second
/// state and `factor` is the contraction factor of the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineUpdate {
    factor: BigRational,
    offset: BigRational,
}

impl AffineUpdate {
    /// Creates the update `x' = factor * x + offset`.
    #[must_use]
    pub fn new(factor: BigRational, offset: BigRational) -> Self {
        Self { factor, offset }
    }

    /// The factor that multiplies the current value.
    #[must_use]
    pub fn factor(&self) -> &BigRational {
        &self.factor
    }

    /// The constant added at each step.
    #[must_use]
    pub fn offset(&self) -> &BigRational {
        &self.offset
    }

    /// Applies one step of the update to `value`.
    #[must_use]
    pub fn apply(&self, value: &BigRational) -> BigRational {
        &self.factor * value + &self.offset
    }

    /// The exact sequence `x_0, x_1, x_2, ...` starting from `start`.
    ///
    /// The iterator is unbounded and computes each term lazily; callers bound
    /// it, for example with [`Iterator::take`]. Numerators and denominators
    /// grow with the step count, so each term costs more than the last.
    pub fn iterate(&self, start: BigRational) -> impl Iterator<Item = BigRational> + '_ {
        iter::successors(Some(start), move |value| Some(self.apply(value)))
    }
}
