//! Hoeffding sample counts and estimates.
//!
//! The exact numbers live in `fixtures/estimate/cases.json`, which the Lean
//! checker's `AxiomCheckTest/Estimate.lean` reads too, so the engine and the
//! checker must agree on every case. The expected bounds there are exact
//! rationals computed independently of both, so rounding a bound up instead
//! of down, or dropping the clamp, changes a value and fails a case.

mod common;

use std::path::PathBuf;

use axiom_sim::{
    Certainty, Estimate, SimError, exp_lower_bound, hoeffding_holds, required_samples,
};
use common::{TestError, ensure, ratio};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use serde_json::Value;

/// The shared estimate cases.
fn cases() -> Result<Value, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/estimate/cases.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

/// The list `name` of the shared cases.
fn list<'a>(cases: &'a Value, name: &str) -> Result<&'a Vec<Value>, TestError> {
    cases
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| TestError::Fixture(name.to_owned()))
}

/// The decimal string `field` of `case`.
fn text<'a>(case: &'a Value, field: &str) -> Result<&'a str, TestError> {
    case.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| TestError::Fixture(field.to_owned()))
}

/// The integer written as the decimal string `field` of `case`.
fn integer(case: &Value, field: &str) -> Result<u64, TestError> {
    text(case, field)?
        .parse()
        .map_err(|_| TestError::Fixture(field.to_owned()))
}

/// The fraction `{"num", "den"}` in `field` of `case`.
fn fraction(case: &Value, field: &str) -> Result<BigRational, TestError> {
    let object = case
        .get(field)
        .ok_or_else(|| TestError::Fixture(field.to_owned()))?;
    let part = |name: &str| -> Result<BigInt, TestError> {
        text(object, name)?
            .parse()
            .map_err(|_| TestError::Fixture(format!("{field}.{name}")))
    };
    let den = part("den")?;
    if den.is_zero() {
        return Err(TestError::Fixture(format!("{field}.den")));
    }
    Ok(BigRational::new(part("num")?, den))
}

#[test]
fn exp_lower_bound_equals_the_exact_floor_rounded_value() -> Result<(), TestError> {
    let all = cases()?;
    let exp = list(&all, "exp")?;
    ensure(!exp.is_empty(), "the fixture has exp cases")?;
    for case in exp {
        let x = fraction(case, "x")?;
        let bound = exp_lower_bound(&x);
        ensure(
            bound == fraction(case, "bound")?,
            format!("exp bound at {x} is {bound}"),
        )?;
    }
    Ok(())
}

#[test]
fn required_samples_is_the_least_accepted_count() -> Result<(), TestError> {
    let all = cases()?;
    for case in list(&all, "required")? {
        let epsilon = fraction(case, "epsilon")?;
        let delta = fraction(case, "delta")?;
        let expected = integer(case, "samples")?;
        let samples = required_samples(&epsilon, &delta)?;
        ensure(samples == expected, format!("required {samples}"))?;
        ensure(
            hoeffding_holds(expected, &epsilon, &delta),
            format!("{expected} meets the bound"),
        )?;
        ensure(
            !hoeffding_holds(expected - 1, &epsilon, &delta),
            format!("{} does not", expected - 1),
        )?;
    }
    Ok(())
}

#[test]
fn hoeffding_verdicts_match_the_fixture() -> Result<(), TestError> {
    let all = cases()?;
    for case in list(&all, "hoeffding")? {
        let n = integer(case, "n")?;
        let accept = match text(case, "verdict")? {
            "accept" => true,
            "reject" => false,
            _ => return Err(TestError::Fixture("verdict".to_owned())),
        };
        ensure(
            hoeffding_holds(n, &fraction(case, "epsilon")?, &fraction(case, "delta")?) == accept,
            format!("verdict for n = {n}"),
        )?;
    }
    Ok(())
}

#[test]
fn an_estimate_is_the_exact_mean_and_is_estimated() -> Result<(), TestError> {
    let all = cases()?;
    let case = list(&all, "required")?
        .first()
        .ok_or_else(|| TestError::Fixture("required".to_owned()))?;
    let samples = integer(case, "samples")?;
    let estimate = Estimate::from_hits(
        9_501,
        samples,
        fraction(case, "epsilon")?,
        fraction(case, "delta")?,
    )?;
    ensure(
        estimate.value == BigRational::new(BigInt::from(9_501), BigInt::from(samples)),
        "the value is hits / samples",
    )?;
    ensure(estimate.samples == samples, "the estimate records n")?;
    ensure(
        estimate.certainty() == Certainty::Estimated,
        "estimates are Estimated",
    )
}

#[test]
fn one_sample_below_the_requirement_is_refused() -> Result<(), TestError> {
    let all = cases()?;
    let case = list(&all, "required")?
        .first()
        .ok_or_else(|| TestError::Fixture("required".to_owned()))?;
    let actual = integer(case, "samples")? - 1;
    let result = Estimate::from_hits(
        0,
        actual,
        fraction(case, "epsilon")?,
        fraction(case, "delta")?,
    );
    ensure(
        result == Err(SimError::TooFewSamples { actual }),
        format!("{actual} samples are too few"),
    )
}

#[test]
fn invalid_parameters_are_refused() -> Result<(), TestError> {
    let half = ratio(1, 2);
    for bad in [BigRational::zero(), BigRational::one(), ratio(-1, 2)] {
        ensure(
            required_samples(&bad, &half) == Err(SimError::InvalidEpsilon),
            format!("epsilon {bad}"),
        )?;
        ensure(
            required_samples(&half, &bad) == Err(SimError::InvalidDelta),
            format!("delta {bad}"),
        )?;
        ensure(
            Estimate::from_hits(1, 1, bad.clone(), half.clone()) == Err(SimError::InvalidEpsilon),
            format!("estimate epsilon {bad}"),
        )?;
        ensure(
            Estimate::from_hits(1, 1, half.clone(), bad.clone()) == Err(SimError::InvalidDelta),
            format!("estimate delta {bad}"),
        )?;
    }
    ensure(
        Estimate::from_hits(3, 2, half.clone(), half) == Err(SimError::HitsExceedSamples),
        "hits above samples",
    )
}

/// `ε = 10^-12` needs about `ln 4 / (2 · 10^-24) ≈ 7 · 10^23` samples, beyond
/// the `2^62` search limit.
#[test]
fn a_tiny_epsilon_is_unreachable() -> Result<(), TestError> {
    ensure(
        required_samples(&ratio(1, 1_000_000_000_000), &ratio(1, 2))
            == Err(SimError::SamplesUnreachable),
        "epsilon 10^-12 is unreachable",
    )
}
