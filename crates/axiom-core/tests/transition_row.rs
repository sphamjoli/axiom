//! Validation of transition rows against the unit simplex.

mod common;

use axiom_core::{TransitionRow, TransitionRowError};
use common::{probability, ratio};

#[test]
fn accepts_a_row_that_sums_to_exactly_one() {
    let row = TransitionRow::try_new(vec![probability(9, 10), probability(1, 10)])
        .expect("9/10 + 1/10 = 1");
    assert_eq!(row.probabilities().len(), 2);
}

#[test]
fn accepts_a_single_certain_transition() {
    assert!(TransitionRow::try_new(vec![probability(1, 1)]).is_ok());
}

#[test]
fn rejects_an_empty_row() {
    assert_eq!(
        TransitionRow::try_new(Vec::new()),
        Err(TransitionRowError::Empty)
    );
}

#[test]
fn rejects_a_row_off_the_simplex_and_reports_the_exact_sum() {
    assert_eq!(
        TransitionRow::try_new(vec![probability(9, 10), probability(2, 10)]),
        Err(TransitionRowError::NotNormalised { sum: ratio(11, 10) })
    );
    assert_eq!(
        TransitionRow::try_new(vec![probability(1, 3), probability(1, 3)]),
        Err(TransitionRowError::NotNormalised { sum: ratio(2, 3) })
    );
}

#[test]
fn rejects_a_sum_that_floating_point_would_round_to_one() {
    let thirds = vec![probability(1, 3), probability(1, 3), probability(333, 1000)];
    assert_eq!(
        TransitionRow::try_new(thirds),
        Err(TransitionRowError::NotNormalised {
            sum: ratio(2999, 3000)
        })
    );
}
