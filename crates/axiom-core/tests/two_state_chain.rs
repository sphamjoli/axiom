//! The link model: a two-state chain and the update it induces.

mod common;

use axiom_core::{Probability, TwoStateChain};
use common::{probability, ratio};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

fn link_chain() -> TwoStateChain {
    TwoStateChain::new(probability(1, 10), probability(3, 10))
}

fn row_values(chain: &TwoStateChain) -> [[BigRational; 2]; 2] {
    chain.rows().map(|row| {
        let entries = row.probabilities();
        [entries[0].value().clone(), entries[1].value().clone()]
    })
}

#[test]
fn link_model_rows_are_exact() {
    assert_eq!(
        row_values(&link_chain()),
        [[ratio(9, 10), ratio(1, 10)], [ratio(3, 10), ratio(7, 10)]]
    );
}

#[test]
fn link_model_update_is_six_tenths_d_plus_one_tenth() {
    let update = link_chain().second_state_update();
    assert_eq!(update.factor(), &ratio(3, 5));
    assert_eq!(update.offset(), &ratio(1, 10));
}

#[test]
fn link_model_iterates_exactly_from_all_links_down() {
    let terms: Vec<BigRational> = link_chain()
        .second_state_update()
        .iterate(BigRational::one())
        .take(4)
        .collect();
    assert_eq!(
        terms,
        vec![ratio(1, 1), ratio(7, 10), ratio(13, 25), ratio(103, 250)]
    );
}

#[test]
fn chain_that_always_switches_alternates_with_factor_minus_one() {
    let switching = TwoStateChain::new(probability(1, 1), probability(1, 1));
    let update = switching.second_state_update();
    assert_eq!(update.factor(), &ratio(-1, 1));
    let terms: Vec<BigRational> = update.iterate(ratio(1, 4)).take(4).collect();
    assert_eq!(
        terms,
        vec![ratio(1, 4), ratio(3, 4), ratio(1, 4), ratio(3, 4)]
    );
}

#[test]
fn chain_that_never_moves_keeps_its_value() {
    let frozen = TwoStateChain::new(probability(0, 1), probability(0, 1));
    let update = frozen.second_state_update();
    assert_eq!(update.factor(), &BigRational::one());
    assert_eq!(update.offset(), &BigRational::zero());
}

fn any_probability() -> impl Strategy<Value = Probability> {
    (0_i64..=1_000, 1_i64..=1_000).prop_filter_map("fraction must not exceed one", |(n, d)| {
        Probability::try_from_fraction(n, d).ok()
    })
}

proptest! {
    #[test]
    fn every_row_of_every_chain_sums_to_exactly_one(a in any_probability(), b in any_probability()) {
        for row in TwoStateChain::new(a, b).rows() {
            let sum = row
                .probabilities()
                .iter()
                .fold(BigRational::zero(), |total, entry| total + entry.value());
            prop_assert_eq!(sum, BigRational::one());
        }
    }

    #[test]
    fn update_matches_one_step_of_the_transition_matrix(
        a in any_probability(),
        b in any_probability(),
        d in any_probability(),
    ) {
        let chain = TwoStateChain::new(a, b);
        let [from_first, from_second] = row_values(&chain);
        let d = d.value().clone();
        let by_matrix = (BigRational::one() - &d) * &from_first[1] + &d * &from_second[1];
        prop_assert_eq!(chain.second_state_update().apply(&d), by_matrix);
    }

    #[test]
    fn factor_is_one_minus_both_exit_probabilities(a in any_probability(), b in any_probability()) {
        let expected = BigRational::one() - a.value() - b.value();
        let update = TwoStateChain::new(a, b).second_state_update();
        prop_assert_eq!(update.factor(), &expected);
    }
}

#[test]
fn large_step_counts_stay_exact() {
    let hundredth = link_chain()
        .second_state_update()
        .iterate(BigRational::one())
        .nth(100)
        .expect("the iterator is unbounded");
    let quarter = ratio(1, 4);
    let deviation = BigRational::new(BigInt::from(3), BigInt::from(5)).pow(100) * ratio(3, 4);
    assert_eq!(hundredth - quarter, deviation);
}
