//! Tests of `axiom analyse` and argument handling.

mod common;

use std::ffi::OsStr;

use common::{TestError, axiom, ensure, repo_root, scratch};

#[test]
fn reproduces_golden_certificate() -> Result<(), TestError> {
    let dir = scratch("golden")?;
    let out = dir.join("links-convergence.json");
    let model = repo_root().join("fixtures/models/links.axm");
    let output = axiom(&[
        OsStr::new("analyse"),
        model.as_os_str(),
        OsStr::new("--cert"),
        out.as_os_str(),
    ])?;
    ensure(output.status.code() == Some(0), "analyse failed")?;
    let written = std::fs::read(&out)?;
    let golden = std::fs::read(repo_root().join("fixtures/certificates/links-convergence.json"))?;
    ensure(
        written == golden,
        "certificate differs from the golden file",
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(
        stdout.contains("2 states") && stdout.contains("2 rows"),
        "summary lacks the state and row counts",
    )
}

#[test]
fn reproduces_retry_golden_certificate() -> Result<(), TestError> {
    let dir = scratch("retry-golden")?;
    let out = dir.join("retry-termination.json");
    let model = repo_root().join("fixtures/models/retry.axm");
    let output = axiom(&[
        OsStr::new("analyse"),
        model.as_os_str(),
        OsStr::new("--cert"),
        out.as_os_str(),
    ])?;
    ensure(output.status.code() == Some(0), "analyse failed")?;
    let written = std::fs::read(&out)?;
    let golden = std::fs::read(repo_root().join("fixtures/certificates/retry-termination.json"))?;
    ensure(
        written == golden,
        "certificate differs from the golden file",
    )?;
    ensure(
        String::from_utf8_lossy(&output.stdout).contains("retry request: wrote 1 claims"),
        "summary does not describe the retry",
    )
}

#[test]
fn rejects_faulty_model_with_position_and_no_echo() -> Result<(), TestError> {
    let dir = scratch("faulty")?;
    let out = dir.join("out.json");
    let model = repo_root().join("fixtures/models/reject/missing-row.axm");
    let output = axiom(&[
        OsStr::new("analyse"),
        model.as_os_str(),
        OsStr::new("--cert"),
        out.as_os_str(),
    ])?;
    ensure(output.status.code() == Some(2), "expected exit status 2")?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    ensure(
        stderr.contains("line 2, column 14"),
        "error lacks the fault position",
    )?;
    ensure(
        !stderr.contains("states up") && !stderr.contains("chain links"),
        "error echoes the model text",
    )?;
    ensure(
        !out.exists(),
        "a certificate was written for a faulty model",
    )
}

#[test]
fn rejects_unsupported_models() -> Result<(), TestError> {
    let dir = scratch("unsupported")?;
    let out = dir.join("out.json");
    let retry_param = dir.join("retry-param.axm");
    std::fs::write(
        &retry_param,
        "param p in ]0, 1[\nretry request {\n  success p\n}\n",
    )?;
    let links_param = repo_root().join("fixtures/models/links-param.axm");
    for (model, kind) in [
        (links_param, "parametric chains"),
        (retry_param, "parametric retries"),
    ] {
        let file = model.display();
        let output = axiom(&[
            OsStr::new("analyse"),
            model.as_os_str(),
            OsStr::new("--cert"),
            out.as_os_str(),
        ])?;
        ensure(output.status.code() == Some(2), "expected exit status 2")?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure(
            stderr.contains(kind) && stderr.contains("not yet supported"),
            format!("{file}: error does not name the unsupported kind"),
        )?;
    }
    Ok(())
}

#[test]
fn usage_errors_exit_with_status_two() -> Result<(), TestError> {
    let cases: [&[&str]; 5] = [
        &[],
        &["frobnicate"],
        &["analyse", "model.axm"],
        &["analyse", "--cert"],
        &["check"],
    ];
    for args in cases {
        let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
        let output = axiom(&args)?;
        ensure(
            output.status.code() == Some(2),
            format!("{args:?}: expected exit status 2"),
        )?;
        ensure(
            String::from_utf8_lossy(&output.stderr).contains("usage:"),
            format!("{args:?}: no usage text"),
        )?;
    }
    Ok(())
}
