//! End-to-end tests of `axiom analyse`, of how the binary reports usage
//! errors and of the bytes it prints for a failure.

mod common;

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

use common::{TestError, axiom, ensure, repo_root, scratch};

/// Runs `axiom analyse` on `model` from the repository's fixtures with
/// `extra` arguments, and checks that it writes the golden certificate
/// `golden` byte for byte and prints `summary`.
fn reproduces(model: &str, extra: &[&str], golden: &str, summary: &str) -> Result<(), TestError> {
    let dir = scratch(golden)?;
    let out = dir.join(golden);
    let model = repo_root().join("fixtures/models").join(model);
    let mut args = vec![
        OsStr::new("analyse"),
        model.as_os_str(),
        OsStr::new("--cert"),
        out.as_os_str(),
    ];
    args.extend(extra.iter().map(OsStr::new));
    let output = axiom(&args)?;
    ensure(
        output.status.code() == Some(0),
        format!("{golden}: analyse failed"),
    )?;
    let written = std::fs::read(&out)?;
    let expected = std::fs::read(repo_root().join("fixtures/certificates").join(golden))?;
    ensure(
        written == expected,
        format!("{golden}: certificate differs from the golden file"),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    ensure(
        stdout.contains(summary),
        format!("{golden}: summary: {stdout}"),
    )
}

#[test]
fn reproduces_the_golden_certificates() -> Result<(), TestError> {
    reproduces(
        "links.axm",
        &[],
        "links-convergence.json",
        "chain links: 2 states, 2 rows; wrote 5 claims",
    )?;
    reproduces(
        "retry.axm",
        &[],
        "retry-termination.json",
        "retry request: wrote 1 claims",
    )?;
    reproduces(
        "links-param.axm",
        &[],
        "links-param.json",
        "parametric chain links: wrote 1 claims",
    )?;
    reproduces(
        "links.axm",
        &["--estimate", "--seed", "42"],
        "links-estimate.json",
        "chain links: 2 states, 2 rows; wrote 6 claims",
    )
}

#[test]
fn refuses_an_estimate_it_cannot_make() -> Result<(), TestError> {
    let dir = scratch("estimate-refused")?;
    let out = dir.join("out.json");
    let fixtures = repo_root().join("fixtures/models");
    let cases = [
        ("links.axm", "1", "axiom: invalid-epsilon: "),
        ("links.axm", "1/100000", "axiom: samples-out-of-range: "),
        ("links-param.axm", "1/100", "axiom: parametric-model: "),
    ];
    for (model, epsilon, code) in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_axiom"))
            .arg("analyse")
            .arg(fixtures.join(model))
            .args(["--cert"])
            .arg(&out)
            .args(["--estimate", "--seed", "1", "--epsilon", epsilon])
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure(
            output.status.code() == Some(2) && stderr.starts_with(code),
            format!("{model} {epsilon}: {stderr}"),
        )?;
        ensure(!out.exists(), format!("{model}: a certificate was written"))?;
    }
    Ok(())
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
fn refuses_parametric_models_no_claim_covers() -> Result<(), TestError> {
    let dir = scratch("unsupported")?;
    let out = dir.join("out.json");
    let links = |a: &str, b: &str| {
        format!(
            "param a in {a}\nparam b in {b}\nchain links {{\n  states up, down\n  from up:   up 1 - a, down a\n  from down: up b, down 1 - b\n}}\n"
        )
    };
    let cases = [
        (
            "retry-param.axm",
            "param p in ]0, 1[\nretry request {\n  success p\n}\n".to_owned(),
            "axiom: parametric-retry: ",
        ),
        (
            "half-exit.axm",
            "param a in ]0, 1[\nparam b in ]0, 1[\nchain links {\n  states up, down\n  from up:   up 1 - 1/2*a, down 1/2*a\n  from down: up b, down 1 - b\n}\n".to_owned(),
            "axiom: parametric-chain: ",
        ),
        (
            "reaches-two.axm",
            links("]0, 1]", "]0, 1]"),
            "axiom: no-parametric-contraction: ",
        ),
        (
            "reaches-zero.axm",
            links("[0, 1[", "[0, 1["),
            "axiom: no-parametric-contraction: ",
        ),
    ];
    for (name, source, code) in cases {
        let model = dir.join(name);
        std::fs::write(&model, source)?;
        let output = analyse_in(&dir, &model, &out)?;
        ensure(
            output.status.code() == Some(2),
            format!("{name}: expected exit status 2"),
        )?;
        ensure(
            String::from_utf8_lossy(&output.stderr).starts_with(code),
            format!("{name}: {}", String::from_utf8_lossy(&output.stderr)),
        )?;
        ensure(!out.exists(), format!("{name}: a certificate was written"))?;
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
