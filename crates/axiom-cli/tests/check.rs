//! End-to-end tests of `axiom check`: exit codes and output through the real
//! Lean checker, and the time limit through a stand-in for `lake`. Which
//! certificates the checker accepts is tested by the Lean checker's own tests
//! against the shared fixtures, and output parsing in `output.rs`. The tests
//! that need Lean, named `lean_*`, are skipped when `lake` is not on PATH,
//! unless `CI` is set, when they fail instead; CI runs them in the Lean job.

mod common;

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

use common::{TestError, axiom, ensure, lake_available, repo_root, scratch};

fn check(cert: &Path) -> Result<Output, TestError> {
    axiom(&[OsStr::new("check"), cert.as_os_str()])
}

#[test]
fn lean_accepted_certificate_exits_zero_with_verdicts_and_timing() -> Result<(), TestError> {
    if !lake_available("lean_accepted_certificate_exits_zero_with_verdicts_and_timing")? {
        return Ok(());
    }
    let output = check(&repo_root().join("fixtures/certificates/links-convergence.json"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(output.status.code() == Some(0), "expected exit status 0")?;
    ensure(
        stdout.starts_with("claim 0 Mass row 0: accepted\n")
            && stdout.contains("\nclaim 4 ConvergenceBound: accepted\n5 of 5 claims accepted in "),
        "missing verdicts or summary",
    )
}

#[test]
fn lean_rejected_claim_exits_one() -> Result<(), TestError> {
    if !lake_available("lean_rejected_claim_exits_one")? {
        return Ok(());
    }
    let output = check(
        &repo_root().join("fixtures/certificates/reject/termination-wrong-expected-attempts.json"),
    )?;
    ensure(output.status.code() == Some(1), "expected exit status 1")?;
    ensure(
        String::from_utf8_lossy(&output.stdout).contains("claim 0 Termination: rejected"),
        "missing rejected verdict",
    )
}

#[test]
fn lean_malformed_certificate_exits_with_status_two() -> Result<(), TestError> {
    if !lake_available("lean_malformed_certificate_exits_with_status_two")? {
        return Ok(());
    }
    let output = check(&repo_root().join("fixtures/certificates/reject/duplicate-key.json"))?;
    ensure(output.status.code() == Some(2), "expected exit status 2")?;
    ensure(
        output.stderr == b"axiom: checker-unexpected-field: the checker refused the certificate: unexpected or missing field\n",
        "missing mapped checker error",
    )
}

#[test]
fn lean_device_instead_of_a_file_exits_with_status_two() -> Result<(), TestError> {
    if !lake_available("lean_device_instead_of_a_file_exits_with_status_two")? {
        return Ok(());
    }
    let output = check(Path::new("/dev/zero"))?;
    ensure(output.status.code() == Some(2), "expected exit status 2")?;
    ensure(
        String::from_utf8_lossy(&output.stderr).starts_with("axiom: checker-not-regular-file: "),
        "missing regular file error",
    )
}

#[test]
fn missing_certificate_is_a_read_failure_with_status_three() -> Result<(), TestError> {
    let output = check(&repo_root().join("fixtures/certificates/no-such-file.json"))?;
    ensure(output.status.code() == Some(3), "expected exit status 3")?;
    ensure(
        output.stderr == b"axiom: read-failed: cannot read the certificate file: not found\n",
        "missing read failure",
    )
}

/// Runs `axiom check` with a `lake` on PATH that sleeps for ten seconds in two
/// processes, one in the background holding the output pipes as Lean would,
/// and
/// the time limit set to `limit`.
fn check_with_sleeping_lake(name: &str, limit: &str) -> Result<Output, TestError> {
    let dir = scratch(name)?;
    let lake = dir.join("lake");
    std::fs::write(&lake, "#!/bin/sh\n/bin/sleep 10 &\n/bin/sleep 10\n")?;
    let chmod = Command::new("chmod").arg("755").arg(&lake).status()?;
    ensure(chmod.success(), "could not make the stand-in executable")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_axiom"))
        .arg("check")
        .arg(repo_root().join("fixtures/certificates/links-mass.json"))
        .env("PATH", &dir)
        .env("AXIOM_CHECK_TIMEOUT_SECS", limit)
        .output()?)
}

#[test]
fn checker_over_its_time_limit_is_killed_with_status_three() -> Result<(), TestError> {
    let output = check_with_sleeping_lake("timeout", "1")?;
    ensure(output.status.code() == Some(3), "expected exit status 3")?;
    ensure(
        String::from_utf8_lossy(&output.stderr).starts_with("axiom: checker-timeout: "),
        "missing time limit error",
    )
}

#[test]
fn invalid_time_limit_in_the_environment_is_a_usage_error() -> Result<(), TestError> {
    for limit in ["0", "-1", "ten"] {
        let output = check_with_sleeping_lake("bad-timeout", limit)?;
        ensure(
            output.status.code() == Some(64),
            format!("{limit}: expected exit status 64"),
        )?;
    }
    Ok(())
}
