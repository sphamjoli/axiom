use axiom_core::{
    Probability, TransitionMatrix, TransitionMatrixError, TransitionRow, TwoStateChain,
    TwoStateChainError,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Pow, Signed, Zero};

use crate::document::{
    CertificateDocument, ClaimDocument, FORMAT, FailureAfterDocument, ModelDocument, RetryDocument,
    VERSION_1, VERSION_2, VERSION_3,
};
use crate::error::{CertificateError, ConvergenceBoundError};
use crate::fraction_text::{FractionText, Sign};

/// The largest certificate, in UTF-8 bytes, that [`Certificate::from_json`]
/// accepts.
pub const MAX_CERTIFICATE_BYTES: usize = 1024 * 1024;

/// The largest number of states a certificate's model may have.
///
/// With at most this many entries per row, each of at most 64 digits,
/// checking that a row sums to one takes milliseconds.
pub const MAX_STATES: usize = 16;

/// The largest step count of a `ConvergenceBound` claim and the largest
/// attempt count of a `Termination` claim's `failure_after`.
///
/// Checking such a claim raises a fraction with parts of at most 64 digits
/// to this power, so the result has at most about 64,000 digits and both
/// the engine and the checker compute it in well under a second; checking
/// is quadratic in the count, which is why the limit is not higher.
pub const MAX_COUNT: u64 = 1_000;

/// The model a certificate embeds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Model {
    /// A Markov chain, given by its transition matrix.
    Chain(TransitionMatrix),
    /// A retry loop whose attempts each succeed independently with
    /// probability `success`. Introduced in version 3.
    Retry {
        /// The probability that one attempt succeeds.
        success: Probability,
    },
}

/// The claim that each of the first `attempts` attempts of a retry loop
/// fails with probability `probability`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureAfter {
    /// The number of attempts, at most [`MAX_COUNT`].
    pub attempts: u64,
    /// The claimed probability `(1 - p)^attempts`.
    pub probability: Probability,
}

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
    /// The model has two states with `a + b != 0`, and
    /// `|lambda^steps (start - d*)| <= tolerance`, where `d* = a / (a + b)`
    /// is the steady state. Since `|lambda| <= 1`, every iterate of the
    /// update from `start`, from step `steps` on, is then within `tolerance`
    /// of `d*`. Introduced in version 3.
    ConvergenceBound {
        /// The starting probability of the second state.
        start: Probability,
        /// The number of steps, at most [`MAX_COUNT`].
        steps: u64,
        /// The tolerance, which is positive.
        tolerance: BigRational,
    },
    /// The model is a retry loop with success probability `p`, and
    /// `probability` is the probability that it eventually succeeds: zero
    /// when `p = 0`, one otherwise. When present, `expected_attempts` is
    /// `1 / p`, which needs `p > 0`, and `failure_after` gives the
    /// probability `(1 - p)^k` that the first `k` attempts all fail.
    /// Introduced in version 3.
    Termination {
        /// The probability of eventual success.
        probability: Probability,
        /// The expected number of attempts.
        expected_attempts: Option<BigRational>,
        /// The probability that a number of attempts all fail.
        failure_after: Option<FailureAfter>,
    },
}

impl Claim {
    /// The lowest certificate version that can express the claim.
    fn version(&self) -> u32 {
        match self {
            Self::Mass { .. } => VERSION_1,
            Self::FixedPoint { .. } | Self::Contraction { .. } => VERSION_2,
            Self::ConvergenceBound { .. } | Self::Termination { .. } => VERSION_3,
        }
    }
}

/// A validated certificate: a model and a non-empty list of distinct claims
/// that refer to it correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    model: Model,
    claims: Vec<Claim>,
}

