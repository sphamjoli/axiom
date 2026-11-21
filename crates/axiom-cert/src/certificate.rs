use axiom_core::{
    Probability, TransitionMatrix, TransitionMatrixError, TransitionRow, TwoStateChain,
    TwoStateChainError,
};
use num_rational::BigRational;
use num_traits::{One, Signed};

use crate::CertificateError;
use crate::document::{
    CertificateDocument, ClaimDocument, FORMAT, ModelDocument, VERSION_1, VERSION_2,
};
use crate::fraction_text::{FractionText, Sign};

/// The largest certificate, in UTF-8 bytes, that [`Certificate::from_json`]
/// accepts.
pub const MAX_CERTIFICATE_BYTES: usize = 1024 * 1024;

/// The largest number of states a certificate's model may have.
///
/// With at most this many entries per row, each of at most 64 digits,
/// checking that a row sums to one takes milliseconds.
pub const MAX_STATES: usize = 16;

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
    /// The model has two states, and `value` is a fixed point of the update
    /// `d' = lambda d + c` of the probability `d` of the second state, where
    /// `a` is entry `[0][1]`, `b` is entry `[1][0]`, `lambda = 1 - a - b` and
    /// `c = a`. Introduced in version 2.
    FixedPoint {
        /// The claimed steady-state probability of the second state.
        value: Probability,
    },
    /// The model has two states, `factor` equals `lambda = 1 - a - b` of the
    /// same update, and `|factor| < 1`. Introduced in version 2.
    Contraction {
        /// The claimed contraction factor, which may be negative.
        factor: BigRational,
    },
}

impl Claim {
    /// The lowest certificate version that can express the claim.
    fn version(&self) -> u32 {
        match self {
            Self::Mass { .. } => VERSION_1,
            Self::FixedPoint { .. } | Self::Contraction { .. } => VERSION_2,
        }
    }
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

    /// A certificate for a two-state chain: one [`Claim::Mass`] for each row,
    /// then its [`Claim::FixedPoint`] and its [`Claim::Contraction`].
    ///
    /// # Errors
    ///
    /// Returns [`TwoStateChainError::NoUniqueFixedPoint`] when the chain never
    /// leaves either state, so no unique steady state exists, and
    /// [`TwoStateChainError::NotContracting`] when it always leaves both, so
    /// the factor is `-1` and the `Contraction` claim would be false.
    pub fn two_state(chain: &TwoStateChain) -> Result<Self, TwoStateChainError> {
        let value = chain.steady_state()?;
        if chain.contraction_factor().abs() >= BigRational::one() {
            return Err(TwoStateChainError::NotContracting);
        }
        let mut certificate = Self::mass(chain.matrix());
        certificate.claims.push(Claim::FixedPoint { value });
        certificate.claims.push(Claim::Contraction {
            factor: chain.contraction_factor(),
        });
        Ok(certificate)
    }

    /// The format version the certificate is written in: the lowest version
    /// that can express every claim, so a certificate of `Mass` claims only
    /// is written as version 1.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.claims
            .iter()
            .map(Claim::version)
            .max()
            .unwrap_or(VERSION_1)
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

    /// The certificate as canonical JSON in the version given by
    /// [`Certificate::version`], pretty-printed with a trailing newline.
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
            version: self.version(),
            model: ModelDocument {
                rows: self.matrix.rows().iter().map(row_text).collect(),
            },
            claims: self.claims.iter().map(claim_document).collect(),
        };
        let mut text = serde_json::to_string_pretty(&document)?;
        text.push('\n');
        Ok(text)
    }

    /// Reads and validates a version 1 or version 2 certificate.
    ///
    /// Whitespace and the order of keys within an object do not matter; every
    /// value must be in its canonical form. The size of the matrix is checked
    /// before any fraction is parsed or summed, so the work done is bounded by
    /// [`MAX_STATES`] and the 64-digit limit. Callers reading from a file or
    /// socket must cap the read at [`MAX_CERTIFICATE_BYTES`] plus one byte
    /// themselves, since this function only sees text already in memory.
    ///
    /// # Errors
    ///
    /// Returns an error for text larger than [`MAX_CERTIFICATE_BYTES`],
    /// malformed JSON, unknown or repeated fields, a wrong `format` or
    /// `version`, a non-canonical fraction, an entry that is not a
    /// probability, a row that does not sum to one, rows that do not form a
    /// square matrix, no claims, a repeated claim, or a claim about a missing
    /// row, a claim kind the version does not include, a two-state claim
    /// about a model without exactly two states, or a claim fraction that is
    /// not canonical or out of range, or a `FixedPoint` or `Contraction` claim
    /// that is false for the model.
    pub fn from_json(text: &str) -> Result<Self, CertificateError> {
        if text.len() > MAX_CERTIFICATE_BYTES {
            return Err(CertificateError::TooLarge {
                bytes: text.len(),
                limit: MAX_CERTIFICATE_BYTES,
            });
        }
        let document: CertificateDocument = serde_json::from_str(text)?;
        if document.format != FORMAT {
            return Err(CertificateError::UnsupportedFormat);
        }
        if document.version != VERSION_1 && document.version != VERSION_2 {
            return Err(CertificateError::UnsupportedVersion(document.version));
        }
        check_shape(&document.model.rows)?;
        let rows = document
            .model
            .rows
            .iter()
            .enumerate()
            .map(|(row, entries)| parse_row(row, entries))
            .collect::<Result<Vec<_>, _>>()?;
        let matrix = TransitionMatrix::try_new(rows)?;
        let claims = parse_claims(&document.claims, document.version, &matrix)?;
        Ok(Self { matrix, claims })
    }
}

