//! The command line parser: its definition, the arguments it accepts, the
//! fixed usage messages and the exit codes the help lists.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use axiom_cli::cli::{Cli, Command, usage_message};
use axiom_cli::outcome::Outcome;
use clap::{CommandFactory, Parser};
use common::{TestError, ensure};

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
    let Command::Analyse { model, cert } = cli.command else {
        return Err(TestError::Unexpected("not analyse".to_owned()));
    };
    ensure(
        (model, cert) == (PathBuf::from("m.axm"), PathBuf::from("out.json")),
        "paths differ",
    )
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
