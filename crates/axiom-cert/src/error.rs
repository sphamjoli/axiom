use axiom_core::{ProbabilityError, TransitionMatrixError, TransitionRowError, TwoStateChainError};
use thiserror::Error;

/// The reason a certificate could not be written or read.
///
/// No variant echoes text from the certificate. Errors report positions,
/// counts and limits only, so a hostile certificate cannot fill a log with
/// its own content or inject terminal control sequences through an error
/// message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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
    /// The text is not valid JSON, has a field the format does not allow,
    /// repeats a field, lacks a required field, or has a value of the wrong
    /// JSON type.
    #[error("certificate is not valid certificate JSON (line {line}, column {column})")]
    Json {
        /// The one-based line where parsing stopped.
        line: usize,
        /// The one-based column where parsing stopped.
        column: usize,
    },
    /// The `format` field is not `axiom-certificate`.
    #[error("certificate format is not axiom-certificate")]
    UnsupportedFormat,
    /// The `version` field names a version this reader does not implement.
    #[error("unsupported certificate version {0}")]
    UnsupportedVersion(u32),
    /// The model has more states than the format allows.
    #[error("model has {states} states; the limit is {limit}")]
    TooManyStates {
        /// The number of rows in the model.
        states: usize,
        /// The largest number of states the format allows.
        limit: usize,
    },
    /// A fraction is not written as canonical decimal strings in lowest terms.
    #[error("row {row}, entry {entry}: fraction is not in canonical form")]
    NonCanonicalFraction {
        /// The index of the row holding the entry.
        row: usize,
        /// The index of the entry within its row.
        entry: usize,
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
    /// A claim's fraction is not in canonical form, or is not in the range
    /// its claim kind allows.
    #[error("claim {claim}: fraction is not in canonical form or out of range")]
    InvalidClaimFraction {
        /// The index of the claim in the claim list.
        claim: usize,
    },
    /// A claim kind that the certificate's version does not include.
    #[error("claim {claim} needs certificate version {needed}")]
    ClaimNeedsVersion {
        /// The index of the claim in the claim list.
        claim: usize,
        /// The lowest version that includes the claim kind.
        needed: u32,
    },
    /// A `FixedPoint` or `Contraction` claim that is false for the model.
    #[error("claim {claim} does not hold for the model")]
    FalseClaim {
        /// The index of the claim in the claim list.
        claim: usize,
    },
    /// A claim that only applies to two-state models, in a model with a
    /// different number of states.
    #[error("claim {claim} needs a two-state model, but the model has {states} states")]
    NotTwoState {
        /// The index of the claim in the claim list.
        claim: usize,
        /// The number of states in the model.
        states: usize,
    },
    /// The model has both or neither of `rows` and `retry`.
    #[error("model must have exactly one of rows and retry")]
    ModelForm,
    /// A retry model in a certificate whose version does not include it.
    #[error("retry model needs certificate version {needed}")]
    ModelNeedsVersion {
        /// The lowest version that includes the retry model.
        needed: u32,
    },
    /// The retry model's success probability is not a canonical probability.
    #[error("retry success is not a canonical probability")]
    InvalidRetrySuccess,
    /// A claim about a chain, in a certificate whose model is a retry loop.
    #[error("claim {claim} needs a chain model")]
    NotChain {
        /// The index of the claim in the claim list.
        claim: usize,
    },
    /// A `Termination` claim in a certificate whose model is a chain.
    #[error("claim {claim} needs a retry model")]
    NotRetry {
        /// The index of the claim in the claim list.
        claim: usize,
    },
    /// A step or attempt count above the format's limit.
    #[error("claim {claim}: count is above the limit {limit}")]
    CountOutOfRange {
        /// The index of the claim in the claim list.
        claim: usize,
        /// The largest count the format allows.
        limit: u64,
    },
    /// The engine cannot make the requested claim about the chain: it has no
    /// unique steady state, or does not come within the tolerance in the
    /// allowed number of steps.
    #[error("no convergence bound can be certified for the chain")]
    NoConvergenceBound(#[source] ConvergenceBoundError),
    /// A claim refers to a row the model does not have.
    #[error("claim {claim} refers to row {row}, but the model has {rows} rows")]
    RowOutOfRange {
        /// The index of the claim in the claim list.
        claim: usize,
        /// The row the claim refers to.
        row: u64,
        /// The number of rows in the model.
        rows: usize,
    },
}

impl From<serde_json::Error> for CertificateError {
    /// Keeps only the position of a JSON error. `serde_json` messages quote
    /// unknown field and variant names from the input, so the message itself
    /// is discarded.
    fn from(error: serde_json::Error) -> Self {
        Self::Json {
            line: error.line(),
            column: error.column(),
        }
    }
}

/// Why the engine could not certify a convergence bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ConvergenceBoundError {
    /// The chain has no unique steady state or does not contract.
    #[error(transparent)]
    Chain(#[from] TwoStateChainError),
    /// The chain does not come within the tolerance in at most the allowed
    /// number of steps.
    #[error("tolerance is not reached within the step limit")]
    NotReached,
}
