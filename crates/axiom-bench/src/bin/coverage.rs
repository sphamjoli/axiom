//! The `axiom-coverage` command: writes the verification coverage report,
//! `coverage.md` and `coverage.csv`, from the source tree. Errors are printed
//! as `axiom-coverage: <code>: <message>`, the same bytes on every machine.

use std::path::PathBuf;
use std::process::ExitCode;

use axiom_bench::coverage::{self, FunctionStatus};
use clap::Parser;
use clap::error::ErrorKind;

/// Reports which claim kinds and engine functions Lean proves correct.
#[derive(Debug, Parser)]
#[command(name = "axiom-coverage", version, disable_help_subcommand = true)]
struct Args {
    /// The repository root.
    #[arg(long, value_name = "DIR", default_value = ".")]
    root: PathBuf,
    /// Where `coverage.md` and `coverage.csv` are written.
    #[arg(long, value_name = "DIR", default_value = "bench")]
    out: PathBuf,
}

fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return ExitCode::SUCCESS;
            }
            eprintln!("axiom-coverage: usage: invalid command line; see `axiom-coverage --help`");
            return ExitCode::from(2);
        }
    };
    let result = coverage::generate(&args.root)
        .and_then(|report| coverage::write(&report, &args.out).map(|()| report));
    match result {
        Ok(report) => {
            println!(
                "{} of {} claim kinds proved; {} of {} engine functions proved",
                report.kinds.iter().filter(|kind| kind.proved).count(),
                report.kinds.len(),
                report.count(FunctionStatus::Proved),
                report.functions.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error.report());
            ExitCode::FAILURE
        }
    }
}
