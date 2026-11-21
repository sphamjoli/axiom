//! Tests of `analyse` on the shared model fixtures.

use std::path::PathBuf;

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
enum TestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
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

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/models")
        .join(name)
}

fn analyse_fixture(name: &str) -> Result<(String, Value), TestError> {
    let source = std::fs::read_to_string(fixture(name))?;
    let value = serde_json::from_str(&axiom_wasm::analyse(&source))?;
    Ok((source, value))
}

fn fraction(value: &Value, num: &str, den: &str) -> bool {
    value["num"] == num && value["den"] == den
}

#[test]
fn links_matches_expected_values() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("links.axm")?;
    ensure(result["ok"] == true, "links was rejected")?;
    let analysis = &result["analysis"];
    let chain = &analysis["chain"];
    ensure(
        chain["states"] == serde_json::json!(["up", "down"]),
        "states",
    )?;
    ensure(
        chain["transitions"].as_array().map(Vec::len) == Some(4),
        "four transitions",
    )?;
    ensure(
        fraction(&chain["transitions"][1]["p"], "1", "10"),
        "up to down",
    )?;
    let convergence = &analysis["convergence"];
    ensure(convergence["state"] == "down", "tracked state")?;
    ensure(fraction(&convergence["a"], "3", "5"), "lambda")?;
    ensure(fraction(&convergence["b"], "1", "10"), "offset")?;
    ensure(
        fraction(&convergence["fixedPoint"], "1", "4"),
        "steady state",
    )?;
    ensure(fraction(&convergence["epsilon"], "1", "1000"), "tolerance")?;
    ensure(convergence["horizon"] == 13, "horizon")?;
    let series = &convergence["series"];
    ensure(fraction(&series[0]["value"], "1", "1"), "d0")?;
    ensure(fraction(&series[2]["value"], "13", "25"), "d2")?;
    ensure(fraction(&series[3]["value"], "103", "250"), "d3")?;
    ensure(series[3]["approx"].as_f64().is_some(), "approx")?;
    ensure(analysis["retry"].is_null(), "no retry")
}

#[test]
fn links_certificate_is_the_golden_one_and_unchecked() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("links.axm")?;
    let certificate = &result["analysis"]["certificate"];
    let golden = std::fs::read_to_string(fixture("../certificates/links-fixed-point.json"))?;
    ensure(certificate["json"] == golden.as_str(), "certificate json")?;
    ensure(certificate["version"] == 2, "version")?;
    let claims = certificate["claims"]
        .as_array()
        .ok_or_else(|| TestError::Failed(String::from("claims")))?;
    ensure(claims.len() == 4, "four claims")?;
    ensure(
        claims.iter().all(|claim| claim["status"] == "unchecked"),
        "every claim unchecked",
    )
}

#[test]
fn retry_reports_success_and_attempts() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("retry.axm")?;
    ensure(result["ok"] == true, "retry was rejected")?;
    let retry = &result["analysis"]["retry"];
    ensure(fraction(&retry["success"], "3", "10"), "success")?;
    ensure(fraction(&retry["successProbability"], "1", "1"), "eventual")?;
    ensure(fraction(&retry["expectedAttempts"], "10", "3"), "attempts")?;
    ensure(result["analysis"]["chain"].is_null(), "no chain")
}

#[test]
fn periodic_gets_mass_claims_only() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("periodic.axm")?;
    ensure(result["ok"] == true, "periodic was rejected")?;
    let analysis = &result["analysis"];
    ensure(analysis["convergence"].is_null(), "no convergence")?;
    ensure(analysis["certificate"]["version"] == 1, "version 1")?;
    ensure(
        analysis["certificate"]["claims"].as_array().map(Vec::len) == Some(2),
        "two mass claims",
    )
}

#[test]
fn reject_reports_position_without_echo() -> Result<(), TestError> {
    let (source, result) = analyse_fixture("reject/missing-row.axm")?;
    ensure(result["ok"] == false, "missing row was accepted")?;
    let diagnostic = &result["diagnostics"][0];
    ensure(diagnostic["code"] == "missing-row", "code")?;
    ensure(diagnostic["line"] == 2, "line")?;
    ensure(diagnostic["column"] == 14, "column")?;
    let message = diagnostic["message"].as_str().unwrap_or_default();
    ensure(!message.is_empty(), "message")?;
    ensure(
        !source
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .any(|word| message.contains(word)),
        "message quotes the source",
    )
}
