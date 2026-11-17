use axiom_core::{Probability, TransitionMatrix, TransitionRow};

use crate::CertificateError;
use crate::document::{CertificateDocument, ClaimDocument, FORMAT, ModelDocument, VERSION};
use crate::fraction_text::FractionText;

/// The largest certificate, in UTF-8 bytes, that [`Certificate::from_json`]
/// accepts.
pub const MAX_CERTIFICATE_BYTES: usize = 1024 * 1024;

/// A claim about the embedded model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Claim {
    /// Every entry of row `row` is a probability and the entries sum to
    /// exactly one.
    Mass {
        /// The index of the row the claim is about.
        row: usize,
    },
}

/// A validated certificate: a square transition matrix and a non-empty list
/// of distinct claims whose row references are all in range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    matrix: TransitionMatrix,
    claims: Vec<Claim>,
}

impl Certificate {
    /// A certificate with one [`Claim::Mass`] for each row, in row order.
    #[must_use]
    pub fn mass(matrix: TransitionMatrix) -> Self {
        let claims = (0..matrix.states())
            .map(|row| Claim::Mass { row })
            .collect();
        Self { matrix, claims }
    }

    /// The model's transition matrix.
    #[must_use]
    pub fn matrix(&self) -> &TransitionMatrix {
        &self.matrix
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
    /// Returns [`CertificateError::Json`] if `serde_json` reports a failure.
    /// The document holds only strings, integers and lists, which `serde_json`
    /// always serialises, so the error is returned instead of panicking on an
    /// outcome that does not occur.
    pub fn to_json(&self) -> Result<String, CertificateError> {
        let document = CertificateDocument {
            format: FORMAT.to_owned(),
            version: VERSION,
            model: ModelDocument {
                rows: self.matrix.rows().iter().map(row_text).collect(),
            },
            claims: self.claims.iter().map(claim_document).collect(),
        };
        let mut text = serde_json::to_string_pretty(&document)?;
        text.push('\n');
        Ok(text)
    }

    /// Reads and validates a version 1 certificate.
    ///
    /// Whitespace and the order of keys within an object do not matter; every
    /// value must be in its canonical form.
    ///
    /// # Errors
    ///
    /// Returns an error for text larger than [`MAX_CERTIFICATE_BYTES`],
    /// malformed JSON, unknown or repeated fields, a wrong `format` or
    /// `version`, a non-canonical fraction, an entry that is not a
    /// probability, a row that does not sum to one, rows that do not form a
    /// square matrix, no claims, a repeated claim, or a claim about a missing
    /// row.
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
        let matrix = TransitionMatrix::try_new(rows)?;
        let claims = parse_claims(&document.claims, matrix.states())?;
        Ok(Self { matrix, claims })
    }
}

fn row_text(row: &TransitionRow) -> Vec<FractionText> {
    row.probabilities()
        .iter()
        .map(|probability| FractionText::from_fraction(probability.value()))
        .collect()
}

fn claim_document(claim: &Claim) -> ClaimDocument {
    match claim {
        Claim::Mass { row } => ClaimDocument::Mass { row: *row },
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

fn parse_claims(documents: &[ClaimDocument], rows: usize) -> Result<Vec<Claim>, CertificateError> {
    if documents.is_empty() {
        return Err(CertificateError::NoClaims);
    }
    let mut claims: Vec<Claim> = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let claim = match *document {
            ClaimDocument::Mass { row } if row < rows => Claim::Mass { row },
            ClaimDocument::Mass { row } => {
                return Err(CertificateError::RowOutOfRange {
                    claim: index,
                    row,
                    rows,
                });
            }
        };
        if let Some(first) = claims.iter().position(|earlier| *earlier == claim) {
            return Err(CertificateError::DuplicateClaim {
                claim: index,
                first,
            });
        }
        claims.push(claim);
    }
    Ok(claims)
}
