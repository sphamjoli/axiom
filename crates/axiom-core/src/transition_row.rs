use std::fmt;

use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::Probability;

/// One row of a transition matrix: the probabilities of moving from one
/// current state to each next state, in state order.
///
/// The entries sum to exactly one, so the row lies on the unit simplex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionRow(Vec<Probability>);

/// The reason a list of probabilities is not a transition row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionRowError {
    /// The row has no entries.
    Empty,
    /// The entries do not sum to one; `sum` is their exact total.
    NotNormalised {
        /// The exact sum of the entries.
        sum: BigRational,
    },
}

impl TransitionRow {
    /// Validates that `probabilities` is non-empty and sums to exactly one.
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
            return Err(TransitionRowError::NotNormalised { sum });
        }
        Ok(Self(probabilities))
    }

    /// The probabilities of each next state, in state order.
    #[must_use]
    pub fn probabilities(&self) -> &[Probability] {
        &self.0
    }
}

impl fmt::Display for TransitionRowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("transition row has no entries"),
            Self::NotNormalised { sum } => {
                write!(formatter, "transition row sums to {sum}, not 1")
            }
        }
    }
}

impl std::error::Error for TransitionRowError {}
