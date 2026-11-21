//! The `analyse` command: reads a model and writes its certificate.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use axiom_cert::Certificate;
use axiom_model::{CheckedModel, MAX_INPUT_BYTES};

use crate::error::CliError;

/// What `analyse` wrote, for the summary line.
#[derive(Debug)]
pub struct Summary {
    name: String,
    states: usize,
    claims: usize,
    cert: String,
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "chain {}: {} states, {} rows; wrote {} Mass claims to {}",
            self.name, self.states, self.states, self.claims, self.cert
        )
    }
}

/// Reads the model at `model` and, for a chain without parameters, writes a
/// certificate with a `Mass` claim for every row to `cert`.
pub fn run(model: &Path, cert: &Path) -> Result<Summary, CliError> {
    let source = read_model(model)?;
    let checked = axiom_model::parse(&source).map_err(|source| CliError::Model {
        path: model.to_path_buf(),
        source,
    })?;
    let chain = match checked {
        CheckedModel::Chain(chain) => chain,
        CheckedModel::Retry(_) => {
            return Err(CliError::Unsupported {
                path: model.to_path_buf(),
                what: "retry models are not yet supported: no claim kind covers them",
            });
        }
    };
    let Some(matrix) = chain.matrix else {
        return Err(CliError::Unsupported {
            path: model.to_path_buf(),
            what: "parametric chains are not yet supported: no claim kind covers them",
        });
    };
    let certificate = Certificate::mass(matrix);
    let text = certificate.to_json()?;
    std::fs::write(cert, text).map_err(|source| CliError::Io {
        path: cert.to_path_buf(),
        source,
    })?;
    Ok(Summary {
        name: chain.name,
        states: certificate.matrix().states(),
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
