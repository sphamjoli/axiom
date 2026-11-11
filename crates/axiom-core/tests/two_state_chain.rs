//! The link model: a two-state chain and the update it induces.

mod common;

use axiom_core::{Probability, TwoStateChain};
use common::{TestError, probability, ratio};
use num_rational::BigRational;
use num_traits::{One, Zero};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

fn link_chain() -> Result<TwoStateChain, TestError> {
    Ok(TwoStateChain::new(probability(1, 10)?, probability(3, 10)?))
}

fn row_values(chain: &TwoStateChain) -> Vec<Vec<BigRational>> {
    chain
        .rows()
        .iter()
        .map(|row| {
            row.probabilities()
                .iter()
                .map(|entry| entry.value().clone())
                .collect()
        })
        .collect()
}

#[test]
fn link_model_rows_are_exact() -> Result<(), TestError> {
    assert_eq!(
        row_values(&link_chain()?),
        vec![
            vec![ratio(9, 10), ratio(1, 10)],
            vec![ratio(3, 10), ratio(7, 10)]
        ]
    );
    Ok(())
}

#[test]
fn link_model_update_is_six_tenths_d_plus_one_tenth() -> Result<(), TestError> {
    let update = link_chain()?.second_state_update();
    assert_eq!(update.factor(), &ratio(3, 5));
    assert_eq!(update.offset(), &ratio(1, 10));
    Ok(())
}

#[test]
fn link_model_iterates_exactly_from_all_links_down() -> Result<(), TestError> {
    let update = link_chain()?.second_state_update();
    let terms: Vec<BigRational> = update.iterate(BigRational::one()).take(4).collect();
    assert_eq!(
        terms,
        vec![ratio(1, 1), ratio(7, 10), ratio(13, 25), ratio(103, 250)]
    );
    Ok(())
}

#[test]
fn chain_that_always_switches_alternates_with_factor_minus_one() -> Result<(), TestError> {
    let update = TwoStateChain::new(probability(1, 1)?, probability(1, 1)?).second_state_update();
    assert_eq!(update.factor(), &ratio(-1, 1));
    let terms: Vec<BigRational> = update.iterate(ratio(1, 4)).take(4).collect();
    assert_eq!(
        terms,
        vec![ratio(1, 4), ratio(3, 4), ratio(1, 4), ratio(3, 4)]
    );
    Ok(())
}

#[test]
fn chain_that_never_moves_keeps_its_value() -> Result<(), TestError> {
    let update = TwoStateChain::new(probability(0, 1)?, probability(0, 1)?).second_state_update();
    assert_eq!(update.factor(), &BigRational::one());
    assert_eq!(update.offset(), &BigRational::zero());
    Ok(())
}

#[test]
fn deviation_after_many_steps_is_exactly_geometric() -> Result<(), TestError> {
    let steps = 100;
    let term = link_chain()?
        .second_state_update()
        .iterate(BigRational::one())
        .nth(steps)
        .ok_or(TestError::MissingTerm(steps))?;
    let factor_power = (0..steps).fold(BigRational::one(), |power, _| power * ratio(3, 5));
    let expected = factor_power * ratio(3, 4);
    assert_eq!(term - ratio(1, 4), expected);
    Ok(())
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
        let rows = row_values(&chain);
        let d = d.value().clone();
        let by_matrix = (BigRational::one() - &d) * &rows[0][1] + &d * &rows[1][1];
        prop_assert_eq!(chain.second_state_update().apply(&d), by_matrix);
    }
}
