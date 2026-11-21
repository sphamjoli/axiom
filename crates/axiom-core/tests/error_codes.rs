//! Every error of the crate has a stable, distinct code and a fixed message.
//! The table pins both, so a renamed code or a reworded message fails here.

use std::collections::BTreeSet;

use axiom_core::{ProbabilityError, TransitionMatrixError, TransitionRowError, TwoStateChainError};

/// The code and message of one error.
type Entry = (&'static str, String);

fn entry(code: &'static str, error: &impl std::error::Error) -> Entry {
    (code, error.to_string())
}

fn every_error() -> Vec<Entry> {
    let mut entries = Vec::new();
    entries.extend(ProbabilityError::ALL.iter().map(|e| entry(e.code(), e)));
    entries.extend(TransitionRowError::ALL.iter().map(|e| entry(e.code(), e)));
    entries.extend(TwoStateChainError::ALL.iter().map(|e| entry(e.code(), e)));
    for error in [
        TransitionMatrixError::Empty,
        TransitionMatrixError::NotSquare {
            row: 1,
            entries: 1,
            states: 2,
        },
    ] {
        entries.push(entry(error.code(), &error));
    }
    entries
}

#[test]
fn every_error_has_a_stable_distinct_code_and_fixed_message() {
    let entries = every_error();
    let found: Vec<(&str, &str)> = entries
        .iter()
        .map(|(code, message)| (*code, message.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("zero-denominator", "fraction has a zero denominator"),
            ("probability-below-zero", "probability is below zero"),
            ("probability-above-one", "probability is above one"),
            ("empty-row", "transition row has no entries"),
            ("row-not-normalised", "transition row does not sum to 1"),
            (
                "no-unique-fixed-point",
                "chain never leaves either state, so it has no unique fixed point"
            ),
            (
                "not-contracting",
                "chain alternates between its states, so its update is not a contraction"
            ),
            ("empty-matrix", "transition matrix has no rows"),
            (
                "matrix-not-square",
                "row 1 has 1 entries, but the model has 2 states"
            ),
        ]
    );
    let codes: BTreeSet<&str> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes.len(), found.len(), "codes are distinct");
}
