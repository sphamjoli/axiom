use num_rational::BigRational;
use num_traits::{Signed, Zero};
use thiserror::Error;

use crate::{AffineUpdate, Probability, TransitionMatrix, TransitionRow};

/// The reason a two-state chain has no unique steady state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TwoStateChainError {
    /// Both exit probabilities are zero, so the chain never moves and every
    /// distribution is a fixed point.
    #[error("chain never leaves either state, so it has no unique fixed point")]
    NoUniqueFixedPoint,
    /// Both exit probabilities are one, so the chain alternates between the
    /// states forever and the update, with factor `-1`, is not a contraction.
    #[error("chain alternates between its states, so its update is not a contraction")]
    NotContracting,
}

/// A chain with two states that moves between them at fixed probabilities.
///
/// From the first state it moves to the second with probability
/// `leave_first`; from the second it moves to the first with probability
/// `leave_second`. In the link model, the states are *up* and *down*,
/// `leave_first` is the failure probability and `leave_second` is the repair
/// probability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoStateChain {
    leave_first: Probability,
    leave_second: Probability,
}

impl TwoStateChain {
    /// Creates the chain from its two exit probabilities.
    #[must_use]
    pub fn new(leave_first: Probability, leave_second: Probability) -> Self {
        Self {
            leave_first,
            leave_second,
        }
    }

    /// The transition matrix with rows `[1 - a, a]` and `[b, 1 - b]`, where `a`
    /// is `leave_first` and `b` is `leave_second`.
    #[must_use]
    pub fn matrix(&self) -> TransitionMatrix {
        TransitionMatrix::two_state([
            TransitionRow::two_state(self.leave_first.clone()),
            TransitionRow::two_state(self.leave_second.complement()),
        ])
    }

    /// The update for the probability `d` of occupying the second state:
    /// `d' = (1 - b) d + a (1 - d) = (1 - a - b) d + a`.
    #[must_use]
    pub fn second_state_update(&self) -> AffineUpdate {
        AffineUpdate::new(self.contraction_factor(), self.leave_first.value().clone())
    }

    /// The contraction factor `1 - a - b` of [`Self::second_state_update`],
    /// which lies in `[-1, 1]`.
    #[must_use]
    pub fn contraction_factor(&self) -> BigRational {
        self.leave_first.complement().value() - self.leave_second.value()
    }

    /// The unique steady-state probability of occupying the second state,
    /// `d* = c / (1 - lambda) = a / (a + b)`.
    ///
    /// # Errors
    ///
    /// Returns [`TwoStateChainError::NoUniqueFixedPoint`] when `a + b = 0`.
    pub fn steady_state(&self) -> Result<Probability, TwoStateChainError> {
        let total = self.leave_first.value() + self.leave_second.value();
        if total.is_zero() {
            return Err(TwoStateChainError::NoUniqueFixedPoint);
        }
        Probability::try_new(&(self.leave_first.value() / total))
            .map_err(|_| TwoStateChainError::NoUniqueFixedPoint)
    }

    /// The least number of steps `n`, at most `limit`, after which the
    /// distance from the steady state is certified to be within `tolerance`
    /// for every later step: `|lambda|^n |start - d*| <= tolerance`, where
    /// `lambda` is [`Self::contraction_factor`] and `d*` is
    /// [`Self::steady_state`]. Returns `None` when no `n` up to `limit`
    /// suffices, for example for the periodic chain, whose factor is `-1`,
    /// unless `start` is already within `tolerance` of the steady state.
    ///
    /// The search multiplies exact rationals at most `limit` times, so the
    /// caller bounds the cost by choosing `limit`. For the link model,
    /// `start = 1` and `tolerance = 1/1000` give 13 steps.
    ///
    /// # Errors
    ///
    /// Returns [`TwoStateChainError::NoUniqueFixedPoint`] when `a + b = 0`.
    pub fn steps_within(
        &self,
        start: &BigRational,
        tolerance: &BigRational,
        limit: u64,
    ) -> Result<Option<u64>, TwoStateChainError> {
        let steady = self.steady_state()?;
        let factor = self.contraction_factor().abs();
        let distance = (start - steady.value()).abs();
        // Compare |p/q|^n (r/s) <= v/w as p^n r w <= v q^n s on integers, so
        // each step is one multiplication with no fraction reduction.
        let mut left = distance.numer() * tolerance.denom();
        let mut right = tolerance.numer() * distance.denom();
        for steps in 0..=limit {
            if left <= right {
                return Ok(Some(steps));
            }
            left *= factor.numer();
            right *= factor.denom();
        }
        Ok(None)
    }
}
