//! The `analyse` command: reads a model and writes its certificate.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use axiom_cert::Certificate;
use axiom_core::TransitionMatrix;
use axiom_model::{CheckedModel, MAX_INPUT_BYTES};

use crate::error::CliError;

/// What `analyse` wrote, for the summary line.
#[derive(Debug)]
pub struct Summary {
    name: String,
    states: Option<usize>,
    claims: usize,
    cert: String,
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.states {
            Some(states) => write!(
                formatter,
                "chain {}: {states} states, {states} rows; wrote {} claims to {}",
                self.name, self.claims, self.cert
            ),
            None => write!(
                formatter,
                "retry {}: wrote {} claims to {}",
                self.name, self.claims, self.cert
            ),
        }
    }
}

/// The attempt count of the `failure_after` part of a `Termination` claim.
const FAILURE_ATTEMPTS: u64 = 20;

/// Reads the model at `model` and, for a model without parameters, writes a
/// certificate to `cert`: for a chain, a `Mass` claim for every row, plus
/// `FixedPoint`, `Contraction` and `ConvergenceBound` claims when the chain
/// has two states and settles; for a retry loop, a `Termination` claim.
///
/// # Errors
///
/// Returns an error when the model cannot be read, is invalid or has
/// parameters, or when the certificate cannot be written.
pub fn run(model: &Path, cert: &Path) -> Result<Summary, CliError> {
    let source = read_model(model)?;
    let checked = axiom_model::parse(&source).map_err(|source| CliError::Model {
        path: model.to_path_buf(),
        source,
    })?;
    let parametric = |kind| CliError::Unsupported {
        path: model.to_path_buf(),
        what: kind,
    };
    let (name, certificate) = match checked {
        CheckedModel::Chain(chain) => {
            let matrix = chain.matrix.ok_or_else(|| {
                parametric("parametric chains are not yet supported: no claim kind covers them")
            })?;
            (chain.name, Certificate::strongest(matrix))
        }
        CheckedModel::Retry(retry) => {
            let success = retry.probability.ok_or_else(|| {
                parametric("parametric retries are not yet supported: no claim kind covers them")
            })?;
            (
                retry.name,
                Certificate::termination(success, FAILURE_ATTEMPTS)?,
            )
        }
    };
    let text = certificate.to_json()?;
    std::fs::write(cert, text).map_err(|source| CliError::Io {
        path: cert.to_path_buf(),
        source,
    })?;
    Ok(Summary {
        name,
        states: certificate.matrix().map(TransitionMatrix::states),
        claims: certificate.claims().len(),
        cert: cert.display().to_string(),
    })
}


/// Reads a model file, refusing files larger than the model size limit
/// without reading more than one byte past it.
fn read_model(path: &Path) -> Result<String, CliError> {
    let io_error = |source| CliError::Io {
        path: path.to_path_buf(),
        source,
    };
    let file = File::open(path).map_err(io_error)?;
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_INPUT_BYTES).unwrap_or(u64::MAX);
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::TooLarge {
            path: path.to_path_buf(),
            limit: MAX_INPUT_BYTES,
        });
    }
    String::from_utf8(bytes).map_err(|_| CliError::NotUtf8 {
        path: path.to_path_buf(),
    })
}
