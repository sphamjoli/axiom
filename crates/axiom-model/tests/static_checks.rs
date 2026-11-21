//! Each static check of the language rejects a faulty model with the line
//! and column of the fault, and no message quotes the file.

mod common;

use axiom_model::{MAX_INPUT_BYTES, ModelError, Position};
use common::{TestError, rejection};

fn at(line: usize, column: usize) -> Position {
    Position { line, column }
}

fn rejects(source: &str, code: &str, line: usize, column: usize) -> Result<(), TestError> {
    let error = rejection(source)?;
    assert_eq!((error.code(), error.position()), (code, at(line, column)));
    assert!(
        error
            .to_string()
            .starts_with(&format!("line {line}, column {column}: "))
    );
    Ok(())
}

#[test]
fn rejects_undeclared_and_misused_names() -> Result<(), TestError> {
    rejects(
        "chain c {\n  states s\n  from t: s 1\n}\n",
        "undeclared-name",
        3,
        8,
    )?;
    rejects(
        "chain c {\n  states s\n  from s: s q\n}\n",
        "undeclared-name",
        3,
        13,
    )?;
    rejects(
        "param p in [0, 1]\nchain c {\n  states s\n  from s: p 1\n}\n",
        "parameter-as-state",
        4,
        11,
    )?;
    rejects(
        "chain c {\n  states s\n  from s: s s\n}\n",
        "state-as-parameter",
        3,
        13,
    )
}

#[test]
fn rejects_repeated_and_keyword_names() -> Result<(), TestError> {
    let error = rejection("chain c {\n  states s, t, s\n  from s: s 1\n}\n")?;
    assert_eq!(
        error,
        ModelError::RepeatedName {
            at: at(2, 16),
            first: at(2, 10),
        }
    );
    rejects("chain c {\n  states in\n}\n", "keyword-as-name", 2, 10)?;
    rejects("retry success { success 1 }", "keyword-as-name", 1, 7)?;
    rejects(
        "param retry in [0, 1]\nretry r { success retry }",
        "keyword-as-name",
        1,
        7,
    )
}

#[test]
fn rejects_rows_before_states() -> Result<(), TestError> {
    rejects(
        "chain c {\n  from s: s 1\n}\n",
        "state-before-declaration",
        2,
        3,
    )
}

#[test]
fn rejects_chains_without_states_or_with_too_many() -> Result<(), TestError> {
    rejects("chain c {\n  states\n}\n", "no-states", 2, 3)?;
    let names: Vec<String> = (0..17).map(|index| format!("s{index}")).collect();
    let source = format!("chain c {{ states {} from s0: s0 1 }}", names.join(", "));
    let error = rejection(&source)?;
    assert_eq!(error.code(), "too-many-states");
    let sixteen: Vec<String> = (0..16).map(|index| format!("s{index}")).collect();
    let rows = sixteen
        .iter()
        .map(|name| format!("from {name}: {name} 1\n"))
        .collect::<Vec<_>>()
        .concat();
    let source = format!("chain c {{\nstates {}\n{rows}}}", sixteen.join(", "));
    assert_eq!(common::chain(&source)?.states.len(), 16);
    Ok(())
}

#[test]
fn rejects_missing_repeated_and_doubled_rows() -> Result<(), TestError> {
    rejects(
        "chain c {\n  states s, t\n  from s: s 1\n}\n",
        "missing-row",
        2,
        13,
    )?;
    let error = rejection("chain c {\n  states s\n  from s: s 1\n  from s: s 1\n}\n")?;
    assert_eq!(
        error,
        ModelError::RepeatedRow {
            at: at(4, 8),
            first: at(3, 8),
        }
    );
    let error = rejection("chain c {\n  states s\n  from s: s 1/2, s 1/2\n}\n")?;
    assert_eq!(
        error,
        ModelError::RepeatedTarget {
            at: at(3, 18),
            first: at(3, 11),
        }
    );
    Ok(())
}

#[test]
fn rejects_rows_that_do_not_sum_to_one() -> Result<(), TestError> {
    rejects(
        "chain c {\n  states s, t\n  from s: s 1/2, t 1/3\n  from t: t 1\n}\n",
        "row-not-normalised",
        3,
        3,
    )?;
    rejects(
        "param a in [0, 1/2]\nchain c {\n  states s, t\n  from s: s 1 - a, t a + a\n  from t: t 1\n}\n",
        "row-not-normalised",
        4,
        3,
    )?;
    common::chain("chain c {\n  states s, t\n  from s: s 2/4, t 3/6\n  from t: t 1\n}\n")?;
    Ok(())
}

