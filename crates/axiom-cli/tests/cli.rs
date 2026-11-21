//! The command line parser: its definition, the arguments it accepts, the
//! fixed usage messages and the exit codes the help lists.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use axiom_cli::cli::{Cli, Command, EstimateRequest, usage_message};
use axiom_cli::outcome::Outcome;
use clap::{CommandFactory, Parser};
use common::{TestError, ensure};
use num_bigint::BigInt;
use num_rational::BigRational;

fn parse(words: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("axiom").chain(words.iter().copied()))
}

/// The usage message for `words`, or `None` when they parse.
fn refused(words: &[&str]) -> Option<String> {
    parse(words).err().map(|error| usage_message(&error))
}

#[test]
fn the_command_line_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn analyse_takes_a_model_and_a_certificate_path() -> Result<(), TestError> {
    let cli = parse(&["analyse", "m.axm", "--cert", "out.json"])
        .map_err(|error| TestError::Unexpected(error.to_string()))?;
    let Command::Analyse {
        model,
        cert,
        estimate,
    } = cli.command
    else {
        return Err(TestError::Unexpected("not analyse".to_owned()));
    };
    ensure(
        (model, cert) == (PathBuf::from("m.axm"), PathBuf::from("out.json")),
        "paths differ",
    )?;
    ensure(estimate.request().is_none(), "an estimate was requested")
}

/// The estimate `analyse` is asked for by `extra`.
fn requested(extra: &[&str]) -> Result<Option<EstimateRequest>, TestError> {
    let mut words = vec!["analyse", "m.axm", "--cert", "out.json"];
    words.extend(extra);
    let cli = parse(&words).map_err(|error| TestError::Unexpected(error.to_string()))?;
    match cli.command {
        Command::Analyse { estimate, .. } => Ok(estimate.request()),
        Command::Check { .. } => Err(TestError::Unexpected("not analyse".to_owned())),
    }
}

#[test]
fn estimate_takes_a_seed_and_defaults_to_one_in_a_hundred_and_one_in_a_thousand()
-> Result<(), TestError> {
    let ratio = |n: u32, d: u32| BigRational::new(BigInt::from(n), BigInt::from(d));
    let max = u64::MAX.to_string();
    ensure(
        requested(&["--estimate", "--seed", &max])?
            == Some(EstimateRequest {
                seed: u64::MAX,
                epsilon: ratio(1, 100),
                delta: ratio(1, 1000),
            }),
        "defaults differ",
    )?;
    ensure(
        requested(&[
            "--estimate",
            "--seed",
            "0",
            "--epsilon",
            "1/20",
            "--delta",
            "2/4",
        ])? == Some(EstimateRequest {
            seed: 0,
            epsilon: ratio(1, 20),
            delta: ratio(1, 2),
        }),
        "explicit values differ",
    )
}

#[test]
fn estimate_options_need_each_other_and_well_formed_fractions() -> Result<(), TestError> {
    let digits = |n: usize| "1".repeat(n);
    let (sixty_four, sixty_five) = (format!("1/{}", digits(64)), format!("1/{}", digits(65)));
    ensure(
        requested(&["--estimate", "--seed", "1", "--epsilon", &sixty_four])?.is_some(),
        "64 digits refused",
    )?;
    let base = ["analyse", "m.axm", "--cert", "o.json"];
    let cases: [&[&str]; 8] = [
        &["--estimate"],
        &["--seed", "1"],
        &["--epsilon", "1/2"],
        &["--estimate", "--seed", "18446744073709551616"],
        &["--estimate", "--seed", "1", "--epsilon", "1/0"],
        &["--estimate", "--seed", "1", "--epsilon", "-1/2"],
        &["--estimate", "--seed", "1", "--delta", "0.5"],
        &["--estimate", "--seed", "1", "--epsilon", &sixty_five],
    ];
    for extra in cases {
        let words: Vec<&str> = base.iter().chain(extra).copied().collect();
        let message = refused(&words);
        ensure(
            message
                .as_deref()
                .is_some_and(|m| !m.contains("/2") && !m.contains("1844")),
            format!("{extra:?}: {message:?}"),
        )?;
    }
    Ok(())
}

#[test]
fn check_takes_a_lean_dir_and_a_time_limit_of_at_least_one_second() -> Result<(), TestError> {
    let cli = parse(&[
        "check",
        "c.json",
        "--lean-dir",
        "lean",
        "--timeout-secs",
        "1",
    ])
    .map_err(|error| TestError::Unexpected(error.to_string()))?;
    let Command::Check { cert, checker } = cli.command else {
        return Err(TestError::Unexpected("not check".to_owned()));
    };
    ensure(cert == Path::new("c.json"), "certificate path differs")?;
    ensure(
        checker.lean_dir == Some(PathBuf::from("lean")),
        "lean dir differs",
    )?;
    ensure(
        checker.timeout() == Duration::from_secs(1),
        "time limit differs",
    )?;
    ensure(
        refused(&["check", "c.json", "--timeout-secs", "0"]).is_some(),
        "a zero time limit is accepted",
    )
}

#[test]
fn usage_errors_never_repeat_what_was_typed() -> Result<(), TestError> {
    let cases: [(&[&str], &str); 7] = [
        (&[], "axiom: usage: missing command; see `axiom --help`"),
        (
            &["frobnicate-SECRET"],
            "axiom: usage: unknown command; see `axiom --help`",
        ),
        (
            &["analyse", "m.axm", "--cert", "o.json", "--SECRET"],
            "axiom: usage: unknown argument; see `axiom --help`",
        ),
        (
            &["analyse", "m.axm", "o-SECRET.json"],
            "axiom: usage: unknown argument; see `axiom --help`",
        ),
        (
            &["analyse", "m-SECRET.axm"],
            "axiom: usage: missing required argument: --cert <OUT>; see `axiom --help`",
        ),
        (
            &["check", "c.json", "--timeout-secs", "SECRET"],
            "axiom: usage: invalid value: --timeout-secs <SECONDS>; see `axiom --help`",
        ),
        (
            &["check", "c.json", "--timeout-secs", "-1"],
            "axiom: usage: unknown argument; see `axiom --help`",
        ),
    ];
    for (words, expected) in cases {
        let message = refused(words);
        ensure(
            message.as_deref() == Some(expected),
            format!("{words:?}: {message:?}"),
        )?;
    }
    Ok(())
}

#[test]
fn exit_codes_are_stable_and_listed_in_the_help() -> Result<(), TestError> {
    let codes: Vec<u8> = Outcome::ALL.iter().map(|o| o.exit_code()).collect();
    ensure(
        codes == [0, 1, 2, 3, 64],
        format!("codes changed: {codes:?}"),
    )?;
    let help = Cli::command().render_long_help().to_string();
    for outcome in Outcome::ALL {
        let line = format!("{:<3} {}", outcome.exit_code(), outcome.label());
        ensure(help.contains(&line), format!("help lacks {line:?}"))?;
    }
    Ok(())
}
