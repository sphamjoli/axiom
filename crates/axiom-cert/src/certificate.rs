use axiom_core::{
    Probability, TransitionMatrix, TransitionMatrixError, TransitionRow, TwoStateChain,
    TwoStateChainError,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Pow, Signed, Zero};

use crate::document::{
    CertificateDocument, ClaimDocument, EndDocument, EstimateDocument, FORMAT,
    FailureAfterDocument, IntervalDocument, ModelDocument, ParametricDocument, QuantityDocument,
    RetryDocument, VERSION_1, VERSION_2, VERSION_3, VERSION_4,
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

/// The largest sample count of an `Estimate` claim. It bounds the work of
/// reproducing the estimate from its seed; checking the count costs the
/// same for every count, because the Hoeffding bound clamps its argument.
pub const MAX_SAMPLES: u64 = 10_000_000;

/// The deepest nesting of arrays and objects a certificate may have.
pub const MAX_JSON_DEPTH: usize = 8;

/// The most members a JSON object in a certificate may have.
pub const MAX_JSON_MEMBERS: usize = 8;

/// The most items a JSON array in a certificate may have.
pub const MAX_JSON_ITEMS: usize = 64;

/// The model a certificate embeds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[expect(
    clippy::large_enum_variant,
    reason = "a certificate holds one model, so the larger size is paid once"
)]
pub enum Model {
    /// A Markov chain, given by its transition matrix.
    Chain(TransitionMatrix),
    /// A retry loop whose attempts each succeed independently with
    /// probability `success`. Introduced in version 3.
    Retry {
        /// The probability that one attempt succeeds.
        success: Probability,
    },
    /// The two-state chain `[[1 - a, a], [b, 1 - b]]` for every `a` in the
    /// interval `a` and every `b` in the interval `b`: `a` is the
    /// probability of leaving the first state and `b` that of leaving the
    /// second. Introduced in version 4.
    Parametric {
        /// The range of the probability of leaving the first state.
        a: Interval,
        /// The range of the probability of leaving the second state.
        b: Interval,
    },
}

/// Whether an end of an [`Interval`] belongs to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The end value is excluded, as `0` is from `]0, 1]`.
    Open,
    /// The end value is included, as `1` is in `]0, 1]`.
    Closed,
}

/// A non-empty interval of probabilities, each end open or closed, such as
/// `]0, 1[`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interval {
    lo: Probability,
    lo_end: End,
    hi: Probability,
    hi_end: End,
}

impl Interval {
    /// The interval from `lo` to `hi` with the given ends.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::EmptyInterval`] when no probability lies
    /// in it: `lo > hi`, or `lo = hi` with an open end. An empty range would
    /// make every claim about it vacuously true.
    pub fn try_new(
        lo: Probability,
        lo_end: End,
        hi: Probability,
        hi_end: End,
    ) -> Result<Self, CertificateError> {
        let non_empty =
            lo.value() < hi.value() || (lo == hi && lo_end == End::Closed && hi_end == End::Closed);
        if non_empty {
            Ok(Self {
                lo,
                lo_end,
                hi,
                hi_end,
            })
        } else {
            Err(CertificateError::EmptyInterval)
        }
    }

    /// The interval from `lo` to `hi`, each end closed when its flag is
    /// set: the form a model's parameter bounds take.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::InvalidInterval`] when an end is not a
    /// probability, and [`CertificateError::EmptyInterval`] as
    /// [`Interval::try_new`] does.
    pub fn from_bounds(
        lo: &BigRational,
        lo_closed: bool,
        hi: &BigRational,
        hi_closed: bool,
    ) -> Result<Self, CertificateError> {
        let probability = |value: &BigRational| {
            Probability::try_new(value).map_err(|_| CertificateError::InvalidInterval)
        };
        let end = |closed: bool| if closed { End::Closed } else { End::Open };
        Self::try_new(
            probability(lo)?,
            end(lo_closed),
            probability(hi)?,
            end(hi_closed),
        )
    }

    /// The lower end's value.
    #[must_use]
    pub fn lo(&self) -> &Probability {
        &self.lo
    }

    /// Whether the lower end belongs to the interval.
    #[must_use]
    pub fn lo_end(&self) -> End {
        self.lo_end
    }

    /// The upper end's value.
    #[must_use]
    pub fn hi(&self) -> &Probability {
        &self.hi
    }

    /// Whether the upper end belongs to the interval.
    #[must_use]
    pub fn hi_end(&self) -> End {
        self.hi_end
    }
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

/// The quantity an [`Estimate`] estimates. Each sample is `0` or `1`, as
/// Hoeffding's inequality requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// For a two-state chain, the probability of being in the second state
    /// after a fixed number of steps from the first, which approximates the
    /// steady state. Written `steady-state`.
    SteadyState,
    /// For a retry loop, the probability of success within a fixed number of
    /// attempts. Written `success`.
    Success,
}