#[test]
fn rejects_weights_out_of_range_for_some_parameter_value() -> Result<(), TestError> {
    rejects(
        "param a in [0, 1]\nchain c {\n  states s, t\n  from s: s 3/2 - a, t a - 1/2\n  from t: t 1\n}\n",
        "weight-above-one",
        4,
        13,
    )?;
    rejects(
        "param a in [1/4, 3/4]\nretry r { success 1/2 - a }",
        "weight-below-zero",
        2,
        19,
    )?;
    rejects("retry r { success 3/2 }", "weight-above-one", 1, 19)?;
    common::retry("param a in ]0, 1[\nretry r { success 1 - a }")?;
    common::retry("param a in [1/4, 3/4]\nretry r { success 1/4 + 1/2*a - 1/8 }")?;
    Ok(())
}

#[test]
fn rejects_empty_intervals_and_unused_parameters() -> Result<(), TestError> {
    rejects(
        "param a in ]1, 1[\nretry r { success a }",
        "empty-interval",
        1,
        12,
    )?;
    rejects(
        "param a in [1, 1[\nretry r { success a }",
        "empty-interval",
        1,
        12,
    )?;
    rejects(
        "param a in [1/2, 1/3]\nretry r { success a }",
        "empty-interval",
        1,
        12,
    )?;
    common::retry("param a in [1, 1]\nretry r { success a }")?;
    rejects(
        "param a in [0, 1]\nparam b in [0, 1]\nretry r { success b }",
        "unused-parameter",
        1,
        7,
    )
}

#[test]
fn rejects_deferred_constructs() -> Result<(), TestError> {
    rejects(
        "param a in [0, 1]\nparam b in [0, 1]\nretry r { success a*b }",
        "nonlinear-weight",
        3,
        20,
    )?;
    rejects(
        "param a in [0, 1]\nretry r { success 1/2*a*a }",
        "nonlinear-weight",
        2,
        24,
    )?;
    rejects(
        "param a in [0, 1]\nretry r { success a^2 }",
        "unexpected-character",
        2,
        20,
    )?;
    rejects(
        "retry r { success 1 }\nretry s { success 1 }",
        "trailing-input",
        2,
        1,
    )?;
    rejects(
        "retry r { success 1 }\nparam a in [0, 1]",
        "trailing-input",
        2,
        1,
    )
}

#[test]
fn enforces_size_limits() -> Result<(), TestError> {
    let padding = "#".repeat(MAX_INPUT_BYTES);
    let error = rejection(&format!("retry r {{ success 1 }}\n{padding}"))?;
    assert_eq!(error.code(), "input-too-large");
    assert_eq!(error.position(), Position::START);

    let digits = "9".repeat(65);
    rejects(
        &format!("retry r {{ success {digits}/{digits} }}"),
        "number-too-long",
        1,
        19,
    )?;
    let digits = "9".repeat(64);
    common::retry(&format!("retry r {{ success {digits}/{digits} }}"))?;

    let params = (0..9)
        .map(|index| format!("param p{index} in [0, 1]\n"))
        .collect::<Vec<_>>()
        .concat();
    rejects(
        &format!("{params}retry r {{ success p0 }}"),
        "too-many-parameters",
        9,
        1,
    )
}

#[test]
fn rejects_malformed_syntax() -> Result<(), TestError> {
    rejects("retry r { success -1/2 }", "unexpected-token", 1, 19)?;
    rejects("retry r { success 1/0 }", "zero-denominator", 1, 21)?;
    rejects("retry r { success 1", "unexpected-end", 1, 20)?;
    rejects("", "unexpected-end", 1, 1)?;
    rejects("retry r { success \u{e9} }", "unexpected-character", 1, 19)?;
    rejects(
        "chain c {\n  states s\n  from s: s 1/2 s 1/2\n}\n",
        "unexpected-token",
        3,
        17,
    )
}

#[test]
fn messages_never_quote_the_input() -> Result<(), TestError> {
    let secret = "Zq7Secret";
    let sources = [
        format!("chain c {{ states s from {secret}: s 1 }}"),
        format!("chain {secret} {{ states {secret}, {secret} from s: s 1 }}"),
        format!("param {secret} in [0, 1]\nretry r {{ success 1 }}"),
        format!("param {secret} in [0, 1]\nretry r {{ success {secret}*{secret} }}"),
        format!("retry r {{ success 1 }} {secret}"),
        format!("retry {secret} {{ success {secret}9 }}"),
        format!("chain c {{ states s from s: s 1, {secret} 0 }}"),
        format!("retry r {{ success 1/2 }} # {secret}\n\u{e9}{secret}"),
    ];
    for source in &sources {
        let message = rejection(source)?.to_string();
        assert!(
            !message.contains(secret),
            "message quotes input: {message:?}"
        );
    }
    Ok(())
}
