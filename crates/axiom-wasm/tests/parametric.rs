//! Tests of the parametric wiring: the view and certificate `analyse`
//! returns for links-param, the point `analyse_at` analyses and the points
//! and models it refuses. The claim and the instance's values come from
//! `axiom-cert` and `axiom-core` and are tested there.

use std::path::PathBuf;

use axiom_cert::{Certificate, CertificateError};
use axiom_model::{CheckedModel, ModelError};
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
enum TestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Certificate(#[from] CertificateError),
    #[error("{0}")]
    Failed(String),
}

fn ensure(condition: bool, message: &str) -> Result<(), TestError> {
    if condition {
        Ok(())
    } else {
        Err(TestError::Failed(message.to_owned()))
    }
}

fn links_param() -> Result<String, TestError> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/models/links-param.axm");
    Ok(std::fs::read_to_string(path)?)
}

fn fraction(num: &str, den: &str) -> Value {
    json!({ "num": num, "den": den })
}

/// `{lo, loClosed, hi, hiClosed}` for an interval open at both ends.
fn open_range(lo: Value, hi: Value) -> Value {
    json!({ "lo": lo, "loClosed": false, "hi": hi, "hiClosed": false })
}

fn at(source: &str, a: (&str, &str), b: (&str, &str)) -> Result<Value, TestError> {
    let point = json!({ "a": fraction(a.0, a.1), "b": fraction(b.0, b.1) }).to_string();
    Ok(serde_json::from_str(&axiom_wasm::analyse_at(
        source, &point,
    ))?)
}

fn error_code(result: &Value) -> Option<&str> {
    result["error"]["code"].as_str()
}

/// links-param gets the parametric view with exact ranges, the instance at
/// the midpoint (1/2, 1/2) and exactly the certificate `axiom-cert` writes.
#[test]
fn links_param_returns_the_parametric_view_and_certificate() -> Result<(), TestError> {
    let source = links_param()?;
    let result: Value = serde_json::from_str(&axiom_wasm::analyse(&source))?;
    let analysis = &result["analysis"];
    let view = &analysis["parametric"];
    ensure(analysis["notice"].is_null(), "no notice")?;
    ensure(
        view["a"]["name"] == "a" && view["b"]["name"] == "b",
        "names",
    )?;
    let unit = open_range(fraction("0", "1"), fraction("1", "1"));
    ensure(
        view["a"]["range"] == unit && view["b"]["range"] == unit,
        "intervals",
    )?;
    ensure(
        view["steadyState"] == "a/(a+b)" && view["factor"] == "1 - a - b",
        "forms",
    )?;
    ensure(
        view["sum"] == open_range(fraction("0", "1"), fraction("2", "1")),
        "a + b",
    )?;
    ensure(
        view["lambda"] == open_range(fraction("-1", "1"), fraction("1", "1")),
        "lambda",
    )?;
    ensure(
        view["point"]["a"] == fraction("1", "2") && view["point"]["b"] == fraction("1", "2"),
        "midpoint",
    )?;
    ensure(analysis["chain"].is_object(), "instance chain")?;

    let CheckedModel::Chain(chain) = axiom_model::parse(&source)? else {
        return Err(TestError::Failed(String::from(
            "links-param is not a chain",
        )));
    };
    let (a, b) = chain
        .two_state_exits()
        .ok_or_else(|| TestError::Failed(String::from("no exits")))?;
    let interval = |p: &axiom_model::Parameter| {
        axiom_cert::Interval::from_bounds(
            &p.lower.value,
            p.lower.closed,
            &p.upper.value,
            p.upper.closed,
        )
    };
    let wanted = Certificate::parametric(interval(a)?, interval(b)?)?.to_json()?;
    let certificate = &analysis["certificate"];
    ensure(certificate["json"] == wanted.as_str(), "certificate json")?;
    ensure(
        certificate["claims"][0]["id"] == "parametric-contraction",
        "claim",
    )?;
    ensure(
        certificate["claims"][0]["status"] == "unchecked",
        "unchecked",
    )
}

