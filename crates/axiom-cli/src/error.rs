//! Errors reported by the command line.
//!
//! The same failure prints the same bytes on every machine: messages state
//! positions, counts and limits, and name files by their role, never by
//! path. They never quote file contents, operating system messages or the
//! checker's error stream. Each error has a stable kebab-case code and is
//! printed as `axiom: <code>: <message>`.

use crate::checker_rule::CheckerRule;
use std::fmt;
use std::io;

use axiom_cert::CertificateError;
use axiom_model::ModelError;
use thiserror::Error;

use crate::outcome::Outcome;

/// Which file an operation was about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    /// The model read by `analyse`.
    Model,
    /// The certificate written by `analyse` or read by `check`.
    Certificate,
}

impl FileRole {
    /// Every role.
    pub const ALL: [Self; 2] = [Self::Model, Self::Certificate];
}

impl fmt::Display for FileRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Model => "model",
            Self::Certificate => "certificate",
        })
    }
}

/// Why a file operation failed, reduced to a fixed word so the operating
/// system's message, which varies with platform and locale, is never shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoFailure {
    /// The file does not exist.
    NotFound,
    /// The file may not be read or written.
    PermissionDenied,
    /// The path names a directory.
    IsADirectory,
    /// Any other failure.
    Other,
}

impl IoFailure {
    /// Every failure kind.
    pub const ALL: [Self; 4] = [
        Self::NotFound,
        Self::PermissionDenied,
        Self::IsADirectory,
        Self::Other,
    ];
}

impl From<&io::Error> for IoFailure {
    fn from(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::IsADirectory => Self::IsADirectory,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for IoFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFound => "not found",
            Self::PermissionDenied => "permission denied",
            Self::IsADirectory => "is a directory",
            Self::Other => "input or output error",
        })
    }
}

/// A reason the Lean checker gave on an `error:` line. The checker's
/// reasons are fixed strings; this maps them onto codes and never forwards
/// the line itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CheckerReason {
    /// The certificate path does not name a regular file.
    #[error("the checker refused the certificate: not a regular file")]
    NotRegularFile,
    /// The certificate is larger than the checker's limit.
    #[error("the checker refused the certificate: larger than its limit")]
    TooLarge,
    /// The certificate is not UTF-8.
    #[error("the checker refused the certificate: not UTF-8")]
    NotUtf8,
    /// The checker could not read the certificate.
    #[error("the checker could not read the certificate")]
    Unreadable,
    /// The checker was not given exactly one argument.
    #[error("the checker was run with the wrong arguments")]
    Arguments,
    /// The checker refused the certificate under a named reading rule.
    #[error("the checker refused the certificate: {0}")]
    Rule(CheckerRule),
    /// The checker refused the certificate for a reason this program does
    /// not know, for example after the checker gained a rule.
    #[error("the checker refused the certificate as invalid")]
    Invalid,
}

impl CheckerReason {
    /// Every reason that names no reading rule; [`CheckerRule::ALL`] lists
    /// the rest.
    pub const ALL: [Self; 6] = [
        Self::NotRegularFile,
        Self::TooLarge,
        Self::NotUtf8,
        Self::Unreadable,
        Self::Arguments,
        Self::Invalid,
    ];

    /// Maps the text after `error: ` on a checker line to a reason. Any text
    /// the checker's reading and argument steps do not print is a parse or
    /// validation failure: the [`CheckerRule`] it names, or
    /// [`CheckerReason::Invalid`] when it names none.
    #[must_use]
    pub fn from_line(reason: &str) -> Self {
        match reason {
            "certificate is not a regular file" => Self::NotRegularFile,
            "certificate is too large" => Self::TooLarge,
            "certificate is not UTF-8" => Self::NotUtf8,
            "certificate cannot be read" => Self::Unreadable,
            "expected exactly one argument, the certificate path" => Self::Arguments,
            other => CheckerRule::from_line(other).map_or(Self::Invalid, Self::Rule),
        }
    }

    /// A stable kebab-case name for the reason.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotRegularFile => "checker-not-regular-file",
            Self::TooLarge => "checker-too-large",
            Self::NotUtf8 => "checker-not-utf8",
            Self::Unreadable => "checker-unreadable",
            Self::Arguments => "checker-arguments",
            Self::Rule(rule) => rule.code(),
            Self::Invalid => "checker-invalid-certificate",
        }
    }

    /// Whether the reason is about the certificate rather than about the
    /// checker's ability to run.
    const fn outcome(self) -> Outcome {
        match self {
            Self::NotRegularFile
            | Self::TooLarge
            | Self::NotUtf8
            | Self::Rule(_)
            | Self::Invalid => Outcome::Refused,
            Self::Unreadable | Self::Arguments => Outcome::Failed,
        }
    }
}

