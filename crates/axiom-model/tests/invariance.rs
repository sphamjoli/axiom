//! Layout carries no meaning: whitespace, comments, the order of rows and
//! the order of entries within a row leave the checked model unchanged.

mod common;

use common::{TestError, chain, fixture, matrix, ratio};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

const LINKS_REARRANGED: &str = "# rows swapped, entries swapped, tabs and comments
chain links{states up,down
\tfrom down :down 7/10 ,up 3/10 # repaired
from    up:down 1/10,
   up 9/10}
";

const LINKS_UNREDUCED: &str = "chain links {
  states up, down
  from up:   up 18/20, down 2/20 + 0
  from down: up 1/10 + 2/10, down 7/10
}
";

#[test]
fn layout_and_row_order_do_not_change_the_model() -> Result<(), TestError> {
    let original = chain(&fixture("links.axm")?)?;
    assert_eq!(chain(LINKS_REARRANGED)?, original);
    assert_eq!(chain(LINKS_UNREDUCED)?, original);
    Ok(())
}

#[test]
fn absent_states_have_weight_zero() -> Result<(), TestError> {
    let model = chain(
        "chain c {\n  states s, t, u\n  from s: t 1\n  from t: u 1\n  from u: s 1/2, u 1/2\n}\n",
    )?;
    assert_eq!(
        matrix(&model)?,
        vec![
            vec![ratio(0, 1), ratio(1, 1), ratio(0, 1)],
            vec![ratio(0, 1), ratio(0, 1), ratio(1, 1)],
            vec![ratio(1, 2), ratio(0, 1), ratio(1, 2)],
        ]
    );
    Ok(())
}

#[test]
fn parametric_weights_combine_like_terms() -> Result<(), TestError> {
    let model = chain(
        "param a in [0, 1/2]\nchain c {\n  states s, t\n  from s: s 1 - a - a + 1/2*a + 1/2*a, t a\n  from t: t 1\n}\n",
    )?;
    let first = model.rows.first().and_then(|row| row.first());
    assert_eq!(
        first.map(|weight| weight.coefficients().to_vec()),
        Some(vec![ratio(-1, 1)])
    );
    assert_eq!(
        first.map(|weight| weight.evaluate(&[ratio(1, 4)])),
        Some(ratio(3, 4))
    );
    Ok(())
}

/// A random row of `states` weights with common denominator `denominator`
/// summing to one, as numerators.
fn row(states: usize, denominator: u32) -> impl Strategy<Value = Vec<u32>> {
    proptest::collection::vec(0..=denominator, states - 1).prop_map(move |mut cuts| {
        cuts.sort_unstable();
        let mut numerators = Vec::with_capacity(cuts.len() + 1);
        let mut previous = 0;
        for cut in cuts {
            numerators.push(cut - previous);
            previous = cut;
        }
        numerators.push(denominator - previous);
        numerators
    })
}

fn model() -> impl Strategy<Value = (u32, Vec<Vec<u32>>, Vec<usize>, Vec<bool>)> {
    (1usize..=5, 1u32..=12).prop_flat_map(|(states, denominator)| {
        (
            Just(denominator),
            proptest::collection::vec(row(states, denominator), states),
            Just((0..states).collect::<Vec<_>>()).prop_shuffle(),
            proptest::collection::vec(any::<bool>(), states),
        )
    })
}

fn render(denominator: u32, rows: &[Vec<u32>], order: &[usize], spacing: &[bool]) -> String {
    let names: Vec<String> = (0..rows.len()).map(|index| format!("s_{index}")).collect();
    let mut text = format!("chain random {{\n  states {}\n", names.join(", "));
    for (&index, &loose) in order.iter().zip(spacing) {
        let separator = if loose { " ,\t" } else { "," };
        let entries: Vec<String> = rows
            .get(index)
            .map(|row| {
                row.iter()
                    .zip(&names)
                    .filter(|(numerator, _)| **numerator > 0)
                    .map(|(numerator, name)| format!("{name} {numerator}/{denominator}"))
                    .collect()
            })
            .unwrap_or_default();
        let source = names.get(index).map_or("", String::as_str);
        let comment = if loose { "  # comment" } else { "" };
        let line = format!("  from {source}: {}{comment}\n", entries.join(separator));
        text.push_str(&line);
    }
    text.push_str("}\n");
    text
}

proptest! {
    #[test]
    fn rendered_chains_parse_back_to_their_matrix(
        (denominator, rows, order, spacing) in model()
    ) {
        let parsed = chain(&render(denominator, &rows, &order, &spacing))
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let found = matrix(&parsed).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let wanted: Vec<Vec<_>> = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|numerator| ratio(i64::from(*numerator), i64::from(denominator)))
                    .collect()
            })
            .collect();
        prop_assert_eq!(found, wanted);
    }
}
