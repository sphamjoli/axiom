//! Validation and arithmetic of exact probabilities.

mod common;

use axiom_core::{Probability, ProbabilityError};
use common::{TestError, probability, ratio};
use num_bigint::BigInt;
use num_rational::BigRational;

#[test]
fn accepts_zero_and_one_as_boundaries() -> Result<(), TestError> {
    assert_eq!(probability(0, 1)?.value(), &ratio(0, 1));
    assert_eq!(probability(1, 1)?.value(), &ratio(1, 1));
    Ok(())
}

#[test]
fn rejects_values_just_outside_the_unit_interval() {
    assert_eq!(probability(-1, 1_000_000), Err(ProbabilityError::Negative));
    assert_eq!(
        probability(1_000_001, 1_000_000),
        Err(ProbabilityError::AboveOne)
    );
}

#[test]
fn rejects_a_zero_denominator_instead_of_panicking() {
    assert_eq!(probability(1, 0), Err(ProbabilityError::ZeroDenominator));
    let unreduced = BigRational::new_raw(BigInt::from(1), BigInt::from(0));
    assert_eq!(
        Probability::try_new(&unreduced),
        Err(ProbabilityError::ZeroDenominator)
    );
}

#[test]
fn stores_fractions_in_lowest_terms() -> Result<(), TestError> {
    assert_eq!(probability(3, 30)?.value(), &ratio(1, 10));
    let unreduced = BigRational::new_raw(BigInt::from(2), BigInt::from(4));
    assert_eq!(Probability::try_new(&unreduced)?.value(), &ratio(1, 2));
    Ok(())
}

#[test]
fn judges_a_negative_denominator_by_the_sign_of_the_value() -> Result<(), TestError> {
    assert_eq!(probability(-1, -4)?.value(), &ratio(1, 4));
    assert_eq!(probability(1, -4), Err(ProbabilityError::Negative));
    Ok(())
}

#[test]
fn complement_is_exact() -> Result<(), TestError> {
    assert_eq!(probability(1, 10)?.complement().value(), &ratio(9, 10));
    Ok(())
}

#[test]
fn every_error_has_a_distinct_message() {
    assert_eq!(
        ProbabilityError::ZeroDenominator.to_string(),
        "fraction has a zero denominator"
    );
    assert_eq!(
        ProbabilityError::Negative.to_string(),
        "probability is below zero"
    );
    assert_eq!(
        ProbabilityError::AboveOne.to_string(),
        "probability is above one"
    );
}
