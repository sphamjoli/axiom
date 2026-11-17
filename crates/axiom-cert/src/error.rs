use axiom_core::{ProbabilityError, TransitionMatrixError, TransitionRowError};
use thiserror::Error;

/// The reason a certificate could not be written or read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CertificateError {
    /// The certificate text is larger than the format allows.
    #[error("certificate is {bytes} bytes; the limit is {limit}")]
    TooLarge {
        /// The size of the rejected text in bytes.
        bytes: usize,
        /// The largest size the format allows.
        limit: usize,
    },
    /// The text is not valid JSON, or does not have the fields the format
    /// requires, or has fields it does not allow.
    #[error("certificate is not valid version 1 JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The `format` field is not `axiom-certificate`.
    #[error("unsupported certificate format {0:?}")]
    UnsupportedFormat(String),
    /// The `version` field names a version this reader does not implement.
    #[error("unsupported certificate version {0}")]
    UnsupportedVersion(u32),
    /// A fraction is not written as canonical decimal strings in lowest terms.
    #[error("fraction {num}/{den} is not in canonical form")]
    NonCanonicalFraction {
        /// The numerator text as written.
        num: String,
        /// The denominator text as written.
        den: String,
    },
    /// A fraction is not a probability.
    #[error("row {row}, entry {entry}: {source}")]
    Probability {
        /// The index of the row holding the entry.
        row: usize,
        /// The index of the entry within its row.
        entry: usize,
        /// Why the entry is not a probability.
        source: ProbabilityError,
    },
    /// A row of the model is not a transition row.
    #[error("row {row}: {source}")]
    TransitionRow {
        /// The index of the rejected row.
        row: usize,
        /// Why the row is not a transition row.
        source: TransitionRowError,
    },
    /// The rows do not form a transition matrix: there are none, or the
    /// matrix is not square.
    #[error(transparent)]
    TransitionMatrix(#[from] TransitionMatrixError),
    /// The certificate makes no claims, so there would be nothing to verify.
    #[error("certificate makes no claims")]
    NoClaims,
    /// A claim repeats an earlier claim.
    #[error("claim {claim} repeats claim {first}")]
    DuplicateClaim {
        /// The index of the repeated claim.
        claim: usize,
        /// The index of the earlier, identical claim.
        first: usize,
    },
    /// A claim refers to a row the model does not have.
    #[error("claim {claim} refers to row {row}, but the model has {rows} rows")]
    RowOutOfRange {
        /// The index of the claim in the claim list.
        claim: usize,
        /// The row the claim refers to.
        row: usize,
        /// The number of rows in the model.
        rows: usize,
    },
}
