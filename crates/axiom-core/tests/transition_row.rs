//! Validation of transition rows against the unit simplex.

mod common;

use axiom_core::{TransitionRow, TransitionRowError};
use common::{TestError, probability, ratio};

#[test]
fn accepts_a_row_that_sums_to_exactly_one() -> Result<(), TestError> {
    let row = TransitionRow::try_new(vec![probability(9, 10)?, probability(1, 10)?])?;
    assert_eq!(row.probabilities().len(), 2);
    Ok(())
}

#[test]
fn accepts_a_row_with_many_states() -> Result<(), TestError> {
    let quarters = vec![probability(1, 4)?; 4];
    assert_eq!(TransitionRow::try_new(quarters)?.probabilities().len(), 4);
    Ok(())
}

#[test]
fn rejects_an_empty_row() {
    assert_eq!(
        TransitionRow::try_new(Vec::new()),
        Err(TransitionRowError::Empty)
    );
}

#[test]
fn rejects_a_row_off_the_simplex_and_reports_the_exact_sum() -> Result<(), TestError> {
    assert_eq!(
        TransitionRow::try_new(vec![probability(9, 10)?, probability(2, 10)?]),
        Err(TransitionRowError::NotNormalised { sum: ratio(11, 10) })
    );
    assert_eq!(
        TransitionRow::try_new(vec![probability(1, 3)?, probability(1, 3)?]),
        Err(TransitionRowError::NotNormalised { sum: ratio(2, 3) })
    );
    Ok(())
}

#[test]
fn rejects_a_sum_that_floating_point_would_round_to_one() -> Result<(), TestError> {
    let almost = vec![
        probability(1, 3)?,
        probability(1, 3)?,
        probability(333, 1000)?,
    ];
    assert_eq!(
        TransitionRow::try_new(almost),
        Err(TransitionRowError::NotNormalised {
            sum: ratio(2999, 3000)
        })
    );
    Ok(())
}

#[test]
fn two_state_row_is_complement_then_probability() -> Result<(), TestError> {
    let row = TransitionRow::two_state(probability(1, 10)?);
    let values: Vec<_> = row
        .probabilities()
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    assert_eq!(values, vec![ratio(9, 10), ratio(1, 10)]);
    Ok(())
}

#[test]
fn every_error_has_a_distinct_message() {
    assert_eq!(
        TransitionRowError::Empty.to_string(),
        "transition row has no entries"
    );
    assert_eq!(
        TransitionRowError::NotNormalised { sum: ratio(11, 10) }.to_string(),
        "transition row sums to 11/10, not 1"
    );
}
