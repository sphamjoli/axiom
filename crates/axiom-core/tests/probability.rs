//! Validation and arithmetic of exact probabilities.

mod common;

use axiom_core::{Probability, ProbabilityError};
use common::{probability, ratio};

#[test]
fn accepts_zero_and_one_as_boundaries() {
    assert_eq!(probability(0, 1).value(), &ratio(0, 1));
    assert_eq!(probability(1, 1).value(), &ratio(1, 1));
}

#[test]
fn rejects_values_just_outside_the_unit_interval() {
    assert_eq!(
        Probability::try_new(ratio(-1, 1_000_000)),
        Err(ProbabilityError::Negative)
    );
    assert_eq!(
        Probability::try_new(ratio(1_000_001, 1_000_000)),
        Err(ProbabilityError::AboveOne)
    );
}

#[test]
fn rejects_a_zero_denominator_instead_of_panicking() {
    assert_eq!(
        Probability::try_from_fraction(1, 0),
        Err(ProbabilityError::ZeroDenominator)
    );
}

#[test]
fn stores_fractions_in_lowest_terms() {
    assert_eq!(probability(3, 30).value(), &ratio(1, 10));
}

#[test]
fn treats_a_negative_denominator_by_sign_of_the_value() {
    assert_eq!(probability(-1, -4).value(), &ratio(1, 4));
    assert_eq!(
        Probability::try_from_fraction(1, -4),
        Err(ProbabilityError::Negative)
    );
}

#[test]
fn complement_is_exact() {
    assert_eq!(probability(1, 10).complement().value(), &ratio(9, 10));
}
