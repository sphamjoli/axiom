//! Hoeffding sample counts and estimates.

mod common;

use axiom_sim::{
    Certainty, Estimate, SimError, chain_matrix, exp_lower_bound, hoeffding_holds,
    required_samples, sample_chain,
};
use common::{TestError, ensure, links, ratio};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

#[test]
fn exp_lower_bound_is_one_at_zero() -> Result<(), TestError> {
    ensure(
        exp_lower_bound(&BigRational::zero()) == BigRational::one(),
        "exp(0) bound is 1",
    )
}

#[test]
fn exp_lower_bound_is_tight_and_below_e() -> Result<(), TestError> {
    let bound = exp_lower_bound(&BigRational::one());
    // e = 2.718281828459045...
    ensure(
        bound > ratio(2_718_281_828, 1_000_000_000),
        "bound is within 10^-9 of e",
    )?;
    ensure(
        bound < ratio(2_718_281_829, 1_000_000_000),
        "bound is below e",
    )
}

#[test]
fn exp_lower_bound_is_below_exp_of_large_arguments() -> Result<(), TestError> {
    // exp(10) = 22026.4657948...
    let bound = exp_lower_bound(&ratio(10, 1));
    ensure(bound > ratio(220_264, 10), "bound is close to exp(10)")?;
    ensure(bound < ratio(2_202_647, 100), "bound is below exp(10)")
}

/// `ln(2000) / (2 · 10^-4) = 38004.51...`, so 38005 samples are needed for
/// `ε = 1/100` and `δ = 1/1000`, and the rational bound finds exactly that.
#[test]
fn d_star_within_one_hundredth_at_confidence_0_999_needs_38005_samples() -> Result<(), TestError> {
    let epsilon = ratio(1, 100);
    let delta = ratio(1, 1000);
    let samples = required_samples(&epsilon, &delta)?;
    ensure(samples == 38_005, format!("required {samples}"))?;
    ensure(
        hoeffding_holds(38_005, &epsilon, &delta),
        "38005 meets the bound",
    )?;
    ensure(!hoeffding_holds(38_004, &epsilon, &delta), "38004 does not")
}

/// `ln(2 · 10^9) / (2 · (1/50)^2) = 26770.52...`, the Lean tests' second case.
#[test]
fn one_fiftieth_at_one_in_a_billion_needs_26771_samples() -> Result<(), TestError> {
    let samples = required_samples(&ratio(1, 50), &ratio(1, 1_000_000_000))?;
    ensure(samples == 26_771, format!("required {samples}"))
}

#[test]
fn estimate_of_d_star_is_within_tolerance() -> Result<(), TestError> {
    let epsilon = ratio(1, 100);
    let delta = ratio(1, 1000);
    let samples = required_samples(&epsilon, &delta)?;
    let chain = links()?;
    let count = usize::try_from(samples).map_err(|_| TestError::Form)?;
    let run = sample_chain(chain_matrix(&chain)?, 0, 60, count, 1)?;
    let estimate = Estimate::from_hits(run.count(1), run.samples(), epsilon.clone(), delta)?;
    ensure(estimate.samples == 38_005, "the estimate records n")?;
    ensure(
        estimate.certainty() == Certainty::Estimated,
        "estimates are Estimated",
    )?;
    let error = (&estimate.value - ratio(1, 4)).abs();
    ensure(error < epsilon, format!("estimate misses d* by {error}"))
}

#[test]
fn too_few_samples_are_refused() -> Result<(), TestError> {
    let result = Estimate::from_hits(9_501, 38_004, ratio(1, 100), ratio(1, 1000));
    ensure(
        result == Err(SimError::TooFewSamples { actual: 38_004 }),
        "38004 samples are too few",
    )
}

#[test]
fn invalid_parameters_are_refused() -> Result<(), TestError> {
    ensure(
        required_samples(&BigRational::zero(), &ratio(1, 2)) == Err(SimError::InvalidEpsilon),
        "epsilon zero",
    )?;
    ensure(
        required_samples(&ratio(1, 2), &BigRational::one()) == Err(SimError::InvalidDelta),
        "delta one",
    )?;
    ensure(
        Estimate::from_hits(3, 2, ratio(1, 2), ratio(1, 2)) == Err(SimError::HitsExceedSamples),
        "hits above samples",
    )
}

#[test]
fn hoeffding_rejects_non_positive_delta() -> Result<(), TestError> {
    ensure(
        !hoeffding_holds(1_000_000, &ratio(1, 2), &BigRational::zero()),
        "delta zero",
    )
}
