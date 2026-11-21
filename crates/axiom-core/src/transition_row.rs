use num_rational::BigRational;
use num_traits::{One, Zero};
use thiserror::Error;

use crate::Probability;

/// One row of a transition matrix: the probabilities of moving from one
/// current state to each next state, in state order.
///
/// A model may have any finite number of states, so a row may have any
/// positive length. The entries sum to exactly one, so the row lies on the
/// unit simplex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionRow(Vec<Probability>);

/// The reason a list of probabilities is not a transition row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransitionRowError {
    /// The row has no entries.
    #[error("transition row has no entries")]
    Empty,
    /// The entries do not sum to one. The sum itself is not kept: its
    /// numerator and denominator can be far larger than the entries.
    #[error("transition row does not sum to 1")]
    NotNormalised,
}

impl TransitionRowError {
    /// Every variant, in declaration order.
    pub const ALL: [Self; 2] = [Self::Empty, Self::NotNormalised];

    /// A stable kebab-case name for the error, the same on every machine.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Empty => "empty-row",
            Self::NotNormalised => "row-not-normalised",
        }
    }
}

impl TransitionRow {
    /// Validates that `probabilities` is non-empty and sums to exactly one.
    ///
    /// Summing fractions with unrelated denominators makes the running
    /// denominator grow towards their least common multiple, so the cost grows
    /// faster than linearly in the number and size of the entries. Callers that
    /// accept rows from untrusted input bound both before calling this.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionRowError::Empty`] for an empty row and
    /// [`TransitionRowError::NotNormalised`] when the sum differs from one.
    pub fn try_new(probabilities: Vec<Probability>) -> Result<Self, TransitionRowError> {
        if probabilities.is_empty() {
            return Err(TransitionRowError::Empty);
        }
        let sum = probabilities
            .iter()
            .fold(BigRational::zero(), |total, probability| {
                total + probability.value()
            });
        if !sum.is_one() {
            return Err(TransitionRowError::NotNormalised);
        }
        Ok(Self(probabilities))
    }

    /// The two-state row `[1 - p, p]`, where `p` is the probability of moving
    /// to the second state.
    ///
    /// The entries sum to one by construction, so no validation is needed.
    #[must_use]
    pub fn two_state(to_second: Probability) -> Self {
        Self(vec![to_second.complement(), to_second])
    }

    /// The probabilities of each next state, in state order.
    #[must_use]
    pub fn probabilities(&self) -> &[Probability] {
        &self.0
    }
}
