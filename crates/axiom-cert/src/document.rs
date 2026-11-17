use serde::{Deserialize, Serialize};

use crate::fraction_text::FractionText;

/// The constant value of the `format` field.
pub(crate) const FORMAT: &str = "axiom-certificate";

/// The format version this crate writes and reads.
pub(crate) const VERSION: u32 = 1;

/// The JSON shape of a version 1 certificate, before validation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CertificateDocument {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) model: ModelDocument,
    pub(crate) claims: Vec<ClaimDocument>,
}

/// The embedded model: the transition matrix, one row per current state.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelDocument {
    pub(crate) rows: Vec<Vec<FractionText>>,
}

/// One claim, tagged by its `kind` field.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(crate) enum ClaimDocument {
    Mass { row: usize },
}
