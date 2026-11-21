//! Errors reported by the command line.
//!
//! Messages name files and positions but never quote file contents.

use std::path::PathBuf;

use axiom_cert::CertificateError;
use axiom_model::ModelError;
use thiserror::Error;

/// A failure that ends a command with exit status 2.
#[derive(Debug, Error)]
pub enum CliError {
    /// The arguments do not form a valid command.
    #[error("{0}")]
    Usage(&'static str),
    /// A file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// A model file is larger than the model size limit.
    #[error("{}: file is larger than the limit of {limit} bytes", path.display())]
    TooLarge {
        /// The file.
        path: PathBuf,
        /// The largest accepted size in bytes.
        limit: usize,
    },
    /// A model file is not valid UTF-8.
    #[error("{}: file is not valid UTF-8", path.display())]
    NotUtf8 {
        /// The file.
        path: PathBuf,
    },
    /// A model file was rejected by the parser or the static checks.
    #[error("{}: {source}", path.display())]
    Model {
        /// The file.
        path: PathBuf,
        /// The fault and its position.
        source: ModelError,
    },
    /// The model is of a kind for which no claim can be written yet.
    #[error("{}: {what}", path.display())]
    Unsupported {
        /// The file.
        path: PathBuf,
        /// What is not supported.
        what: &'static str,
    },
    /// The certificate could not be written.
    #[error(transparent)]
    Certificate(#[from] CertificateError),
    /// The Lean checker could not be started.
    #[error("cannot run the Lean checker with lake in {}: {source}", dir.display())]
    LeanStart {
        /// The Lean project directory.
        dir: PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// The Lean checker reported an error or produced unexpected output.
    #[error("Lean checker: {0}")]
    Lean(String),
}