impl Certificate {
    /// A certificate with one [`Claim::Mass`] for each row, in row order.
    #[must_use]
    pub fn mass(matrix: TransitionMatrix) -> Self {
        let claims = (0..matrix.states())
            .map(|row| Claim::Mass { row })
            .collect();
        Self {
            model: Model::Chain(matrix),
            claims,
        }
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

    /// The certificate with the most claims the engine can make about
    /// `matrix`. For a two-state chain with a unique, attracting steady
    /// state it is the [`Certificate::two_state_with_bound`] certificate from
    /// `start = 1` with `tolerance = 1/1000`, falling back to
    /// [`Certificate::two_state`] when no bound is reached within
    /// [`MAX_COUNT`] steps; otherwise the [`Certificate::mass`] certificate.
    /// For example, Alice's links chain gets the steady state 1/4, the
    /// contraction factor 3/5 and a 13-step bound, while the periodic chain
    /// that swaps its two states every step gets only `Mass` claims, because
    /// it never settles.
    #[must_use]
    pub fn strongest(matrix: TransitionMatrix) -> Self {
        if let [first, second] = matrix.rows()
            && let (Some(leave_first), Some(leave_second)) =
                (first.probabilities().get(1), second.probabilities().first())
        {
            let chain = TwoStateChain::new(leave_first.clone(), leave_second.clone());
            let tolerance = BigRational::new(BigInt::from(1), BigInt::from(1000));
            if let Ok(certificate) =
                Self::two_state_with_bound(&chain, &Probability::one(), &tolerance)
            {
                return certificate;
            }
            if let Ok(certificate) = Self::two_state(&chain) {
                return certificate;
            }
        }
        Self::mass(matrix)
    }

    /// The claims of [`Certificate::two_state`] followed by a
    /// [`Claim::ConvergenceBound`] from `start` with the least number of
    /// steps after which every iterate is within `tolerance` of the steady
    /// state. For the link model, `start = 1` and `tolerance = 1/1000` give
    /// 13 steps.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::NoConvergenceBound`] when the chain has no
    /// unique steady state or does not contract, when `tolerance` is not
    /// positive, or when more than [`MAX_COUNT`] steps would be needed.
    pub fn two_state_with_bound(
        chain: &TwoStateChain,
        start: &Probability,
        tolerance: &BigRational,
    ) -> Result<Self, CertificateError> {
        let chain_error =
            |error| CertificateError::NoConvergenceBound(ConvergenceBoundError::Chain(error));
        let mut certificate = Self::two_state(chain).map_err(chain_error)?;
        if !tolerance.is_positive() {
            return Err(CertificateError::NoConvergenceBound(
                ConvergenceBoundError::NotReached,
            ));
        }
        let steps = chain
            .steps_within(start.value(), tolerance, MAX_COUNT)
            .map_err(chain_error)?
            .ok_or(CertificateError::NoConvergenceBound(
                ConvergenceBoundError::NotReached,
            ))?;
        certificate.claims.push(Claim::ConvergenceBound {
            start: start.clone(),
            steps,
            tolerance: tolerance.clone(),
        });
        Ok(certificate)
    }

    /// A certificate for a retry loop whose attempts succeed with probability
    /// `success`: one [`Claim::Termination`] with the probability of eventual
    /// success, the expected number of attempts `1 / success` when `success`
    /// is positive, and the probability that the first `failure_attempts`
    /// attempts all fail.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::CountOutOfRange`] when `failure_attempts`
    /// is above [`MAX_COUNT`].
    pub fn termination(
        success: Probability,
        failure_attempts: u64,
    ) -> Result<Self, CertificateError> {
        if failure_attempts > MAX_COUNT {
            return Err(CertificateError::CountOutOfRange {
                claim: 0,
                limit: MAX_COUNT,
            });
        }
        let probability = success_probability(&success);
        let expected_attempts = (!success.value().is_zero()).then(|| success.value().recip());
        let failure = success.complement().pow(failure_attempts);
        Ok(Self {
            model: Model::Retry { success },
            claims: vec![Claim::Termination {
                probability,
                expected_attempts,
                failure_after: Some(FailureAfter {
                    attempts: failure_attempts,
                    probability: failure,
                }),
            }],
        })
    }

    /// The format version the certificate is written in: the lowest version
    /// that can express the model and every claim, so a certificate of `Mass`
    /// claims only is written as version 1.
    #[must_use]
    pub fn version(&self) -> u32 {
        let model = match self.model {
            Model::Chain(_) => VERSION_1,
            Model::Retry { .. } => VERSION_3,
        };
        self.claims.iter().map(Claim::version).fold(model, u32::max)
    }

    /// The embedded model.
    #[must_use]
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The model's transition matrix, or `None` for a retry model.
    #[must_use]
    pub fn matrix(&self) -> Option<&TransitionMatrix> {
        match &self.model {
            Model::Chain(matrix) => Some(matrix),
            Model::Retry { .. } => None,
        }
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
        let model = match &self.model {
            Model::Chain(matrix) => ModelDocument {
                rows: Some(matrix.rows().iter().map(row_text).collect()),
                retry: None,
            },
            Model::Retry { success } => ModelDocument {
                rows: None,
                retry: Some(RetryDocument {
                    success: FractionText::from_fraction(success.value()),
                }),
            },
        };
        let document = CertificateDocument {
            format: FORMAT.to_owned(),
            version: self.version(),
            model,
            claims: self.claims.iter().map(claim_document).collect(),
        };
        let mut text = serde_json::to_string_pretty(&document)?;
        text.push('\n');
        Ok(text)
    }

    /// Reads and validates a version 1, 2 or 3 certificate.
    ///
    /// Whitespace and the order of keys within an object do not matter; every
    /// value must be in its canonical form. The size of the matrix is checked
    /// before any fraction is parsed or summed, and step and attempt counts
    /// are capped at [`MAX_COUNT`], so the work done is bounded by
    /// [`MAX_STATES`], [`MAX_COUNT`] and the 64-digit limit. Callers reading
    /// from a file or socket must cap the read at [`MAX_CERTIFICATE_BYTES`]
    /// plus one byte themselves, since this function only sees text already
    /// in memory.
    ///
    /// # Errors
    ///
    /// Returns an error for text larger than [`MAX_CERTIFICATE_BYTES`],
    /// malformed JSON, unknown or repeated fields, a wrong `format` or
    /// `version`, a model with both or neither of `rows` and `retry`, a retry
    /// model before version 3, a non-canonical fraction, an entry that is not
    /// a probability, a row that does not sum to one, rows that do not form a
    /// square matrix, no claims, a repeated claim, a claim about a missing
    /// row, a claim kind the version does not include, a claim about the
    /// wrong kind of model, a two-state claim about a model without exactly
    /// two states, a count above [`MAX_COUNT`], a claim fraction that is not
    /// canonical or out of range, or a claim other than `Mass` that is false
    /// for the model.
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
        if !(VERSION_1..=VERSION_3).contains(&document.version) {
            return Err(CertificateError::UnsupportedVersion(document.version));
        }
        let model = match (document.model.rows, document.model.retry) {
            (Some(rows), None) => Model::Chain(parse_matrix(&rows)?),
            (None, Some(retry)) => {
                if document.version < VERSION_3 {
                    return Err(CertificateError::ModelNeedsVersion { needed: VERSION_3 });
                }
                let success = retry
                    .success
                    .to_fraction(Sign::Forbidden)
                    .and_then(|value| Probability::try_new(&value).ok())
                    .ok_or(CertificateError::InvalidRetrySuccess)?;
                Model::Retry { success }
            }
            _ => return Err(CertificateError::ModelForm),
        };
        let claims = parse_claims(&document.claims, document.version, &model)?;
        Ok(Self { model, claims })
    }
}