/// An estimate of a [`Quantity`] from `samples` samples drawn with `seed`.
///
/// What a checked `Estimate` claim guarantees is only that `samples`
/// satisfies Hoeffding's bound for `epsilon` and `delta`: with probability
/// at least `1 - delta` over the sampling, the sample mean is within
/// `epsilon` of the true mean. Nothing checks that `value` is that sample
/// mean, so an estimate is always estimated, never proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate {
    /// What is estimated.
    pub quantity: Quantity,
    /// The estimated value, the sample mean.
    pub value: Probability,
    /// The number of samples, at most [`MAX_SAMPLES`].
    pub samples: u64,
    /// The tolerance, strictly between zero and one.
    pub epsilon: BigRational,
    /// The failure probability, strictly between zero and one.
    pub delta: BigRational,
    /// The seed the samples were drawn with.
    pub seed: u64,
}

impl Estimate {
    /// Checks a sample count against [`MAX_SAMPLES`] before any sampling,
    /// so a caller can refuse a count it would take too long to draw.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::SamplesOutOfRange`] for claim `claim`
    /// when `samples` is above [`MAX_SAMPLES`].
    pub fn check_samples(claim: usize, samples: u64) -> Result<(), CertificateError> {
        if samples > MAX_SAMPLES {
            Err(CertificateError::SamplesOutOfRange {
                claim,
                limit: MAX_SAMPLES,
            })
        } else {
            Ok(())
        }
    }
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
    /// The model has two states with `a + b != 0`, and `value` is the fixed
    /// point of the update `d' = lambda d + c` of the probability `d` of the second state, where
    /// `a` is entry `[0][1]`, `b` is entry `[1][0]`, `lambda = 1 - a - b` and
    /// `c = a`, which is then unique: the steady state `a / (a + b)`.
    /// Introduced in version 2.
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
    /// The model is parametric, and for every `a` and `b` in range
    /// `0 < a + b < 2`. By `TwoState.parametric` in the checker's theorems,
    /// each such chain then has the unique steady state `a / (a + b)` and
    /// contracts with factor `|1 - a - b| < 1`. Introduced in version 4.
    ParametricContraction,
    /// An estimate whose sample count satisfies Hoeffding's bound; see
    /// [`Estimate`]. It is the only claim whose certainty is estimated.
    /// Introduced in version 4.
    Estimate(Estimate),
}

