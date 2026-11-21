//! The command line, parsed by clap. Each command's arguments are typed
//! here. A parse error is a usage error, exit code [`Outcome::Usage`], and
//! its message never repeats what was typed.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use num_bigint::BigInt;
use num_rational::BigRational;

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
        /// Whether to add an estimated claim from sampling.
        #[command(flatten)]
        estimate: EstimateArgs,
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

/// The options of `analyse --estimate`: sample the model from a seed and add
/// an `Estimate` claim. Without `--estimate` none of them may be given, and
/// the certificate is the same as before estimates existed.
#[derive(Debug, Args)]
pub struct EstimateArgs {
    /// Add an `Estimate` claim from sampling the model; needs `--seed`.
    #[arg(long, requires = "seed")]
    pub estimate: bool,
    /// The seed the samples are drawn with.
    #[arg(long, value_name = "U64", requires = "estimate")]
    pub seed: Option<u64>,
    /// The tolerance, a fraction such as 1/100 strictly between zero and one.
    #[arg(long, value_name = "FRACTION", requires = "estimate", value_parser = fraction)]
    pub epsilon: Option<BigRational>,
    /// The failure probability, a fraction such as 1/1000 strictly between
    /// zero and one.
    #[arg(long, value_name = "FRACTION", requires = "estimate", value_parser = fraction)]
    pub delta: Option<BigRational>,
}

/// The tolerance used when `--epsilon` is not given.
pub const DEFAULT_EPSILON: (u32, u32) = (1, 100);

/// The failure probability used when `--delta` is not given.
pub const DEFAULT_DELTA: (u32, u32) = (1, 1000);

/// The longest numerator or denominator `--epsilon` and `--delta` accept,
/// in decimal digits, the certificate format's limit.
const MAX_FRACTION_DIGITS: usize = 64;

/// What `analyse --estimate` was asked for, with the defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimateRequest {
    /// The seed the samples are drawn with.
    pub seed: u64,
    /// The tolerance.
    pub epsilon: BigRational,
    /// The failure probability.
    pub delta: BigRational,
}

impl EstimateArgs {
    /// The estimate requested, or `None` without `--estimate`. Clap has
    /// already required `--seed` with `--estimate`.
    #[must_use]
    pub fn request(&self) -> Option<EstimateRequest> {
        let ratio = |(n, d): (u32, u32)| BigRational::new(BigInt::from(n), BigInt::from(d));
        let seed = self.seed.filter(|_| self.estimate)?;
        Some(EstimateRequest {
            seed,
            epsilon: self
                .epsilon
                .clone()
                .unwrap_or_else(|| ratio(DEFAULT_EPSILON)),
            delta: self.delta.clone().unwrap_or_else(|| ratio(DEFAULT_DELTA)),
        })
    }
}

/// Parses `n/d` or `n`, with unsigned decimal parts of at most 64 digits and
/// a positive denominator. The error names no part of the input; the usage
/// message reports only that the value is invalid.
fn fraction(text: &str) -> Result<BigRational, &'static str> {
    let part = |digits: &str| -> Option<BigInt> {
        let ok = !digits.is_empty()
            && digits.len() <= MAX_FRACTION_DIGITS
            && digits.bytes().all(|byte| byte.is_ascii_digit());
        ok.then(|| digits.parse().ok()).flatten()
    };
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    match (part(numerator), part(denominator)) {
        (Some(numerator), Some(denominator)) if denominator != BigInt::from(0) => {
            Ok(BigRational::new(numerator, denominator))
        }
        _ => Err("expected a fraction such as 1/100"),
    }
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
