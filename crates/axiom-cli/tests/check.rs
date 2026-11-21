//! End-to-end tests of `axiom check` through the Lean checker. Each test is
//! skipped, with a message, when `lake` is not on PATH.

mod common;

use std::ffi::OsStr;
use std::path::Path;
use std::process::Output;

use common::{TestError, axiom, ensure, lake_available, repo_root, scratch};

fn check(cert: &Path) -> Result<Output, TestError> {
    axiom(&[OsStr::new("check"), cert.as_os_str()])
}

#[test]
fn accepts_golden_certificate() -> Result<(), TestError> {
    if !lake_available("accepts_golden_certificate") {
        return Ok(());
    }
    let output = check(&repo_root().join("fixtures/certificates/links-mass.json"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(output.status.code() == Some(0), "expected exit status 0")?;
    ensure(
        stdout.contains("claim 0 Mass row 0: accepted")
            && stdout.contains("claim 1 Mass row 1: accepted"),
        "missing accepted verdicts",
    )?;
    ensure(
        stdout.contains("2 of 2 claims accepted in"),
        "missing summary with timing",
    )
}

#[test]
fn rejects_row_that_does_not_sum_to_one() -> Result<(), TestError> {
    if !lake_available("rejects_row_that_does_not_sum_to_one") {
        return Ok(());
    }
    let output = check(&repo_root().join("fixtures/certificates/reject/does-not-sum-to-one.json"))?;
    ensure(output.status.code() == Some(1), "expected exit status 1")?;
    ensure(
        String::from_utf8_lossy(&output.stdout).contains(": rejected"),
        "missing rejected verdict",
    )
}

#[test]
fn rejects_mutant_entry() -> Result<(), TestError> {
    if !lake_available("rejects_mutant_entry") {
        return Ok(());
    }
    let golden =
        std::fs::read_to_string(repo_root().join("fixtures/certificates/links-mass.json"))?;
    let original = "\"num\": \"9\",\n          \"den\": \"10\"";
    ensure(
        golden.contains(original),
        "golden file has an unexpected layout",
    )?;
    let mutant = golden.replacen(
        original,
        "\"num\": \"999\",\n          \"den\": \"1000\"",
        1,
    );
    let dir = scratch("mutant")?;
    let path = dir.join("mutant.json");
    std::fs::write(&path, mutant)?;
    let output = check(&path)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(output.status.code() == Some(1), "expected exit status 1")?;
    ensure(
        stdout.contains("claim 0 Mass row 0: rejected")
            && stdout.contains("claim 1 Mass row 1: accepted"),
        "expected row 0 rejected and row 1 accepted",
    )
}

#[test]
fn malformed_certificate_exits_with_status_two() -> Result<(), TestError> {
    if !lake_available("malformed_certificate_exits_with_status_two") {
        return Ok(());
    }
    let output = check(&repo_root().join("fixtures/certificates/reject/duplicate-key.json"))?;
    ensure(output.status.code() == Some(2), "expected exit status 2")?;
    ensure(
        String::from_utf8_lossy(&output.stderr).contains("Lean checker:"),
        "missing checker error",
    )
}

#[test]
fn missing_certificate_exits_with_status_two() -> Result<(), TestError> {
    let output = check(&repo_root().join("fixtures/certificates/no-such-file.json"))?;
    ensure(output.status.code() == Some(2), "expected exit status 2")
}