/// Rejects a model with too many states or a ragged shape before any fraction
/// is parsed, so the cost of summing rows stays bounded.
fn check_shape(rows: &[Vec<FractionText>]) -> Result<(), CertificateError> {
    let states = rows.len();
    if states > MAX_STATES {
        return Err(CertificateError::TooManyStates {
            states,
            limit: MAX_STATES,
        });
    }
    if let Some((row, ragged)) = rows.iter().enumerate().find(|(_, row)| row.len() != states) {
        return Err(TransitionMatrixError::NotSquare {
            row,
            entries: ragged.len(),
            states,
        }
        .into());
    }
    Ok(())
}

fn row_text(row: &TransitionRow) -> Vec<FractionText> {
    row.probabilities()
        .iter()
        .map(|probability| FractionText::from_fraction(probability.value()))
        .collect()
}

fn claim_document(claim: &Claim) -> ClaimDocument {
    match claim {
        Claim::Mass { row } => ClaimDocument::Mass { row: *row as u64 },
        Claim::FixedPoint { value } => ClaimDocument::FixedPoint {
            value: FractionText::from_fraction(value.value()),
        },
        Claim::Contraction { factor } => ClaimDocument::Contraction {
            factor: FractionText::from_fraction(factor),
        },
    }
}

fn parse_row(row: usize, entries: &[FractionText]) -> Result<TransitionRow, CertificateError> {
    let probabilities = entries
        .iter()
        .enumerate()
        .map(|(entry, text)| {
            let value = text
                .to_fraction(Sign::Forbidden)
                .ok_or(CertificateError::NonCanonicalFraction { row, entry })?;
            Probability::try_new(&value).map_err(|source| CertificateError::Probability {
                row,
                entry,
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    TransitionRow::try_new(probabilities)
        .map_err(|source| CertificateError::TransitionRow { row, source })
}

fn parse_claims(
    documents: &[ClaimDocument],
    version: u32,
    matrix: &TransitionMatrix,
) -> Result<Vec<Claim>, CertificateError> {
    let rows = matrix.states();
    if documents.is_empty() {
        return Err(CertificateError::NoClaims);
    }
    let mut claims: Vec<Claim> = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let claim = match document {
            ClaimDocument::Mass { row } => match usize::try_from(*row) {
                Ok(index_in_range) if index_in_range < rows => Claim::Mass {
                    row: index_in_range,
                },
                _ => {
                    return Err(CertificateError::RowOutOfRange {
                        claim: index,
                        row: *row,
                        rows,
                    });
                }
            },
            ClaimDocument::FixedPoint { value } => {
                let chain = two_state_chain(index, version, matrix)?;
                let value = value
                    .to_fraction(Sign::Forbidden)
                    .and_then(|value| Probability::try_new(&value).ok())
                    .ok_or(CertificateError::InvalidClaimFraction { claim: index })?;
                if chain.second_state_update().apply(value.value()) != *value.value() {
                    return Err(CertificateError::FalseClaim { claim: index });
                }
                Claim::FixedPoint { value }
            }
            ClaimDocument::Contraction { factor } => {
                let chain = two_state_chain(index, version, matrix)?;
                let factor = factor
                    .to_fraction(Sign::Allowed)
                    .ok_or(CertificateError::InvalidClaimFraction { claim: index })?;
                if factor != chain.contraction_factor() || factor.abs() >= BigRational::one() {
                    return Err(CertificateError::FalseClaim { claim: index });
                }
                Claim::Contraction { factor }
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

/// The two-state chain of a version 2 certificate's model, for a claim that
/// only applies to two-state models. Its exit probabilities are
/// `a = rows[0][1]` and `b = rows[1][0]`.
fn two_state_chain(
    claim: usize,
    version: u32,
    matrix: &TransitionMatrix,
) -> Result<TwoStateChain, CertificateError> {
    if version < VERSION_2 {
        return Err(CertificateError::ClaimNeedsVersion {
            claim,
            needed: VERSION_2,
        });
    }
    let states = matrix.states();
    let entry = |row: usize, column: usize| {
        matrix
            .rows()
            .get(row)
            .and_then(|entries| entries.probabilities().get(column))
            .cloned()
    };
    match (states, entry(0, 1), entry(1, 0)) {
        (2, Some(leave_first), Some(leave_second)) => {
            Ok(TwoStateChain::new(leave_first, leave_second))
        }
        _ => Err(CertificateError::NotTwoState { claim, states }),
    }
}
