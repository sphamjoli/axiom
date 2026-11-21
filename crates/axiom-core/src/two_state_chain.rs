use num_rational::BigRational;
use num_traits::Zero;
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
}
