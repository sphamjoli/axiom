//! Exact-seed simulation and estimated certainty.
//!
//! [`SplitMix64`] is a small deterministic generator, so a run is reproduced
//! exactly from its seed on every platform. [`sample_chain`] and
//! [`sample_retry`] draw trajectories of a checked model: every random choice
//! compares a 64-bit draw against an exact integer threshold derived from
//! the rational probabilities, with no floating point anywhere in the
//! decision.
//!
//! [`required_samples`] and [`Estimate`] turn samples into an *estimate*: an
//! exact sample mean with the Hoeffding guarantee `P(|mean - μ| ≥ ε) ≤ δ`.
//! An estimate is a statement about the sampling procedure, not a proof
//! about the model, so its certainty is always [`Certainty::Estimated`],
//! never proven. The sample count is decided by [`hoeffding_holds`], the
//! same decision the Lean checker's `checkHoeffding` makes, which is proved
//! sound there: when it accepts, `2 exp(-2 n ε²) ≤ δ` holds over the reals.
//!
//! ```
//! use axiom_sim::required_samples;
//! use num_rational::BigRational;
//!
//! let epsilon = BigRational::new(1.into(), 100.into());
//! let delta = BigRational::new(1.into(), 1000.into());
//! assert_eq!(required_samples(&epsilon, &delta), Ok(38_005));
//! ```

mod error;
mod hoeffding;
mod rng;
mod sample;

pub use axiom_core::{
    EXP_ARGUMENT_CAP, EXP_PRECISION_BITS, EXP_SERIES_TERMS, exp_lower_bound, hoeffding_holds,
};
pub use error::SimError;
pub use hoeffding::{Certainty, Estimate, required_samples};
pub use rng::SplitMix64;
pub use sample::{
    ChainSamples, RetrySamples, Threshold, chain_matrix, choose, retry_probability, row_thresholds,
    sample_chain, sample_paths, sample_retry,
};
