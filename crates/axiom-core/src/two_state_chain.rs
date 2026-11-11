use crate::{AffineUpdate, Probability, TransitionRow};

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

    /// The transition matrix rows `[1 - a, a]` and `[b, 1 - b]`, where `a` is
    /// `leave_first` and `b` is `leave_second`.
    #[must_use]
    pub fn rows(&self) -> [TransitionRow; 2] {
        [
            TransitionRow::two_state(self.leave_first.clone()),
            TransitionRow::two_state(self.leave_second.complement()),
        ]
    }

    /// The update for the probability `d` of occupying the second state:
    /// `d' = (1 - b) d + a (1 - d) = (1 - a - b) d + a`.
    #[must_use]
    pub fn second_state_update(&self) -> AffineUpdate {
        let factor = self.leave_first.complement().value() - self.leave_second.value();
        AffineUpdate::new(factor, self.leave_first.value().clone())
    }
}