impl Claim {
    /// The lowest certificate version that can express the claim.
    fn version(&self) -> u32 {
        match self {
            Self::Mass { .. } => VERSION_1,
            Self::FixedPoint { .. } | Self::Contraction { .. } => VERSION_2,
            Self::ConvergenceBound { .. } | Self::Termination { .. } => VERSION_3,
            Self::ParametricContraction | Self::Estimate(_) => VERSION_4,
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
    /// positive, when `start` or `tolerance` has a part longer than the
    /// format's 64 digits, or when more than [`MAX_COUNT`] steps would be needed.
    pub fn two_state_with_bound(
        chain: &TwoStateChain,
        start: &Probability,
        tolerance: &BigRational,
    ) -> Result<Self, CertificateError> {
        let chain_error =
            |error| CertificateError::NoConvergenceBound(ConvergenceBoundError::Chain(error));
        let mut certificate = Self::two_state(chain).map_err(chain_error)?;
        if !tolerance.is_positive()
            || !FractionText::fits(tolerance)
            || !FractionText::fits(start.value())
        {
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
    /// attempts all fail when that probability fits the format's 64-digit
    /// limit; otherwise the claim has no `failure_after`.
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
        let failure_after = FractionText::fits(failure.value()).then_some(FailureAfter {
            attempts: failure_attempts,
            probability: failure,
        });
        Ok(Self {
            model: Model::Retry { success },
            claims: vec![Claim::Termination {
                probability,
                expected_attempts,
                failure_after,
            }],
        })
    }

    /// A certificate for the parametric two-state chain with `a` and `b` in
    /// their intervals: one [`Claim::ParametricContraction`].
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::NoParametricContraction`] when the range
    /// includes a chain with `a + b = 0`, which never moves and so has no
    /// unique steady state, or with `a + b = 2`, which swaps its states every
    /// step and so never settles.
    pub fn parametric(a: Interval, b: Interval) -> Result<Self, CertificateError> {
        if !parametric_contraction_holds(&a, &b) {
            return Err(CertificateError::NoParametricContraction);
        }
        Ok(Self {
            model: Model::Parametric { a, b },
            claims: vec![Claim::ParametricContraction],
        })
    }

    /// The certificate with `estimate` appended as its last claim.
    ///
    /// # Errors
    ///
    /// Returns the error the reader would return for the claim: a quantity
    /// about the wrong kind of model, a sample count above [`MAX_SAMPLES`],
    /// `epsilon` or `delta` not strictly between zero and one or longer than
    /// the format's 64 digits, or a sample count that does not meet the
    /// Hoeffding bound.
    pub fn with_estimate(mut self, estimate: Estimate) -> Result<Self, CertificateError> {
        let index = self.claims.len();
        check_estimate(index, &estimate, &self.model)?;
        self.claims.push(Claim::Estimate(estimate));
        Ok(self)
    }

    /// The format version the certificate is written in: the lowest version
    /// that can express the model and every claim, so a certificate of `Mass`
    /// claims only is written as version 1.
    #[must_use]
    pub fn version(&self) -> u32 {
        let model = match self.model {
            Model::Chain(_) => VERSION_1,
            Model::Retry { .. } => VERSION_3,
            Model::Parametric { .. } => VERSION_4,
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
            Model::Retry { .. } | Model::Parametric { .. } => None,
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
                parametric: None,
            },
            Model::Retry { success } => ModelDocument {
                rows: None,
                retry: Some(RetryDocument {
                    success: FractionText::from_fraction(success.value()),
                }),
                parametric: None,
            },
            Model::Parametric { a, b } => ModelDocument {
                rows: None,
                retry: None,
                parametric: Some(ParametricDocument {
                    a: interval_document(a),
                    b: interval_document(b),
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

    /// Reads and validates a version 1, 2, 3 or 4 certificate.
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
    /// `version`, a model without exactly one of `rows`, `retry` and
    /// `parametric`, a retry model before version 3, a parametric model
    /// before version 4, an empty or non-canonical interval, a
    /// non-canonical fraction, an entry that is not
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
        check_json_limits(&serde_json::from_str(text)?, 1)?;
        let document: CertificateDocument = serde_json::from_str(text)?;
        if document.format != FORMAT {
            return Err(CertificateError::UnsupportedFormat);
        }
        if !(VERSION_1..=VERSION_4).contains(&document.version) {
            return Err(CertificateError::UnsupportedVersion(document.version));
        }
        let model = match (
            document.model.rows,
            document.model.retry,
            document.model.parametric,
        ) {
            (Some(rows), None, None) => Model::Chain(parse_matrix(&rows)?),
            (None, None, Some(parametric)) => {
                if document.version < VERSION_4 {
                    return Err(CertificateError::ModelNeedsVersion { needed: VERSION_4 });
                }
                Model::Parametric {
                    a: parse_interval(&parametric.a)?,
                    b: parse_interval(&parametric.b)?,
                }
            }
            (None, Some(retry), None) => {
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

/// Whether `0 < a + b < 2` for every `a` and `b` in their intervals. The sum
/// is affine, so its infimum over the box is the sum of the lower ends,
/// reached only when both are closed, and its supremum the sum of the upper
/// ends, reached only when both are closed. Both intervals lie in `[0, 1]`
/// because their ends are probabilities. The Lean checker's
/// `checkParametric` decides the same.
fn parametric_contraction_holds(a: &Interval, b: &Interval) -> bool {
    let lo_sum = a.lo.value() + b.lo.value();
    let hi_sum = a.hi.value() + b.hi.value();
    let two = BigRational::from_integer(BigInt::from(2));
    let either_open = |first: End, second: End| first == End::Open || second == End::Open;
    let above_zero = lo_sum.is_positive() || either_open(a.lo_end, b.lo_end);
    let below_two = hi_sum < two || either_open(a.hi_end, b.hi_end);
    above_zero && below_two
}

/// Checks estimate claim `index` against the model, the limits and the
/// Hoeffding bound, in the order the checker's reader and check apply them:
/// the quantity's model, the sample count, the range of `epsilon` and
/// `delta`, then the bound. `axiom_core::hoeffding_holds` is the decision
/// the Lean checker's `checkHoeffding` makes.
fn check_estimate(
    index: usize,
    estimate: &Estimate,
    model: &Model,
) -> Result<(), CertificateError> {
    match (estimate.quantity, model) {
        (Quantity::SteadyState, _) => {
            two_state_chain(index, VERSION_4, VERSION_4, model)?;
        }
        (Quantity::Success, Model::Retry { .. }) => {}
        (Quantity::Success, _) => return Err(CertificateError::NotRetry { claim: index }),
    }
    Estimate::check_samples(index, estimate.samples)?;
    let open_unit = |value: &BigRational| {
        value.is_positive() && *value < BigRational::one() && FractionText::fits(value)
    };
    if !open_unit(&estimate.epsilon) || !open_unit(&estimate.delta) {
        return Err(CertificateError::InvalidClaimFraction { claim: index });
    }
    if !FractionText::fits(estimate.value.value()) {
        return Err(CertificateError::InvalidClaimFraction { claim: index });
    }
    if axiom_core::hoeffding_holds(estimate.samples, &estimate.epsilon, &estimate.delta) {
        Ok(())
    } else {
        Err(CertificateError::FalseClaim { claim: index })
    }
}

/// The document form of an interval.
fn interval_document(interval: &Interval) -> IntervalDocument {
    let end = |end: End| match end {
        End::Open => EndDocument::Open,
        End::Closed => EndDocument::Closed,
    };
    IntervalDocument {
        lo: FractionText::from_fraction(interval.lo.value()),
        hi: FractionText::from_fraction(interval.hi.value()),
        lo_end: end(interval.lo_end),
        hi_end: end(interval.hi_end),
    }
}

/// Parses an interval: canonical probabilities for both ends, and not empty.
fn parse_interval(document: &IntervalDocument) -> Result<Interval, CertificateError> {
    let probability = |text: &FractionText| {
        text.to_fraction(Sign::Forbidden)
            .and_then(|value| Probability::try_new(&value).ok())
            .ok_or(CertificateError::InvalidInterval)
    };
    let end = |end: EndDocument| match end {
        EndDocument::Open => End::Open,
        EndDocument::Closed => End::Closed,
    };
    Interval::try_new(
        probability(&document.lo)?,
        end(document.lo_end),
        probability(&document.hi)?,
        end(document.hi_end),
    )
}

/// Rejects JSON nested deeper than [`MAX_JSON_DEPTH`], an object with more
/// than [`MAX_JSON_MEMBERS`] members or an array with more than
/// [`MAX_JSON_ITEMS`] items, the same limits the checker's reader enforces.
/// `depth` is the nesting level of `value` if it is an array or object.
fn check_json_limits(value: &serde_json::Value, depth: usize) -> Result<(), CertificateError> {
    let children: Vec<&serde_json::Value> = match value {
        serde_json::Value::Array(items) if items.len() > MAX_JSON_ITEMS => {
            return Err(CertificateError::JsonLimit);
        }
        serde_json::Value::Object(members) if members.len() > MAX_JSON_MEMBERS => {
            return Err(CertificateError::JsonLimit);
        }
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Object(members) => members.values().collect(),
        _ => return Ok(()),
    };
    if depth > MAX_JSON_DEPTH {
        return Err(CertificateError::JsonLimit);
    }
    children
        .into_iter()
        .try_for_each(|child| check_json_limits(child, depth + 1))
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
        Claim::ParametricContraction => ClaimDocument::ParametricContraction,
        Claim::Estimate(estimate) => ClaimDocument::Estimate(EstimateDocument {
            quantity: match estimate.quantity {
                Quantity::SteadyState => QuantityDocument::SteadyState,
                Quantity::Success => QuantityDocument::Success,
            },
            value: FractionText::from_fraction(estimate.value.value()),
            samples: estimate.samples,
            epsilon: FractionText::from_fraction(&estimate.epsilon),
            delta: FractionText::from_fraction(&estimate.delta),
            seed: estimate.seed,
        }),
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
            if chain.steady_state().as_ref() != Ok(&value) {
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
        ClaimDocument::Estimate(document) => parse_estimate(index, version, model, document),
        ClaimDocument::ParametricContraction => {
            require_version(index, version, VERSION_4)?;
            let Model::Parametric { a, b } = model else {
                return Err(CertificateError::NotParametric { claim: index });
            };
            if parametric_contraction_holds(a, b) {
                Ok(Claim::ParametricContraction)
            } else {
                Err(false_claim)
            }
        }
    }
}

/// Parses an `Estimate` claim and checks it with [`check_estimate`].
fn parse_estimate(
    index: usize,
    version: u32,
    model: &Model,
    document: &EstimateDocument,
) -> Result<Claim, CertificateError> {
    require_version(index, version, VERSION_4)?;
    let fraction = |text: &FractionText| {
        text.to_fraction(Sign::Forbidden)
            .ok_or(CertificateError::InvalidClaimFraction { claim: index })
    };
    let estimate = Estimate {
        quantity: match document.quantity {
            QuantityDocument::SteadyState => Quantity::SteadyState,
            QuantityDocument::Success => Quantity::Success,
        },
        value: claim_probability(index, &document.value)?,
        samples: document.samples,
        epsilon: fraction(&document.epsilon)?,
        delta: fraction(&document.delta)?,
        seed: document.seed,
    };
    check_estimate(index, &estimate, model)?;
    Ok(Claim::Estimate(estimate))
}

/// Rejects claim `index` of a kind that needs version `needed` in a
/// certificate of an older `version`.
fn require_version(index: usize, version: u32, needed: u32) -> Result<(), CertificateError> {
    if version < needed {
        Err(CertificateError::ClaimNeedsVersion {
            claim: index,
            needed,
        })
    } else {
        Ok(())
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
