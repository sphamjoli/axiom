//! Command line parsing.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::CliError;

/// The usage text printed by `axiom help` and after a usage error.
pub const USAGE: &str = "usage:
  axiom analyse <model.axm> --cert <out.json>
  axiom check <cert.json>
  axiom help

exit status: 0 success or every claim accepted, 1 some claim rejected,
2 usage or input error";

/// A parsed command.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Print the usage text.
    Help,
    /// Write a certificate for a model.
    Analyse {
        /// The model file to read.
        model: PathBuf,
        /// The certificate file to write.
        cert: PathBuf,
    },
    /// Check a certificate with the Lean checker.
    Check {
        /// The certificate file to check.
        cert: PathBuf,
    },
}

/// Parses the arguments that follow the program name.
pub fn parse(arguments: &[OsString]) -> Result<Command, CliError> {
    let Some((command, rest)) = arguments.split_first() else {
        return Err(CliError::Usage("missing command"));
    };
    match command.to_str() {
        Some("help" | "--help" | "-h") if rest.is_empty() => Ok(Command::Help),
        Some("analyse") => parse_analyse(rest),
        Some("check") => match rest {
            [cert] if !is_flag(cert) => Ok(Command::Check {
                cert: PathBuf::from(cert),
            }),
            _ => Err(CliError::Usage("check takes exactly one certificate path")),
        },
        _ => Err(CliError::Usage("unknown command")),
    }
}

fn parse_analyse(rest: &[OsString]) -> Result<Command, CliError> {
    let mut model = None;
    let mut cert = None;
    let mut items = rest.iter();
    while let Some(item) = items.next() {
        if item.to_str() == Some("--cert") {
            let value = items.next().ok_or(CliError::Usage("--cert needs a path"))?;
            if cert.replace(PathBuf::from(value)).is_some() {
                return Err(CliError::Usage("--cert given more than once"));
            }
        } else if is_flag(item) {
            return Err(CliError::Usage("unknown option"));
        } else if model.replace(PathBuf::from(item)).is_some() {
            return Err(CliError::Usage("analyse takes exactly one model path"));
        }
    }
    match (model, cert) {
        (Some(model), Some(cert)) => Ok(Command::Analyse { model, cert }),
        (None, _) => Err(CliError::Usage("analyse needs a model path")),
        (_, None) => Err(CliError::Usage("analyse needs --cert <out.json>")),
    }
}

fn is_flag(item: &OsString) -> bool {
    item.to_str().is_some_and(|text| text.starts_with('-'))
}
