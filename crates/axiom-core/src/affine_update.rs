use std::iter;

use num_rational::BigRational;

/// The update `x' = factor * x + offset`, applied once per step.
///
/// For a two-state chain with exit probabilities `a` and `b`, `x` is the
/// probability of occupying the second state, `factor` is `1 - a - b` and lies
/// in `[-1, 1]`, and `offset` is `a`. The update moves every value towards a
/// single steady state only when the absolute value of `factor` is below one.
///
/// The type carries no probabilistic invariant of its own; it is built by
/// [`crate::TwoStateChain::second_state_update`], which guarantees the ranges
/// above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineUpdate {
    factor: BigRational,
    offset: BigRational,
}

impl AffineUpdate {
    pub(crate) fn new(factor: BigRational, offset: BigRational) -> Self {
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
    /// The iterator is unbounded and computes each term lazily; callers must
    /// bound it, for example with [`Iterator::take`]. The numerator and
    /// denominator of the n-th term can each grow to a size proportional to
    /// n digits, so producing n terms costs time roughly quadratic in n and
    /// memory linear in n per term. There is no internal cap, so callers that
    /// accept a step count from untrusted input must limit it themselves.
    pub fn iterate(&self, start: BigRational) -> impl Iterator<Item = BigRational> + '_ {
        iter::successors(Some(start), move |value| Some(self.apply(value)))
    }
}
