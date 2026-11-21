//! The `axiom` command line.
//!
//! `axiom analyse <MODEL> --cert <OUT>` reads a model and writes a
//! certificate of the strongest claims the engine can make about it; with
//! `--estimate --seed <U64>` it also samples the model and adds an
//! estimated claim. `axiom check <CERT>` runs the Lean checker on a
//! certificate and reports the verdict on each claim. The exit codes are
//! those of [`Outcome`], listed by `axiom --help`. Errors are printed as
//! `axiom: <code>: <message>`, the same bytes on every machine.

use std::process::ExitCode;

use axiom_cli::cli::{self, Cli, Command};
use axiom_cli::outcome::Outcome;
use axiom_cli::{analyse, check};
use clap::Parser;
use clap::error::ErrorKind;

fn main() -> ExitCode {
    let outcome = match Cli::try_parse_from(std::env::args_os()) {
        Ok(cli) => run(cli),
        Err(error) => {
            // Help and the version are answers; every other parse error gets
            // fixed words that repeat nothing typed.
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                Outcome::Accepted
            } else {
                eprintln!("{}", cli::usage_message(&error));
                Outcome::Usage
            }
        }
    };
    ExitCode::from(outcome.exit_code())
}

fn run(cli: Cli) -> Outcome {
    let result = match cli.command {
        Command::Analyse {
            model,
            cert,
            estimate,
        } => analyse::run(&model, &cert, estimate.request().as_ref()).map(|summary| {
            println!("{summary}");
            Outcome::Accepted
        }),
        Command::Check { cert, checker } => {
            check::run(&cert, checker.lean_dir.as_deref(), checker.timeout()).map(|report| {
                print!("{report}");
                if report.all_accepted() {
                    Outcome::Accepted
                } else {
                    Outcome::Rejected
                }
            })
        }
    };
    result.unwrap_or_else(|error| {
        eprintln!("{}", error.report());
        error.outcome()
    })
}
