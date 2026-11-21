//! Hoeffding sample counts, decided exactly over the rationals.
//!
//! For `n` independent samples in `[0, 1]` with mean `μ`, Hoeffding's
//! inequality gives `P(|mean - μ| ≥ ε) ≤ 2 exp(-2 n ε²)`. A sample count is
//! enough for failure probability `δ` when `2 exp(-2 n ε²) ≤ δ`, that is
//! when `exp(x) ≥ 2 / δ` for `x = 2 n ε²`. [`exp_lower_bound`] computes a
//! rational `L ≤ exp(x)`, and [`hoeffding_holds`] accepts when `2 ≤ δ L`.
//! Using a lower bound for `exp(x)`, an upper bound for `exp(-x)`, is the
//! direction that keeps the decision sound: it can only reject a count that
//! is enough, never accept one that is not. The Lean checker's
//! `checkHoeffding` runs the same computation step for step, and its
//! soundness theorem proves the real inequality whenever it accepts.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::error::SimError;

/// The terms `Σ_{k < K} y^k / k!` of the series lower bound for `exp(y)`.
pub const EXP_SERIES_TERMS: u32 = 16;

/// The bits kept after the binary point when rounding a bound down.
pub const EXP_PRECISION_BITS: u32 = 128;

/// The largest argument the bound looks at: `exp(min(x, 256)) ≤ exp(x)`.
pub const EXP_ARGUMENT_CAP: u32 = 256;

/// The halving steps the argument reduction tries at most.
const HALVING_FUEL: u32 = 16;

/// The certainty of a result. Estimates are always [`Certainty::Estimated`]:
/// they hold with probability at least `1 - δ` over the sampling, and are
/// never [`Certainty::Proven`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    /// Established for every outcome by a checked proof.
    Proven,
    /// Established with a stated failure probability by sampling.
    Estimated,
}

fn integer(value: u64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

fn power_of_two(bits: u32) -> BigRational {
    BigRational::from_integer(BigInt::from(1) << bits)
}

/// `max(0, ⌊z · 2^P⌋ / 2^P)`: a lower bound of a non-negative `z` with `P`
/// bits after the binary point, so the numbers stay small.
fn round_down(z: &BigRational) -> BigRational {
    let scale = power_of_two(EXP_PRECISION_BITS);
    let rounded = (z * &scale).floor() / scale;
    if rounded < BigRational::zero() {
        BigRational::zero()
    } else {
        rounded
    }
}

/// `Σ_{k < K} y^k / k!`, which is at most `exp(y)` for `y ≥ 0`.
fn series(y: &BigRational) -> BigRational {
    let mut sum = BigRational::zero();
    let mut term = BigRational::one();
    for k in 0..EXP_SERIES_TERMS {
        sum += &term;
        term = term * y / integer(u64::from(k) + 1);
    }
    sum
}

/// The number of halvings that bring `x` to at most one, up to the fuel.
/// Soundness does not depend on this choice; it only keeps the series
/// accurate.
fn halvings(x: &BigRational) -> u32 {
    let mut current = x.clone();
    let mut count = 0;
    let two = integer(2);
    while count < HALVING_FUEL && current > BigRational::one() {
        current /= &two;
        count += 1;
    }
    count
}

/// A rational lower bound for `exp(x)`.
///
/// The argument is clamped to `[0, 256]`, reduced to `y = x / 2^s` with `s`
/// halvings, bounded below by the series `Σ_{k < 16} y^k / k!`, and squared
/// back `s` times, `exp(x) = exp(y)^(2^s)`. Every intermediate value is
/// rounded down to 128 binary places, and squaring is monotone on
/// non-negative numbers, so each step stays a lower bound.
#[must_use]
pub fn exp_lower_bound(x: &BigRational) -> BigRational {
    let cap = integer(u64::from(EXP_ARGUMENT_CAP));
    let clamped = if *x < BigRational::zero() {
        BigRational::zero()
    } else if *x > cap {
        cap
    } else {
        x.clone()
    };
    let steps = halvings(&clamped);
    let reduced = clamped / power_of_two(steps);
    let mut bound = round_down(&series(&reduced));
    for _ in 0..steps {
        bound = round_down(&(&bound * &bound));
    }
    bound
}

/// Whether `n` samples give tolerance `epsilon` with failure probability at
/// most `delta`: accepts only when `2 exp(-2 n ε²) ≤ δ` holds. A `delta` of
/// zero or below is always rejected, because the bound is non-negative and
/// so `δ L ≤ 0 < 2`; the Lean checker tests `0 < δ` explicitly for its proof.
#[must_use]
pub fn hoeffding_holds(n: u64, epsilon: &BigRational, delta: &BigRational) -> bool {
    let exponent = integer(2) * integer(n) * epsilon * epsilon;
    integer(2) <= delta * exp_lower_bound(&exponent)
}

fn in_open_unit(value: &BigRational) -> bool {
    *value > BigRational::zero() && *value < BigRational::one()
}

/// The sample count for tolerance `epsilon` and failure probability
/// `delta`: the least `n` that [`hoeffding_holds`] accepts, found by
/// doubling then bisection. It equals `⌈ln(2/δ) / (2 ε²)⌉` whenever the
/// rational bound is tight enough to separate it from the count below, and
/// it is never smaller, because an accepted count meets the real bound.
///
/// # Errors
///
/// [`SimError::InvalidEpsilon`] or [`SimError::InvalidDelta`] outside
/// `(0, 1)`, and [`SimError::SamplesUnreachable`] when no count up to
/// `2^62` is accepted.
pub fn required_samples(epsilon: &BigRational, delta: &BigRational) -> Result<u64, SimError> {
    if !in_open_unit(epsilon) {
        return Err(SimError::InvalidEpsilon);
    }
    if !in_open_unit(delta) {
        return Err(SimError::InvalidDelta);
    }
    // With δ < 1, zero samples are never enough, so `low` starts rejected.
    let mut low = 0_u64;
    let mut high = 1_u64;
    while !hoeffding_holds(high, epsilon, delta) {
        if high >= 1 << 62 {
            return Err(SimError::SamplesUnreachable);
        }
        low = high;
        high *= 2;
    }
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if hoeffding_holds(middle, epsilon, delta) {
            high = middle;
        } else {
            low = middle;
        }
    }
    Ok(high)
}

