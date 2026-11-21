//! Tests of `simulate`. They cover the wiring only: the JSON shape, the
//! seed, the caps and the error mapping. How samples are drawn and when an
//! estimate holds is owned and tested by `axiom-sim`.

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

fn model(name: &str) -> Result<String, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/models")
        .join(name);
    Ok(std::fs::read_to_string(path)?)
}

fn request(seed: &str, samples: u64, steps: u64, max_attempts: u64) -> String {
    format!(
        r#"{{"seed":"{seed}","samples":{samples},"steps":{steps},"maxAttempts":{max_attempts}}}"#
    )
}

fn run(source: &str, request: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&axiom_wasm::simulate(
        source, request,
    ))?)
}

/// Runs the links chain and returns the error code, or `"ok"`.
fn links_code(request: &str) -> Result<String, TestError> {
    let result = run(&model("links.axm")?, request)?;
    if result["ok"] == true {
        return Ok(String::from("ok"));
    }
    Ok(result["error"]["code"].as_str().unwrap_or("").to_owned())
}

#[test]
fn the_same_seed_gives_identical_json_and_another_seed_does_not() -> Result<(), TestError> {
    let source = model("links.axm")?;
    let first = axiom_wasm::simulate(&source, &request("42", 500, 20, 10));
    let again = axiom_wasm::simulate(&source, &request("42", 500, 20, 10));
    let other = axiom_wasm::simulate(&source, &request("43", 500, 20, 10));
    ensure(first.starts_with(r#"{"ok":true"#), "links ran")?;
    ensure(first == again, "same seed, same bytes")?;
    ensure(first != other, "different seed, different bytes")
}

/// The full default sample count meets the Hoeffding bound for 1/100 and
/// 1/1000, so links gets an estimate, always marked estimated.
#[test]
fn links_returns_an_estimated_estimate() -> Result<(), TestError> {
    let result = run(&model("links.axm")?, &request("7", 38_005, 100, 10))?;
    let simulation = &result["simulation"];
    ensure(simulation["requiredSamples"] == 38_005, "required samples")?;
    let estimate = &simulation["estimate"];
    ensure(estimate["certainty"] == "estimated", "certainty")?;
    ensure(estimate["samples"] == 38_005, "samples")?;
    ensure(
        estimate["epsilon"] == serde_json::json!({"num": "1", "den": "100"}),
        "default epsilon",
    )?;
    ensure(
        estimate["delta"] == serde_json::json!({"num": "1", "den": "1000"}),
        "default delta",
    )?;
    let chain = &simulation["chain"];
    ensure(
        chain["steadyState"] == serde_json::json!({"num": "1", "den": "4"}),
        "steady state",
    )?;
    ensure(
        chain["paths"].as_array().map(Vec::len) == Some(axiom_wasm::PATHS),
        "paths",
    )?;
    ensure(
        chain["running"].as_array().map(Vec::len) == Some(200),
        "running points",
    )
}

/// One sample fewer than the bound needs still runs, without an estimate.
#[test]
fn too_few_samples_run_without_an_estimate() -> Result<(), TestError> {
    let result = run(&model("links.axm")?, &request("7", 38_004, 10, 10))?;
    ensure(result["ok"] == true, "ran")?;
    ensure(result["simulation"]["estimate"].is_null(), "no estimate")
}

/// Paths are cut to 200 steps even when the samples run longer.
#[test]
fn sample_paths_are_capped_at_200_steps() -> Result<(), TestError> {
    let result = run(&model("links.axm")?, &request("7", 1, 1000, 10))?;
    let lengths: Vec<Option<usize>> = result["simulation"]["chain"]["paths"]
        .as_array()
        .map(|paths| paths.iter().map(|p| p.as_array().map(Vec::len)).collect())
        .unwrap_or_default();
    ensure(
        lengths == vec![Some(201); axiom_wasm::PATHS],
        "201 states each",
    )
}

/// Every run lands in exactly one place: a bucket, past the buckets, or
/// failed. With 100 attempts allowed there are 40 buckets, and at 1/100 per
/// attempt many runs need more than 40 or fail.
#[test]
fn retry_returns_a_histogram() -> Result<(), TestError> {
    let slow = "retry slow {\n  success 1/100\n}\n";
    let result = run(slow, &request("7", 1000, 1, 100))?;
    let retry = &result["simulation"]["retry"];
    let histogram = retry["histogram"]
        .as_array()
        .ok_or_else(|| TestError::Failed(String::from("no histogram")))?;
    ensure(histogram.len() == 40, "40 buckets")?;
    ensure(
        histogram.first().map(|b| &b["attempts"]) == Some(&Value::from(1)),
        "from 1",
    )?;
    let counted: u64 = histogram.iter().filter_map(|b| b["count"].as_u64()).sum();
    let beyond = retry["beyond"].as_u64().unwrap_or(u64::MAX);
    let failures = retry["failures"].as_u64().unwrap_or(u64::MAX);
    ensure(
        beyond > 0 && failures > 0,
        "runs past the buckets and failures",
    )?;
    ensure(
        counted + beyond + failures == 1000,
        "every run counted once",
    )?;
    ensure(
        retry["expectedAttempts"] == serde_json::json!({"num": "100", "den": "1"}),
        "expected attempts",
    )
}

#[test]
fn samples_cap_is_200000() -> Result<(), TestError> {
    ensure(
        links_code(&request("1", 200_000, 1, 1))? == "ok",
        "at the cap",
    )?;
    ensure(
        links_code(&request("1", 200_001, 1, 1))? == "samples-out-of-range",
        "past the cap",
    )?;
    ensure(
        links_code(&request("1", 0, 1, 1))? == "samples-out-of-range",
        "zero",
    )
}

#[test]
fn steps_cap_is_1000() -> Result<(), TestError> {
    ensure(links_code(&request("1", 1, 1000, 1))? == "ok", "at the cap")?;
    ensure(
        links_code(&request("1", 1, 1001, 1))? == "steps-out-of-range",
        "past the cap",
    )?;
    ensure(
        links_code(&request("1", 1, 0, 1))? == "steps-out-of-range",
        "zero",
    )
}

#[test]
fn attempts_cap_is_1000() -> Result<(), TestError> {
    ensure(links_code(&request("1", 1, 1, 1000))? == "ok", "at the cap")?;
    ensure(
        links_code(&request("1", 1, 1, 1001))? == "attempts-out-of-range",
        "past the cap",
    )?;
    ensure(
        links_code(&request("1", 1, 1, 0))? == "attempts-out-of-range",
        "zero",
    )
}

/// The request is padded with trailing spaces to the cap and one past it.
#[test]
fn request_cap_is_1024_bytes() -> Result<(), TestError> {
    let base = request("1", 1, 1, 1);
    let at = format!("{base:<width$}", width = axiom_wasm::MAX_REQUEST_BYTES);
    let past = format!("{at} ");
    ensure(links_code(&at)? == "ok", "at the cap")?;
    ensure(links_code(&past)? == "request-too-large", "past the cap")
}

/// The model is padded with a comment to the cap and one past it.
#[test]
fn source_cap_is_65536_bytes() -> Result<(), TestError> {
    let base = model("links.axm")?;
    let room = axiom_wasm::MAX_SOURCE_BYTES - base.len() - 2;
    let at = format!("{base}#{}\n", "x".repeat(room));
    let past = format!("{at} ");
    let ask = request("1", 1, 1, 1);
    ensure(run(&at, &ask)?["ok"] == true, "at the cap")?;
    ensure(
        run(&past, &ask)?["error"]["code"] == "source-too-large",
        "past the cap",
    )
}

/// Each request below is refused with its code and a message that does not
/// repeat what was sent.
#[test]
fn bad_requests_get_their_code_and_a_fixed_message() -> Result<(), TestError> {
    let cases = [
        ("not json", "malformed-request"),
        (
            r#"{"seed":"1","samples":1,"steps":1,"maxAttempts":1,"extra":true}"#,
            "malformed-request",
        ),
        (
            r#"{"seed":"1","samples":-1,"steps":1,"maxAttempts":1}"#,
            "malformed-request",
        ),
        (&request("18446744073709551616", 1, 1, 1), "invalid-seed"),
        (&request("-5", 1, 1, 1), "invalid-seed"),
        (&request("", 1, 1, 1), "invalid-seed"),
        (&request("1e3", 1, 1, 1), "invalid-seed"),
        (&request("+5", 1, 1, 1), "invalid-seed"),
        (
            r#"{"seed":"1","samples":1,"steps":1,"maxAttempts":1,"epsilon":{"num":"1","den":"0"}}"#,
            "invalid-fraction",
        ),
        (
            r#"{"seed":"1","samples":1,"steps":1,"maxAttempts":1,"delta":{"num":"1","den":"1000000000"}}"#,
            "invalid-fraction",
        ),
    ];
    let source = model("links.axm")?;
    for (ask, code) in cases {
        let result = run(&source, ask)?;
        let message = result["error"]["message"].as_str().unwrap_or("");
        ensure(result["error"]["code"] == code, code)?;
        ensure(
            !message.contains("1e3") && !message.contains("extra"),
            "no echo",
        )?;
    }
    Ok(())
}

#[test]
fn the_largest_seed_is_accepted() -> Result<(), TestError> {
    let result = run(
        &model("links.axm")?,
        &request("18446744073709551615", 1, 1, 1),
    )?;
    ensure(
        result["simulation"]["seed"] == "18446744073709551615",
        "seed kept as a string",
    )
}

#[test]
fn model_errors_keep_their_code_and_position() -> Result<(), TestError> {
    let result = run("chain broken {\n  states a\n", &request("1", 1, 1, 1))?;
    let error = &result["error"];
    ensure(result["ok"] == false, "refused")?;
    ensure(
        error["line"].is_u64() && error["column"].is_u64(),
        "position",
    )?;
    ensure(error["code"].is_string(), "code")
}

#[test]
fn sim_errors_keep_their_code() -> Result<(), TestError> {
    let parametric = run(&model("links-param.axm")?, &request("1", 1, 1, 1))?;
    ensure(
        parametric["error"]["code"] == "parametric-model",
        "parametric",
    )?;
    let loose =
        r#"{"seed":"1","samples":1,"steps":1,"maxAttempts":1,"epsilon":{"num":"1","den":"1"}}"#;
    ensure(links_code(loose)? == "invalid-epsilon", "epsilon of one")?;
    let tight = r#"{"seed":"1","samples":1,"steps":1,"maxAttempts":1,"epsilon":{"num":"1","den":"999999999"},"delta":{"num":"1","den":"999999999"}}"#;
    ensure(links_code(tight)? == "samples-unreachable", "unreachable")
}