/// The probability that a retry loop with success probability `success`
/// eventually succeeds: zero when `success` is zero, one otherwise.
fn success_probability(success: &Probability) -> Probability {
    if success.value().is_zero() {
        success.clone()
    } else {
        Probability::one()
    }
}

/// Parses and validates the transition matrix of a chain model.
fn parse_matrix(rows: &[Vec<FractionText>]) -> Result<TransitionMatrix, CertificateError> {
    check_shape(rows)?;
    let rows = rows
        .iter()
        .enumerate()
        .map(|(row, entries)| parse_row(row, entries))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TransitionMatrix::try_new(rows)?)
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
        Claim::ConvergenceBound {
            start,
            steps,
            tolerance,
        } => ClaimDocument::ConvergenceBound {
            start: FractionText::from_fraction(start.value()),
            steps: *steps,
            tolerance: FractionText::from_fraction(tolerance),
        },
        Claim::Termination {
            probability,
            expected_attempts,
            failure_after,
        } => ClaimDocument::Termination {
            probability: FractionText::from_fraction(probability.value()),
            expected_attempts: expected_attempts.as_ref().map(FractionText::from_fraction),
            failure_after: failure_after.as_ref().map(|failure| FailureAfterDocument {
                attempts: failure.attempts,
                probability: FractionText::from_fraction(failure.probability.value()),
            }),
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
    model: &Model,
) -> Result<Vec<Claim>, CertificateError> {
    if documents.is_empty() {
        return Err(CertificateError::NoClaims);
    }
    let mut claims: Vec<Claim> = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let claim = parse_claim(index, document, version, model)?;
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

/// Parses claim `index` and, for every kind but `Mass`, checks that it holds
/// for the model. `Mass` claims hold because the matrix has been validated.
fn parse_claim(
    index: usize,
    document: &ClaimDocument,
    version: u32,
    model: &Model,
) -> Result<Claim, CertificateError> {
    let invalid_fraction = CertificateError::InvalidClaimFraction { claim: index };
    let false_claim = CertificateError::FalseClaim { claim: index };
    let probability = |text: &FractionText| claim_probability(index, text);
    let count = |value: u64| claim_count(index, value);
    match document {
        ClaimDocument::Mass { row } => {
            let Model::Chain(matrix) = model else {
                return Err(CertificateError::NotChain { claim: index });
            };
            let rows = matrix.states();
            match usize::try_from(*row) {
                Ok(in_range) if in_range < rows => Ok(Claim::Mass { row: in_range }),
                _ => Err(CertificateError::RowOutOfRange {
                    claim: index,
                    row: *row,
                    rows,
                }),
            }
        }
        ClaimDocument::FixedPoint { value } => {
            let chain = two_state_chain(index, version, VERSION_2, model)?;
            let value = probability(value)?;
            if chain.second_state_update().apply(value.value()) != *value.value() {
                return Err(false_claim);
            }
            Ok(Claim::FixedPoint { value })
        }
        ClaimDocument::Contraction { factor } => {
            let chain = two_state_chain(index, version, VERSION_2, model)?;
            let factor = factor.to_fraction(Sign::Allowed).ok_or(invalid_fraction)?;
            if factor != chain.contraction_factor() || factor.abs() >= BigRational::one() {
                return Err(false_claim);
            }
            Ok(Claim::Contraction { factor })
        }
        ClaimDocument::ConvergenceBound {
            start,
            steps,
            tolerance,
        } => {
            let chain = two_state_chain(index, version, VERSION_3, model)?;
            let start = probability(start)?;
            let steps = count(*steps)?;
            let tolerance = tolerance
                .to_fraction(Sign::Forbidden)
                .filter(Signed::is_positive)
                .ok_or(invalid_fraction)?;
            let steady = chain.steady_state().map_err(|_| false_claim.clone())?;
            let factor = chain.contraction_factor().abs();
            let distance = (start.value() - steady.value()).abs();
            if factor > BigRational::one() || !bound_holds(&factor, steps, &distance, &tolerance) {
                return Err(false_claim);
            }
            Ok(Claim::ConvergenceBound {
                start,
                steps,
                tolerance,
            })
        }
        ClaimDocument::Termination {
            probability: claimed,
            expected_attempts,
            failure_after,
        } => parse_termination(
            index,
            version,
            model,
            claimed,
            expected_attempts.as_ref(),
            failure_after.as_ref(),
        ),
    }
}

/// The probability written in a claim, or an error naming the claim.
fn claim_probability(index: usize, text: &FractionText) -> Result<Probability, CertificateError> {
    text.to_fraction(Sign::Forbidden)
        .and_then(|value| Probability::try_new(&value).ok())
        .ok_or(CertificateError::InvalidClaimFraction { claim: index })
}

/// A step or attempt count written in a claim, if it is within
/// [`MAX_COUNT`].
fn claim_count(index: usize, value: u64) -> Result<u64, CertificateError> {
    if value > MAX_COUNT {
        Err(CertificateError::CountOutOfRange {
            claim: index,
            limit: MAX_COUNT,
        })
    } else {
        Ok(value)
    }
}

/// Parses a `Termination` claim and checks that it holds for the model.
fn parse_termination(
    index: usize,
    version: u32,
    model: &Model,
    claimed: &FractionText,
    expected_attempts: Option<&FractionText>,
    failure_after: Option<&FailureAfterDocument>,
) -> Result<Claim, CertificateError> {
    if version < VERSION_3 {
        return Err(CertificateError::ClaimNeedsVersion {
            claim: index,
            needed: VERSION_3,
        });
    }
    let Model::Retry { success } = model else {
        return Err(CertificateError::NotRetry { claim: index });
    };
    let claimed = claim_probability(index, claimed)?;
    let expected_attempts = expected_attempts
        .map(|text| {
            text.to_fraction(Sign::Forbidden)
                .ok_or(CertificateError::InvalidClaimFraction { claim: index })
        })
        .transpose()?;
    let failure_after = failure_after
        .map(|document| {
            Ok::<_, CertificateError>(FailureAfter {
                attempts: claim_count(index, document.attempts)?,
                probability: claim_probability(index, &document.probability)?,
            })
        })
        .transpose()?;
    let claim = Claim::Termination {
        probability: claimed,
        expected_attempts,
        failure_after,
    };
    if termination_holds(success, &claim) {
        Ok(claim)
    } else {
        Err(CertificateError::FalseClaim { claim: index })
    }
}

/// Whether `factor^steps * distance <= tolerance` for non-negative values,
/// compared as `p^n r w <= v q^n s` on integers so that no fraction with
/// hundreds of thousands of digits is ever reduced.
fn bound_holds(
    factor: &BigRational,
    steps: u64,
    distance: &BigRational,
    tolerance: &BigRational,
) -> bool {
    let left = Pow::pow(factor.numer(), steps) * distance.numer() * tolerance.denom();
    let right = Pow::pow(factor.denom(), steps) * tolerance.numer() * distance.denom();
    left <= right
}

/// Whether a `Termination` claim holds for a retry loop with success
/// probability `success`.
fn termination_holds(success: &Probability, claim: &Claim) -> bool {
    let Claim::Termination {
        probability,
        expected_attempts,
        failure_after,
    } = claim
    else {
        return false;
    };
    let expected_holds = expected_attempts
        .as_ref()
        .is_none_or(|expected| !success.value().is_zero() && *expected == success.value().recip());
    let failure_holds = failure_after
        .as_ref()
        .is_none_or(|failure| success.complement().pow(failure.attempts) == failure.probability);
    *probability == success_probability(success) && expected_holds && failure_holds
}

/// The two-state chain of a certificate's model, for a claim that needs
/// version `needed` and only applies to two-state chains. Its exit
/// probabilities are `a = rows[0][1]` and `b = rows[1][0]`.
fn two_state_chain(
    claim: usize,
    version: u32,
    needed: u32,
    model: &Model,
) -> Result<TwoStateChain, CertificateError> {
    if version < needed {
        return Err(CertificateError::ClaimNeedsVersion { claim, needed });
    }
    let Model::Chain(matrix) = model else {
        return Err(CertificateError::NotChain { claim });
    };
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
