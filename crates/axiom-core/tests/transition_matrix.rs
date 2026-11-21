//! Validation of transition matrices: non-empty and square.

mod common;

use axiom_core::{TransitionMatrix, TransitionMatrixError, TransitionRow};
use common::{TestError, probability};

fn halves() -> Result<TransitionRow, TestError> {
    Ok(TransitionRow::two_state(probability(1, 2)?))
}

#[test]
fn accepts_a_square_matrix_and_keeps_its_rows_in_order() -> Result<(), TestError> {
    let first = TransitionRow::two_state(probability(1, 10)?);
    let second = TransitionRow::two_state(probability(7, 10)?);
    let matrix = TransitionMatrix::try_new(vec![first.clone(), second.clone()])?;
    assert_eq!(matrix.states(), 2);
    assert_eq!(matrix.rows(), [first, second].as_slice());
    Ok(())
}

#[test]
fn accepts_a_single_state_that_stays_put() -> Result<(), TestError> {
    let stay = TransitionRow::try_new(vec![probability(1, 1)?])?;
    assert_eq!(TransitionMatrix::try_new(vec![stay])?.states(), 1);
    Ok(())
}

#[test]
fn rejects_an_empty_matrix() {
    assert_eq!(
        TransitionMatrix::try_new(Vec::new()),
        Err(TransitionMatrixError::Empty)
    );
}

#[test]
fn rejects_a_ragged_matrix_and_names_the_first_bad_row() -> Result<(), TestError> {
    let stay = TransitionRow::try_new(vec![probability(1, 1)?])?;
    assert_eq!(
        TransitionMatrix::try_new(vec![halves()?, stay]),
        Err(TransitionMatrixError::NotSquare {
            row: 1,
            entries: 1,
            states: 2
        })
    );
    Ok(())
}

#[test]
fn rejects_rows_longer_than_the_number_of_states() -> Result<(), TestError> {
    assert_eq!(
        TransitionMatrix::try_new(vec![halves()?]),
        Err(TransitionMatrixError::NotSquare {
            row: 0,
            entries: 2,
            states: 1
        })
    );
    Ok(())
}

#[test]
fn every_error_names_what_went_wrong() {
    assert_eq!(
        TransitionMatrixError::Empty.to_string(),
        "transition matrix has no rows"
    );
    assert_eq!(
        TransitionMatrixError::NotSquare {
            row: 1,
            entries: 1,
            states: 2
        }
        .to_string(),
        "row 1 has 1 entries, but the model has 2 states"
    );
}
