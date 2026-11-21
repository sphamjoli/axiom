//! Every simulation error has a stable, distinct code and a fixed message
//! built from indices and counts only. The table pins both.

use std::collections::BTreeSet;

use axiom_sim::SimError;

#[test]
fn every_error_has_a_stable_distinct_code_and_fixed_message() {
    let errors = [
        SimError::Parametric,
        SimError::StateOutOfRange(4),
        SimError::InvalidEpsilon,
        SimError::InvalidDelta,
        SimError::SamplesUnreachable,
        SimError::TooFewSamples { actual: 9 },
        SimError::HitsExceedSamples,
    ];
    let found: Vec<(&str, String)> = errors
        .iter()
        .map(|error| (error.code(), error.to_string()))
        .collect();
    let expected = [
        (
            "parametric-model",
            "the model has parameters, so it has no exact probabilities to sample",
        ),
        ("state-out-of-range", "state 4 is outside the chain"),
        (
            "invalid-epsilon",
            "epsilon must lie strictly between zero and one",
        ),
        (
            "invalid-delta",
            "delta must lie strictly between zero and one",
        ),
        (
            "samples-unreachable",
            "no sample count up to the search limit meets the confidence",
        ),
        (
            "too-few-samples",
            "9 samples are fewer than the confidence requires",
        ),
        ("hits-exceed-samples", "more hits than samples"),
    ];
    let expected: Vec<(&str, String)> = expected
        .iter()
        .map(|(code, message)| (*code, (*message).to_owned()))
        .collect();
    assert_eq!(found, expected);
    let codes: BTreeSet<&str> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes.len(), found.len(), "codes are distinct");
}
