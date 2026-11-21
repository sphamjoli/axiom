//! The `axiom-bench` command: measures the engine, the Lean checker and
//! simulation on every fixture and the scaling family, and writes CSV files,
//! a Markdown table and SVG plots. Errors are printed as
//! `axiom-bench: <code>: <message>`, the same bytes on every machine.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use axiom_bench::run::{self, Settings};
use clap::Parser;
use clap::error::ErrorKind;

/// Measures checking against simulating to the same confidence.
#[derive(Debug, Parser)]
#[command(name = "axiom-bench", version, disable_help_subcommand = true)]
struct Args {
    /// The fixtures directory.
    #[arg(long, value_name = "DIR", default_value = "fixtures/models")]
    fixtures: PathBuf,
    /// Where the CSV files, the table and the plots are written.
    #[arg(long, value_name = "DIR", default_value = "bench")]
    out: PathBuf,
    /// Where the certificates handed to the checker are written.
    #[arg(long, value_name = "DIR", default_value = "target/bench")]
    work: PathBuf,
    /// The Lean project the checker runs from; defaults to the source tree's.
    #[arg(long, value_name = "DIR")]
    lean_dir: Option<PathBuf>,
    /// How many times each certificate is checked; the median is kept.
    #[arg(long, value_name = "N", default_value_t = 3, value_parser = clap::value_parser!(u64).range(1..=100))]
    repeat: u64,
    /// The most random draws one simulation job runs before its time is
    /// extrapolated linearly.
    #[arg(long, value_name = "DRAWS", default_value_t = 200_000_000, value_parser = clap::value_parser!(u64).range(1..))]
    sim_budget: u64,
    /// The seed every simulation draws from.
    #[arg(long, value_name = "U64", default_value_t = 1)]
    seed: u64,
    /// The checker's wall-clock limit in whole seconds.
    #[arg(long, value_name = "SECONDS", default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..))]
    timeout_secs: u64,
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
            eprintln!("axiom-bench: usage: invalid command line; see `axiom-bench --help`");
            return ExitCode::from(2);
        }
    };
    let settings = Settings {
        fixtures: args.fixtures,
        out: args.out,
        work: args.work,
        lean_dir: args.lean_dir,
        repeat: usize::try_from(args.repeat).unwrap_or(1),
        sim_budget: args.sim_budget,
        seed: args.seed,
        timeout: Duration::from_secs(args.timeout_secs),
    };
    match run::run(&settings, &mut |case| eprintln!("measuring {case}")) {
        Ok(measurements) => {
            println!(
                "measured {} cases and {} claim kinds",
                measurements.runs.len(),
                measurements.comparisons.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error.report());
            ExitCode::FAILURE
        }
    }
}