/// A failure that ends a command.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CliError {
    /// A file could not be read.
    #[error("cannot read the {file} file: {failure}")]
    ReadFailed {
        /// Which file.
        file: FileRole,
        /// Why, as a fixed word.
        failure: IoFailure,
    },
    /// A file could not be written.
    #[error("cannot write the {file} file: {failure}")]
    WriteFailed {
        /// Which file.
        file: FileRole,
        /// Why, as a fixed word.
        failure: IoFailure,
    },
    /// The model file is larger than the model size limit.
    #[error("model file is larger than the limit of {limit} bytes")]
    ModelTooLarge {
        /// The largest accepted size in bytes.
        limit: usize,
    },
    /// The model file is not valid UTF-8.
    #[error("model file is not valid UTF-8")]
    ModelNotUtf8,
    /// The model was rejected by the parser or the static checks.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// The model is a chain with parameters that is not a two-state chain
    /// whose exit probabilities are exactly its two parameters, the only
    /// parametric shape a claim kind covers.
    #[error("parametric chain is not a two-state chain whose exits are its two parameters")]
    ParametricChain,
    /// The model is a retry loop with parameters, for which no claim kind
    /// exists.
    #[error("parametric retries are not yet supported: no claim kind covers them")]
    ParametricRetry,
    /// The certificate could not be built or encoded.
    #[error(transparent)]
    Certificate(#[from] CertificateError),
    /// `lake` could not be started or waited for.
    #[error("cannot run the Lean checker with lake")]
    CheckerNotStarted,
    /// The checker ran longer than its wall-clock limit and was killed.
    #[error("the Lean checker was killed after the limit of {seconds} s")]
    CheckerTimeout {
        /// The limit in seconds.
        seconds: u64,
    },
    /// The checker reported an error on an `error:` line.
    #[error(transparent)]
    CheckerRefused(CheckerReason),
    /// The checker exited without a verdict or an error line.
    #[error("the Lean checker exited without a verdict")]
    CheckerNoVerdict,
    /// The checker printed a line that is neither a verdict nor an error.
    #[error("the Lean checker printed an unexpected line")]
    CheckerUnexpectedOutput,
    /// The checker's exit status disagrees with its verdicts.
    #[error("the Lean checker's exit status disagrees with its verdicts")]
    CheckerStatusMismatch,
}

impl CliError {
    /// A stable kebab-case name for the error. A model or certificate error
    /// keeps its own crate's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ReadFailed { .. } => "read-failed",
            Self::WriteFailed { .. } => "write-failed",
            Self::ModelTooLarge { .. } => "model-too-large",
            Self::ModelNotUtf8 => "model-not-utf8",
            Self::Model(error) => error.code(),
            Self::ParametricChain => "parametric-chain",
            Self::ParametricRetry => "parametric-retry",
            Self::Certificate(error) => error.code(),
            Self::CheckerNotStarted => "checker-not-started",
            Self::CheckerTimeout { .. } => "checker-timeout",
            Self::CheckerRefused(reason) => reason.code(),
            Self::CheckerNoVerdict => "checker-no-verdict",
            Self::CheckerUnexpectedOutput => "checker-unexpected-output",
            Self::CheckerStatusMismatch => "checker-status-mismatch",
        }
    }

    /// How a command that ends with this error ended: refused for an input
    /// that is invalid or unsupported, failed otherwise.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        match self {
            Self::ModelTooLarge { .. }
            | Self::ModelNotUtf8
            | Self::Model(_)
            | Self::ParametricChain
            | Self::ParametricRetry
            | Self::Certificate(_) => Outcome::Refused,
            Self::CheckerRefused(reason) => reason.outcome(),
            Self::ReadFailed { .. }
            | Self::WriteFailed { .. }
            | Self::CheckerNotStarted
            | Self::CheckerTimeout { .. }
            | Self::CheckerNoVerdict
            | Self::CheckerUnexpectedOutput
            | Self::CheckerStatusMismatch => Outcome::Failed,
        }
    }

    /// The line printed on the error stream: `axiom: <code>: <message>`.
    #[must_use]
    pub fn report(&self) -> String {
        format!("axiom: {}: {self}", self.code())
    }
}