/// The instance at a = 1/10, b = 3/10 is the links chain, and its point
/// source simulates as a fixed model.
#[test]
fn the_point_a_tenth_b_three_tenths_is_the_links_chain() -> Result<(), TestError> {
    let result = at(&links_param()?, ("1", "10"), ("3", "10"))?;
    let analysis = &result["analysis"];
    ensure(analysis["convergence"]["horizon"] == 13, "horizon")?;
    ensure(
        analysis["convergence"]["fixedPoint"] == fraction("1", "4"),
        "steady state",
    )?;
    let instance = analysis["parametric"]["point"]["source"]
        .as_str()
        .ok_or_else(|| TestError::Failed(String::from("instance source")))?;
    ensure(
        instance.contains("from up: up 9/10, down 1/10"),
        "instance text",
    )?;
    let CheckedModel::Chain(fixed) = axiom_model::parse(instance)? else {
        return Err(TestError::Failed(String::from("instance is not a chain")));
    };
    ensure(fixed.matrix.is_some(), "instance has no parameters")
}

/// Open ends are excluded exactly, values past an end are refused, and a
/// malformed or oversized request is refused before the model is read.
#[test]
fn points_outside_the_box_or_malformed_are_refused() -> Result<(), TestError> {
    let source = links_param()?;
    for (a, b) in [
        (("0", "1"), ("1", "2")),
        (("1", "2"), ("1", "1")),
        (("3", "2"), ("1", "2")),
    ] {
        ensure(
            error_code(&at(&source, a, b)?) == Some("point-out-of-range"),
            "out of range",
        )?;
    }
    let invalid = [
        (("1", "0"), ("1", "2")),
        (("-1", "2"), ("1", "2")),
        (("1234567890", "2"), ("1", "2")),
    ];
    for (a, b) in invalid {
        ensure(
            error_code(&at(&source, a, b)?) == Some("invalid-point-fraction"),
            "invalid fraction",
        )?;
    }
    let refused = |point: &str| -> Result<Option<String>, TestError> {
        let value: Value = serde_json::from_str(&axiom_wasm::analyse_at(&source, point))?;
        Ok(error_code(&value).map(str::to_owned))
    };
    ensure(
        refused(r#"{"a": 1}"#)?.as_deref() == Some("malformed-point"),
        "malformed",
    )?;
    let long = format!(
        r#"{{"a": {{"num": "1", "den": "2"}}, "b": {{"num": "1", "den": "2"}}, "pad": "{}"}}"#,
        " ".repeat(axiom_wasm::MAX_POINT_BYTES)
    );
    ensure(
        refused(&long)?.as_deref() == Some("point-too-large"),
        "too large",
    )?;
    let fixed = at(
        "chain c {\n  states s\n  from s: s 1\n}\n",
        ("1", "2"),
        ("1", "2"),
    )?;
    ensure(
        error_code(&fixed) == Some("not-parametric"),
        "not parametric",
    )
}

/// A parametric chain of another shape keeps a notice that names what the
/// browser supports and why this model is not it.
#[test]
fn other_parametric_chains_get_a_precise_notice() -> Result<(), TestError> {
    let source = "param a in ]0, 1[\nparam b in ]0, 1[\nchain c {\n  states s, t, u\n  from s: s 1 - a, t a\n  from t: s b, t 1 - b\n  from u: u 1\n}\n";
    let result: Value = serde_json::from_str(&axiom_wasm::analyse(source))?;
    ensure(
        result["analysis"]["notice"]
            == "The browser analyses parameters for two-state chains whose exit probabilities are the parameters, such as links-param. This model has 3 states, so replace its parameters with values in the text to analyse one instance.",
        "notice text",
    )?;
    ensure(result["analysis"]["parametric"].is_null(), "no view")
}
