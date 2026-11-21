//! The rational interface that proof 3 trusts, checked from outside.
//!
//! Proof 3 models each method of the engine's internal `ExactRational` as the
//! matching operation on rationals. These tests compare every public function
//! that goes through that interface with a closed form computed on numerator
//! and denominator integers alone, at sizes up to the 64-digit limit of
//! ADR-001, so a fault in the interface or in its use shows up here.

mod common;

use axiom_core::{Probability, TwoStateChain};
use num_bigint::BigInt;
use num_rational::BigRational;
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

/// A positive integer of one to 64 decimal digits.
fn positive_big() -> impl Strategy<Value = BigInt> {
    "[1-9][0-9]{0,63}".prop_filter_map("decimal digits always parse", |digits| {
        digits.parse::<BigInt>().ok()
    })
}

/// A probability `numerator / denominator` with up to 64 digits in each part,
/// returned with its unreduced parts.
fn big_probability() -> impl Strategy<Value = (BigInt, BigInt, Probability)> {
    (positive_big(), positive_big()).prop_filter_map(
        "the smaller part over the larger is a probability",
        |(x, y)| {
            let (numerator, denominator) = if x <= y { (x, y) } else { (y, x) };
            let value = BigRational::new(numerator.clone(), denominator.clone());
            let probability = Probability::try_new(&value).ok()?;
            Some((numerator, denominator, probability))
        },
    )
}

/// A rational with up to 64 digits in each part, possibly negative.
fn big_rational() -> impl Strategy<Value = (BigInt, BigInt)> {
    (positive_big(), positive_big(), proptest::bool::ANY).prop_map(
        |(numerator, denominator, negative)| {
            (if negative { -numerator } else { numerator }, denominator)
        },
    )
}

proptest! {
    #[test]
    fn complement_is_denominator_minus_numerator_over_denominator(
        (p, q, probability) in big_probability()
    ) {
        let expected = BigRational::new(&q - &p, q);
        let complement = probability.complement();
        prop_assert_eq!(complement.value(), &expected);
    }

    #[test]
    fn contraction_factor_is_one_minus_a_minus_b_on_integers(
        (p, q, a) in big_probability(),
        (r, s, b) in big_probability(),
    ) {
        let expected = BigRational::new(&q * &s - &p * &s - &r * &q, &q * &s);
        prop_assert_eq!(TwoStateChain::new(a, b).contraction_factor(), expected);
    }

    #[test]
    fn steady_state_is_a_over_a_plus_b_on_integers(
        (p, q, a) in big_probability(),
        (r, s, b) in big_probability(),
    ) {
        let expected = BigRational::new(&p * &s, &p * &s + &r * &q);
        let steady = TwoStateChain::new(a, b).steady_state()?;
        prop_assert_eq!(steady.value(), &expected);
    }

    #[test]
    fn affine_update_is_factor_times_value_plus_offset_on_integers(
        (p, q, a) in big_probability(),
        (r, s, b) in big_probability(),
        (n, m) in big_rational(),
    ) {
        let factor_numerator = &q * &s - &p * &s - &r * &q;
        let factor_denominator = &q * &s;
        let expected = BigRational::new(
            &factor_numerator * &n * &q + &p * &factor_denominator * &m,
            &factor_denominator * &m * &q,
        );
        let update = TwoStateChain::new(a, b).second_state_update();
        prop_assert_eq!(update.apply(&BigRational::new(n, m)), expected);
    }
}
