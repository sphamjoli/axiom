//! Every analysis error has a stable, distinct code and a fixed message
//! built from counts and limits only. The table pins both.

use std::collections::BTreeSet;

use axiom_infer::InferError;

#[test]
fn every_error_has_a_stable_distinct_code_and_fixed_message() {
    let errors = [
        InferError::NotTwoState { states: 3 },
        InferError::ParameterCount {
            expected: 2,
            actual: 1,
        },
        InferError::InvalidInstance,
        InferError::TooManyPoints { limit: 5 },
        InferError::InvalidGrid { limit: 6 },
        InferError::TooManySamples { limit: 7 },
        InferError::StepBudgetExceeded { limit: 8 },
        InferError::StateOutOfRange,
        InferError::Sim(axiom_sim::SimError::InvalidDelta),
    ];
    let found: Vec<(&str, String)> = errors
        .iter()
        .map(|error| (error.code(), error.to_string()))
        .collect();
    let expected = [
        (
            "black-box-not-two-state",
            "the black box has 3 states, not two",
        ),
        ("parameter-count", "expected 2 parameter values, got 1"),
        (
            "invalid-instance",
            "the parameter values do not give a valid chain of the model",
        ),
        (
            "too-many-points",
            "more parameter points than the limit of 5",
        ),
        (
            "invalid-grid",
            "the coverage grid must have between 1 and 6 cells",
        ),
        (
            "too-many-samples",
            "the request needs more than the limit of 7 samples or steps",
        ),
        (
            "step-budget-exceeded",
            "both states were not visited often enough within 8 steps",
        ),
        (
            "black-box-state-out-of-range",
            "the black box stepped to a state outside the chain",
        ),
        (
            "invalid-delta",
            "delta must lie strictly between zero and one",
        ),
    ];
    let expected: Vec<(&str, String)> = expected
        .iter()
        .map(|(code, message)| (*code, (*message).to_owned()))
        .collect();
    assert_eq!(found, expected);
    let codes: BTreeSet<&str> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes.len(), found.len(), "codes are distinct");
}