/// An estimated mean: with probability at least `1 - delta` over the
/// sampling, the true mean lies within `epsilon` of `value`.
///
/// Its certainty is always [`Certainty::Estimated`]. It is built only when
/// [`hoeffding_holds`] accepts the sample count, so the Lean checker's
/// `checkHoeffding` accepts the same claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate {
    /// The exact sample mean.
    pub value: BigRational,
    /// The number of samples.
    pub samples: u64,
    /// The tolerance.
    pub epsilon: BigRational,
    /// The failure probability.
    pub delta: BigRational,
}

impl Estimate {
    /// The estimate from `hits` successes out of `samples` independent
    /// samples.
    ///
    /// # Errors
    ///
    /// [`SimError::InvalidEpsilon`] or [`SimError::InvalidDelta`] outside
    /// `(0, 1)`, [`SimError::HitsExceedSamples`] when `hits > samples`, and
    /// [`SimError::TooFewSamples`] when the count does not meet the bound.
    pub fn from_hits(
        hits: u64,
        samples: u64,
        epsilon: BigRational,
        delta: BigRational,
    ) -> Result<Self, SimError> {
        if !in_open_unit(&epsilon) {
            return Err(SimError::InvalidEpsilon);
        }
        if !in_open_unit(&delta) {
            return Err(SimError::InvalidDelta);
        }
        if hits > samples {
            return Err(SimError::HitsExceedSamples);
        }
        if !hoeffding_holds(samples, &epsilon, &delta) {
            return Err(SimError::TooFewSamples { actual: samples });
        }
        Ok(Self {
            value: BigRational::new(BigInt::from(hits), BigInt::from(samples)),
            samples,
            epsilon,
            delta,
        })
    }

    /// The certainty of an estimate, always [`Certainty::Estimated`].
    #[must_use]
    pub fn certainty(&self) -> Certainty {
        Certainty::Estimated
    }
}
