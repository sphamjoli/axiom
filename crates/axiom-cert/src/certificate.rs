use axiom_core::{Probability, TransitionRow};

use crate::CertificateError;
use crate::document::{CertificateDocument, ClaimDocument, FORMAT, ModelDocument, VERSION};
use crate::fraction_text::FractionText;

/// The largest certificate text, in bytes, that [`Certificate::from_json`]
/// accepts.
pub const MAX_CERTIFICATE_BYTES: usize = 1024 * 1024;

/// A claim about the embedded model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Claim {
    /// Every entry of row `row` is a probability and the entries sum to
    /// exactly one.
    Mass {
        /// The index of the row the claim is about.
        row: usize,
    },
}

/// A validated certificate: a model's transition matrix and claims whose row
/// references are all in range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    rows: Vec<TransitionRow>,
    claims: Vec<Claim>,
}

impl Certificate {
    /// A certificate with one [`Claim::Mass`] for each row, in row order.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::EmptyModel`] when `rows` is empty.
    pub fn mass(rows: Vec<TransitionRow>) -> Result<Self, CertificateError> {
        if rows.is_empty() {
            return Err(CertificateError::EmptyModel);
        }
        let claims = (0..rows.len()).map(|row| Claim::Mass { row }).collect();
        Ok(Self { rows, claims })
    }

    /// The model's transition matrix, one row per current state.
    #[must_use]
    pub fn rows(&self) -> &[TransitionRow] {
        &self.rows
    }

    /// The claims, in the order they appear in the certificate.
    #[must_use]
    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    /// The certificate as canonical version 1 JSON, pretty-printed with a
    /// trailing newline.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::Json`] if serialisation fails.
    pub fn to_json(&self) -> Result<String, CertificateError> {
        let document = CertificateDocument {
            format: FORMAT.to_owned(),
            version: VERSION,
            model: ModelDocument {
                rows: self.rows.iter().map(row_text).collect(),
            },
            claims: self.claims.iter().copied().map(claim_document).collect(),
        };
        let mut text = serde_json::to_string_pretty(&document)?;
        text.push('\n');
        Ok(text)
    }

    /// Reads and validates a version 1 certificate.
    ///
    /// # Errors
    ///
    /// Returns an error for text larger than [`MAX_CERTIFICATE_BYTES`],
    /// malformed JSON or unknown fields, a wrong `format` or `version`, a
    /// non-canonical fraction, an entry that is not a probability, a row that
    /// does not sum to one, an empty model, or a claim that refers to a
    /// missing row.
    pub fn from_json(text: &str) -> Result<Self, CertificateError> {
        if text.len() > MAX_CERTIFICATE_BYTES {
            return Err(CertificateError::TooLarge {
                bytes: text.len(),
                limit: MAX_CERTIFICATE_BYTES,
            });
        }
        let document: CertificateDocument = serde_json::from_str(text)?;
        if document.format != FORMAT {
            return Err(CertificateError::UnsupportedFormat(document.format));
        }
        if document.version != VERSION {
            return Err(CertificateError::UnsupportedVersion(document.version));
        }
        let rows = document
            .model
            .rows
            .iter()
            .enumerate()
            .map(|(row, entries)| parse_row(row, entries))
            .collect::<Result<Vec<_>, _>>()?;
        if rows.is_empty() {
            return Err(CertificateError::EmptyModel);
        }
        let claims = document
            .claims
            .iter()
            .enumerate()
            .map(|(claim, document)| parse_claim(claim, document, rows.len()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { rows, claims })
    }
}

fn row_text(row: &TransitionRow) -> Vec<FractionText> {
    row.probabilities()
        .iter()
        .map(|probability| FractionText::from_fraction(probability.value()))
        .collect()
}

fn claim_document(claim: Claim) -> ClaimDocument {
    match claim {
        Claim::Mass { row } => ClaimDocument::Mass { row },
    }
}

fn parse_row(row: usize, entries: &[FractionText]) -> Result<TransitionRow, CertificateError> {
    let probabilities = entries
        .iter()
        .enumerate()
        .map(|(entry, text)| {
            Probability::try_new(&text.to_fraction()?)
                .map_err(|source| CertificateError::Probability { row, entry, source })
        })
        .collect::<Result<Vec<_>, _>>()?;
    TransitionRow::try_new(probabilities)
        .map_err(|source| CertificateError::TransitionRow { row, source })
}

fn parse_claim(
    claim: usize,
    document: &ClaimDocument,
    rows: usize,
) -> Result<Claim, CertificateError> {
    match *document {
        ClaimDocument::Mass { row } if row < rows => Ok(Claim::Mass { row }),
        ClaimDocument::Mass { row } => Err(CertificateError::RowOutOfRange { claim, row, rows }),
    }
}
