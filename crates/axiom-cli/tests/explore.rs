//! The `explore` command end to end: with the model supplied, the checker
//! promotes the links candidates to proven; with `--black-box`, they ship as
//! estimates under the limitation line; a candidate the engine cannot claim
//! stays estimated; and unsupported models are refused before sampling.

mod common;

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;

use axiom_cli::explore::{ESTIMATED, LIMITATION, PROVEN};
use common::{TestError, axiom, ensure, lake_available, repo_root, scratch};

/// Runs `explore` on the shared model fixture `model` with seed 7, and
/// returns its output and the certificate path it was given.
fn explore(test: &str, model: &str, black_box: bool) -> Result<(Output, PathBuf), TestError> {
    let out = scratch(test)?.join("out.json");
    let model = repo_root().join("fixtures/models").join(model);
    let mut args = vec![
        OsStr::new("explore"),
        model.as_os_str(),
        OsStr::new("--seed"),
        OsStr::new("7"),
        OsStr::new("--cert"),
        out.as_os_str(),
    ];
    if black_box {
        args.push(OsStr::new("--black-box"));
    }
    Ok((axiom(&args)?, out))
}

/// The candidate lines of an exploration's output.
fn candidates(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.starts_with("candidate "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn lean_promotes_the_links_candidates_when_the_model_is_supplied() -> Result<(), TestError> {
    let test = "lean_promotes_the_links_candidates_when_the_model_is_supplied";
    if !lake_available(test)? {
        return Ok(());
    }
    let (output, _) = explore(test, "links.axm", false)?;
    ensure(output.status.code() == Some(0), "exit status 0")?;
    let lines = candidates(&output);
    let expected = [
        format!("{ESTIMATED}; exact value 1/4 {PROVEN}"),
        format!("{ESTIMATED}; exact value 3/5 {PROVEN}"),
    ];
    ensure(
        lines.len() == 2
            && lines
                .iter()
                .zip(&expected)
                .all(|(line, end)| line.ends_with(end.as_str())),
        format!("{lines:?}"),
    )
}

#[test]
fn lean_black_box_candidates_ship_as_estimates_with_the_limitation() -> Result<(), TestError> {
    let test = "lean_black_box_candidates_ship_as_estimates_with_the_limitation";
    if !lake_available(test)? {
        return Ok(());
    }
    let (output, _) = explore(test, "links.axm", true)?;
    ensure(output.status.code() == Some(0), "exit status 0")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = candidates(&output);
    ensure(
        lines.len() == 2 && lines.iter().all(|line| line.ends_with(ESTIMATED)),
        format!("{lines:?}"),
    )?;
    ensure(!stdout.contains(PROVEN), "a black-box candidate was proven")?;
    ensure(
        stdout.lines().last() == Some(LIMITATION),
        "the limitation is the last line",
    )
}

#[test]
fn lean_a_candidate_the_engine_cannot_claim_stays_estimated() -> Result<(), TestError> {
    let test = "lean_a_candidate_the_engine_cannot_claim_stays_estimated";
    if !lake_available(test)? {
        return Ok(());
    }
    let (output, _) = explore(test, "periodic.axm", false)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = candidates(&output);
    ensure(
        lines.len() == 2 && lines.iter().all(|line| line.ends_with(ESTIMATED)),
        format!("{lines:?}"),
    )?;
    ensure(
        stdout.contains("non-convergent: the bounds admit |lambda| = 1"),
        "the periodic chain is flagged",
    )
}

#[test]
fn explore_refuses_models_its_candidates_do_not_cover() -> Result<(), TestError> {
    for model in ["retry.axm", "links-param.axm"] {
        let test = format!("explore_refuses_{}", model.replace(['.', '-'], "_"));
        let (output, out) = explore(&test, model, false)?;
        ensure(output.status.code() == Some(2), format!("{model}: exit 2"))?;
        ensure(
            String::from_utf8_lossy(&output.stderr)
                == "axiom: explore-unsupported: explore needs a two-state chain without parameters\n",
            format!("{model}: {}", String::from_utf8_lossy(&output.stderr)),
        )?;
        ensure(!out.exists(), "no certificate is written")?;
    }
    Ok(())
}
