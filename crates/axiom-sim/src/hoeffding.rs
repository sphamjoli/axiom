//! Hoeffding sample counts and estimates.
//!
//! The decision whether a sample count is enough, [`hoeffding_holds`], is
//! `axiom_core`'s, shared with the certificate reader; this module searches
//! for the least sufficient count and builds estimates from sample counts.

use axiom_core::hoeffding_holds;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::error::SimError;

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
