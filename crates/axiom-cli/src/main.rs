//! The `axiom` command line.
//!
//! `axiom analyse <model.axm> --cert <out.json>` reads a model and, for a
//! chain without parameters, writes a certificate with a `Mass` claim for
//! every row. `axiom check <cert.json>` runs the Lean checker on a
//! certificate and reports the verdict on each claim.
//!
//! Exit status: 0 when the command succeeded and, for `check`, every claim
//! was accepted; 1 when `check` found a rejected claim; 2 for a usage error,
//! an unreadable or invalid input, an unsupported model or a checker that
//! could not be run.

mod analyse;
mod check;
mod cli;
mod error;

use std::process::ExitCode;

use cli::Command;
use error::CliError;

/// The exit status when every claim was accepted or the command succeeded.
const EXIT_OK: u8 = 0;
/// The exit status when the checker rejected at least one claim.
const EXIT_REJECTED: u8 = 1;
/// The exit status for usage and input errors.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            if matches!(error, CliError::Usage(_)) {
                eprintln!("{}", cli::USAGE);
            }
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(arguments: &[std::ffi::OsString]) -> Result<u8, CliError> {
    match cli::parse(arguments)? {
        Command::Help => {
            println!("{}", cli::USAGE);
            Ok(EXIT_OK)
        }
        Command::Analyse { model, cert } => {
            let summary = analyse::run(&model, &cert)?;
            println!("{summary}");
            Ok(EXIT_OK)
        }
        Command::Check { cert } => {
            let report = check::run(&cert)?;
            print!("{report}");
            Ok(if report.all_accepted() {
                EXIT_OK
            } else {
                EXIT_REJECTED
            })
        }
    }
}
