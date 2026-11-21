//! Candidates proposed from samples: the links chain gives bounds that hold
//! its exact fixed point and factor, the candidates are exact ratios of
//! counts, the periodic chain is flagged non-convergent, and each limit is
//! enforced.

mod common;

use axiom_infer::{Config, InferError, MAX_SAMPLES, MatrixBox, propose};
use common::{TestError, chain_of, ensure, fixture_box, matrix_of, ratio};

fn to_i64(count: u64) -> Result<i64, TestError> {
    i64::try_from(count).map_err(|_| TestError::Mismatch("count too large".into()))
}

#[test]
fn links_gives_bounds_that_hold_one_quarter_and_three_fifths() -> Result<(), TestError> {
    // The bounds miss the true values with probability at most delta = 1/1000
    // over the sampling; the seed is fixed, so the run is deterministic.
    let config = Config::new(ratio(1, 100), ratio(1, 1000), 7);
    let found = propose(&fixture_box("links.axm")?, &config)?;
    let fixed = found
        .fixed_point_bounds
        .clone()
        .ok_or(TestError::Mismatch("no fixed point bounds".into()))?;
    ensure(
        fixed.contains(&ratio(1, 4)),
        format!("{fixed:?} misses 1/4"),
    )?;
    ensure(
        found.contraction_bounds.contains(&ratio(3, 5)),
        "the factor bounds miss 3/5",
    )?;
    ensure(found.convergent(), "links settles")?;
    // The candidates are exact ratios of counts: with n steps from each
    // state, x and y of them exits, â/(â+b̂) = x/(x+y) and 1-â-b̂ = (n-x-y)/n.
    let (first, second) = (found.leave_first, found.leave_second);
    ensure(
        first.visits == second.visits,
        "the same steps from each state",
    )?;
    let (n, x, y) = (
        to_i64(first.visits)?,
        to_i64(first.exits)?,
        to_i64(second.exits)?,
    );
    ensure(
        found.fixed_point == Some(ratio(x, x + y)),
        "the fixed point is x/(x+y)",
    )?;
    ensure(
        found.contraction == ratio(n - x - y, n),
        "the factor is (n-x-y)/n",
    )
}

#[test]
fn the_periodic_chain_is_flagged_non_convergent() -> Result<(), TestError> {
    let config = Config::new(ratio(1, 100), ratio(1, 1000), 7);
    let found = propose(&fixture_box("periodic.axm")?, &config)?;
    ensure(
        found.contraction == ratio(-1, 1),
        "every step leaves its state",
    )?;
    ensure(!found.convergent(), "a = b = 1 is flagged")
}

#[test]
fn a_chain_that_never_moves_has_no_fixed_point_candidate() -> Result<(), TestError> {
    let source = "chain still {\n  states x, y\n  from x: x 1\n  from y: y 1\n}\n";
    let config = Config::new(ratio(1, 10), ratio(1, 10), 1);
    let found = propose(&MatrixBox::new(&matrix_of(&chain_of(source)?)?), &config)?;
    ensure(found.fixed_point.is_none(), "a + b = 0 has no fixed point")?;
    ensure(
        found.fixed_point_bounds.is_none(),
        "the bounds admit a + b = 0",
    )?;
    ensure(!found.convergent(), "lambda = 1 is flagged")
}

#[test]
fn propose_enforces_its_limits() -> Result<(), TestError> {
    let links = fixture_box("links.axm")?;
    let mut config = Config::new(ratio(1, 100), ratio(1, 1000), 7);
    config.max_steps = 10;
    ensure(
        propose(&links, &config) == Err(InferError::StepBudgetExceeded { limit: 10 }),
        "the step budget",
    )?;
    // epsilon 1/3000 needs about 4.1e7 samples per state, above the limit.
    let fine = Config::new(ratio(1, 3_000), ratio(1, 1000), 7);
    ensure(
        propose(&links, &fine) == Err(InferError::TooManySamples { limit: MAX_SAMPLES }),
        "the sample limit",
    )?;
    let source = "chain c {\n  states x, y, z\n  from x: x 1\n  from y: y 1\n  from z: z 1\n}\n";
    let three = MatrixBox::new(&matrix_of(&chain_of(source)?)?);
    ensure(
        propose(&three, &config) == Err(InferError::NotTwoState { states: 3 }),
        "three states",
    )
}
