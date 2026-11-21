//! Every model error has a stable, distinct code, and its message starts
//! with the fault's position. The messages themselves are pinned by the
//! static checks, and that no message quotes the file by
//! `messages_never_quote_the_input` there.

use std::collections::BTreeSet;

use axiom_model::{Expected, ModelError, Position};

const AT: Position = Position { line: 3, column: 7 };

fn every_error() -> Vec<ModelError> {
    let first = Position::START;
    vec![
        ModelError::InputTooLarge {
            at: AT,
            bytes: 9,
            limit: 8,
        },
        ModelError::UnexpectedCharacter { at: AT },
        ModelError::NumberTooLong {
            at: AT,
            digits: 9,
            limit: 8,
        },
        ModelError::ZeroDenominator { at: AT },
        ModelError::UnexpectedToken {
            at: AT,
            expected: Expected::Name,
        },
        ModelError::UnexpectedEnd {
            at: AT,
            expected: Expected::CloseBrace,
        },
        ModelError::KeywordAsName { at: AT },
        ModelError::UndeclaredName { at: AT },
        ModelError::RepeatedName { at: AT, first },
        ModelError::ParameterAsState { at: AT },
        ModelError::StateAsParameter { at: AT },
        ModelError::StateBeforeDeclaration { at: AT },
        ModelError::NoStates { at: AT },
        ModelError::TooManyStates { at: AT, limit: 8 },
        ModelError::TooManyParameters { at: AT, limit: 8 },
        ModelError::MissingRow { at: AT },
        ModelError::RepeatedRow { at: AT, first },
        ModelError::RepeatedTarget { at: AT, first },
        ModelError::RowNotNormalised { at: AT },
        ModelError::WeightBelowZero { at: AT },
        ModelError::WeightAboveOne { at: AT },
        ModelError::EmptyInterval { at: AT },
        ModelError::UnusedParameter { at: AT },
        ModelError::NonlinearWeight { at: AT },
        ModelError::TrailingInput { at: AT },
    ]
}

#[test]
fn every_error_has_a_stable_distinct_code_and_starts_with_its_position() {
    let errors = every_error();
    let codes: Vec<&str> = errors.iter().map(ModelError::code).collect();
    assert_eq!(
        codes,
        [
            "input-too-large",
            "unexpected-character",
            "number-too-long",
            "zero-denominator",
            "unexpected-token",
            "unexpected-end",
            "keyword-as-name",
            "undeclared-name",
            "repeated-name",
            "parameter-as-state",
            "state-as-parameter",
            "state-before-declaration",
            "no-states",
            "too-many-states",
            "too-many-parameters",
            "missing-row",
            "repeated-row",
            "repeated-target",
            "row-not-normalised",
            "weight-below-zero",
            "weight-above-one",
            "empty-interval",
            "unused-parameter",
            "nonlinear-weight",
            "trailing-input",
        ]
    );
    assert_eq!(
        codes.iter().collect::<BTreeSet<_>>().len(),
        codes.len(),
        "codes are distinct"
    );
    for error in &errors {
        assert_eq!(error.position(), AT, "{}", error.code());
        assert!(
            error.to_string().starts_with("line 3, column 7: "),
            "{}",
            error.code()
        );
    }
}
