//! End-to-end tests of `axiom analyse`, of how the binary reports usage
//! errors and of the bytes it prints for a failure.

mod common;

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

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

/// Runs `axiom analyse <model> --cert <out>` from `cwd`.
fn analyse_in(cwd: &Path, model: &Path, out: &Path) -> Result<Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_axiom"))
        .arg("analyse")
        .arg(model)
        .arg("--cert")
        .arg(out)
        .current_dir(cwd)
        .output()?)
}

#[test]
fn rejects_faulty_model_with_its_code_and_position() -> Result<(), TestError> {
    let dir = scratch("faulty")?;
    let out = dir.join("out.json");
    let model = repo_root().join("fixtures/models/reject/missing-row.axm");
    let output = analyse_in(&dir, &model, &out)?;
    ensure(output.status.code() == Some(2), "expected exit status 2")?;
    ensure(
        output.stderr == b"axiom: missing-row: line 2, column 14: state has no row\n",
        format!("stderr: {}", String::from_utf8_lossy(&output.stderr)),
    )?;
    ensure(
        !out.exists(),
        "a certificate was written for a faulty model",
    )
}

#[test]
fn the_same_failure_prints_the_same_bytes_from_any_directory() -> Result<(), TestError> {
    let source = std::fs::read(repo_root().join("fixtures/models/reject/missing-row.axm"))?;
    let mut faulty = Vec::new();
    let mut missing = Vec::new();
    for name in ["determinism-first", "determinism-second-elsewhere"] {
        let dir = scratch(name)?;
        let model = dir.join(format!("{name}.axm"));
        std::fs::write(&model, &source)?;
        let out = dir.join("out.json");
        // The same command twice, then by a relative path from the model's
        // own directory.
        faulty.push(analyse_in(&repo_root(), &model, &out)?.stderr);
        faulty.push(analyse_in(&repo_root(), &model, &out)?.stderr);
        faulty.push(analyse_in(&dir, Path::new(&format!("{name}.axm")), &out)?.stderr);
        missing.push(analyse_in(&dir, &dir.join("absent.axm"), &out)?.stderr);
    }
    for (what, runs) in [("model error", &faulty), ("read failure", &missing)] {
        ensure(
            runs.windows(2).all(|pair| pair.first() == pair.last()),
            format!("a {what} differs between runs"),
        )?;
        let text: String = runs
            .iter()
            .map(|run| String::from_utf8_lossy(run))
            .collect();
        ensure(
            text.starts_with("axiom: ") && !text.contains("determinism") && !text.contains('/'),
            format!("a {what} names a path: {text}"),
        )?;
    }
    Ok(())
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
    for (model, code) in [
        (links_param, "axiom: parametric-chain: "),
        (retry_param, "axiom: parametric-retry: "),
    ] {
        let output = analyse_in(&dir, &model, &out)?;
        ensure(output.status.code() == Some(2), "expected exit status 2")?;
        ensure(
            String::from_utf8_lossy(&output.stderr).starts_with(code),
            format!("{code}: missing"),
        )?;
    }
    Ok(())
}

#[test]
fn usage_errors_exit_with_status_sixty_four_and_one_fixed_line() -> Result<(), TestError> {
    let cases: [&[&str]; 4] = [&[], &["frobnicate"], &["analyse", "model.axm"], &["check"]];
    for args in cases {
        let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
        let output = axiom(&args)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure(
            output.status.code() == Some(64),
            format!("{args:?}: expected exit status 64"),
        )?;
        ensure(
            stderr.starts_with("axiom: usage: ") && stderr.lines().count() == 1,
            format!("{args:?}: {stderr}"),
        )?;
    }
    Ok(())
}

#[test]
fn help_and_version_exit_zero() -> Result<(), TestError> {
    for flag in ["--help", "--version"] {
        let output = axiom(&[OsStr::new(flag)])?;
        ensure(
            output.status.code() == Some(0) && !output.stdout.is_empty(),
            format!("{flag}: expected exit status 0 and output"),
        )?;
    }
    Ok(())
}
