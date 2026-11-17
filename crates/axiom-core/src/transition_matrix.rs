use thiserror::Error;

use crate::TransitionRow;

/// A transition matrix: one [`TransitionRow`] per current state, each with one
/// entry per next state.
///
/// The matrix is square and non-empty, so a model with `n` states has `n` rows
/// of `n` entries, and every row sums to exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionMatrix(Vec<TransitionRow>);

/// The reason a list of rows is not a transition matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransitionMatrixError {
    /// The matrix has no rows.
    #[error("transition matrix has no rows")]
    Empty,
    /// A row's length differs from the number of states.
    #[error("row {row} has {entries} entries, but the model has {states} states")]
    NotSquare {
        /// The index of the rejected row.
        row: usize,
        /// The number of entries in that row.
        entries: usize,
        /// The number of states, which is the number of rows.
        states: usize,
    },
}

impl TransitionMatrix {
    /// Validates that `rows` is non-empty and square.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionMatrixError::Empty`] for no rows and
    /// [`TransitionMatrixError::NotSquare`] for the first row whose length
    /// differs from the number of rows.
    pub fn try_new(rows: Vec<TransitionRow>) -> Result<Self, TransitionMatrixError> {
        if rows.is_empty() {
            return Err(TransitionMatrixError::Empty);
        }
        let states = rows.len();
        if let Some((row, ragged)) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| row.probabilities().len() != states)
        {
            return Err(TransitionMatrixError::NotSquare {
                row,
                entries: ragged.probabilities().len(),
                states,
            });
        }
        Ok(Self(rows))
    }

    /// A two-state matrix from rows built by [`TransitionRow::two_state`],
    /// which is square by construction.
    pub(crate) fn two_state(rows: [TransitionRow; 2]) -> Self {
        Self(rows.into())
    }

    /// The rows, one per current state, in state order.
    #[must_use]
    pub fn rows(&self) -> &[TransitionRow] {
        &self.0
    }

    /// The number of states.
    #[must_use]
    pub fn states(&self) -> usize {
        self.0.len()
    }
}
