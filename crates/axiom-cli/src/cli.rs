//! The command line, parsed by clap. Each command's arguments are typed
//! here. A parse error is a usage error, exit code [`Outcome::Usage`], and
//! its message never repeats what was typed.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use crate::check::{DEFAULT_TIMEOUT_SECS, LEAN_DIR_VARIABLE, TIMEOUT_VARIABLE};
use crate::outcome::Outcome;

/// Writes certificates for models and checks them with the Lean checker.
#[derive(Debug, Parser)]
#[command(
    name = "axiom",
    version,
    disable_help_subcommand = true,
    after_help = exit_codes()
)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// A command.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Read a model and, for a model without parameters, write a certificate
    /// of the strongest claims the engine can make about it.
    Analyse {
        /// The model file to read.
        #[arg(value_name = "MODEL")]
        model: PathBuf,
        /// The certificate file to write.
        #[arg(long, value_name = "OUT")]
        cert: PathBuf,
    },
    /// Run the Lean checker on a certificate and print the verdict on each
    /// claim.
    Check {
        /// The certificate file to check.
        #[arg(value_name = "CERT")]
        cert: PathBuf,
        /// Where and how long the checker runs.
        #[command(flatten)]
        checker: CheckerArgs,
    },
}

/// Where the Lean checker runs from and how long it may run.
#[derive(Debug, Args)]
pub struct CheckerArgs {
    /// The Lean project whose checker code runs; defaults to the `lean`
    /// directory of the source tree the binary was built from.
    #[arg(long, value_name = "DIR", env = LEAN_DIR_VARIABLE, hide_env_values = true)]
    pub lean_dir: Option<PathBuf>,
    /// The checker's wall-clock limit in whole seconds, at least 1.
    #[arg(
        long,
        value_name = "SECONDS",
        env = TIMEOUT_VARIABLE,
        hide_env_values = true,
        default_value_t = DEFAULT_TIMEOUT_SECS,
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub timeout_secs: u64,
}

impl CheckerArgs {
    /// The checker's wall-clock limit.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

/// The exit codes, generated from [`Outcome`] so the help cannot drift from
/// the codes the program returns.
#[must_use]
pub fn exit_codes() -> String {
    let codes: Vec<String> = Outcome::ALL
        .iter()
        .map(|outcome| format!("  {:<3} {}", outcome.exit_code(), outcome.label()))
        .collect();
    format!("Exit codes:\n{}", codes.join("\n"))
}

/// The fixed message for a parse error that is not a request for help or
/// the version. Unknown arguments, commands and invalid values are what was
/// typed, so they are never named; an option the program defines may be.
#[must_use]
pub fn usage_message(error: &clap::Error) -> String {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    let what = match error.kind() {
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => "invalid value",
        ErrorKind::UnknownArgument => "unknown argument",
        ErrorKind::InvalidSubcommand => "unknown command",
        ErrorKind::MissingRequiredArgument => "missing required argument",
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            "missing command"
        }
        ErrorKind::ArgumentConflict => "conflicting arguments",
        ErrorKind::TooManyValues | ErrorKind::WrongNumberOfValues | ErrorKind::TooFewValues => {
            "wrong number of values"
        }
        ErrorKind::InvalidUtf8 => "an argument is not valid UTF-8",
        _ => "invalid command line",
    };
    let named = !matches!(
        error.kind(),
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand | ErrorKind::InvalidUtf8
    );
    let flag = match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(flag)) if named => Some(flag.clone()),
        Some(ContextValue::Strings(flags)) if named => Some(flags.join(", ")),
        _ => None,
    };
    match flag {
        Some(flag) => format!("axiom: usage: {what}: {flag}; see `axiom --help`"),
        None => format!("axiom: usage: {what}; see `axiom --help`"),
    }
}
