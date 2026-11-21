//! The black-box harness: the adapter agrees with the exact sampler, the
//! parametric adapter agrees with the fixed model at the same point,
//! parameter points stay in range and cover the box, and each limit is
//! enforced at its edge.

mod common;

use axiom_infer::{
    BlackBox, InferError, MAX_CELLS, MAX_POINTS, MAX_SAMPLES, MAX_TRAJECTORY_STEPS, MatrixBox,
    coverage, hits_after, sample_parameters, sample_state,
};
use axiom_sim::SplitMix64;
use common::{TestError, ensure, fixture_box, fixture_chain, matrix_of, ratio};

#[test]
fn the_adapter_reproduces_the_exact_sampler_from_the_same_seed() -> Result<(), TestError> {
    let matrix = matrix_of(&fixture_chain("links.axm")?)?;
    let hits = hits_after(&MatrixBox::new(&matrix), 0, 64, 5_000, 1, 11)?;
    let truth = axiom_sim::sample_chain(&matrix, 0, 64, 5_000, 11)?.count(1);
    ensure(
        hits == truth,
        format!("{hits} hits, the sampler counts {truth}"),
    )
}

#[test]
fn a_parametric_instance_is_the_fixed_model_at_that_point() -> Result<(), TestError> {
    let param = fixture_chain("links-param.axm")?;
    let instance = MatrixBox::instance(&param, &[ratio(1, 10), ratio(3, 10)])?;
    ensure(
        instance == fixture_box("links.axm")?,
        "the instance differs from links",
    )
}

#[test]
fn an_instance_refuses_points_the_model_does_not_cover() -> Result<(), TestError> {
    let param = fixture_chain("links-param.axm")?;
    let wrong_length = MatrixBox::instance(&param, &[ratio(1, 2)]);
    ensure(
        wrong_length
            == Err(InferError::ParameterCount {
                expected: 2,
                actual: 1,
            }),
        "one value for two parameters is refused",
    )?;
    // The ranges are open, so the end 1 itself is outside.
    let at_open_end = MatrixBox::instance(&param, &[ratio(1, 1), ratio(1, 2)]);
    ensure(
        at_open_end == Err(InferError::InvalidInstance),
        "an open end is outside",
    )
}

#[test]
fn sampled_points_lie_in_range_and_cover_the_links_box() -> Result<(), TestError> {
    // 4000 points on a 10 x 10 grid: a given cell is missed with probability
    // (99/100)^4000 < 4e-18, so some cell is missed with probability below
    // 4e-16 for a seed drawn at random. The seed is fixed, so the run is
    // deterministic.
    let param = fixture_chain("links-param.axm")?;
    let points = sample_parameters(&param.parameters, 4_000, 3)?;
    let found = coverage(&param.parameters, &points, 10)?;
    ensure(found.cells == 100, "a 10 x 10 grid has 100 cells")?;
    ensure(
        found.fraction() == ratio(1, 1),
        format!("coverage {}", found.fraction()),
    )?;
    for point in &points {
        MatrixBox::instance(&param, point)?;
    }
    Ok(())
}

#[test]
fn coverage_counts_the_cells_the_points_reach() -> Result<(), TestError> {
    let param = fixture_chain("links-param.axm")?;
    let corner = vec![ratio(1, 100), ratio(99, 100)];
    let opposite = vec![ratio(99, 100), ratio(1, 100)];
    let found = coverage(&param.parameters, &[corner.clone(), corner, opposite], 2)?;
    ensure(found.hit == 2 && found.cells == 4, format!("{found:?}"))
}

#[test]
fn coverage_and_sampling_enforce_their_limits() -> Result<(), TestError> {
    let param = fixture_chain("links-param.axm")?;
    let grid = Err(InferError::InvalidGrid { limit: MAX_CELLS });
    ensure(
        coverage(&param.parameters, &[], 0) == grid,
        "zero cells per axis",
    )?;
    // 1024^2 is exactly the limit; 1025^2 is above it.
    ensure(
        coverage(&param.parameters, &[], 1024).is_ok(),
        "the limit itself",
    )?;
    ensure(
        coverage(&param.parameters, &[], 1025) == grid,
        "above the limit",
    )?;
    ensure(
        coverage(&param.parameters, &[vec![ratio(1, 2)]], 2)
            == Err(InferError::ParameterCount {
                expected: 2,
                actual: 1,
            }),
        "a point of the wrong length",
    )?;
    ensure(
        coverage(&param.parameters, &[vec![ratio(0, 1), ratio(1, 2)]], 2)
            == Err(InferError::InvalidInstance),
        "a point outside the box",
    )?;
    ensure(
        sample_parameters(&[], MAX_POINTS, 1)?.len() == MAX_POINTS,
        "at the point limit",
    )?;
    ensure(
        sample_parameters(&param.parameters, MAX_POINTS + 1, 1)
            == Err(InferError::TooManyPoints { limit: MAX_POINTS }),
        "above the point limit",
    )
}

#[test]
fn trajectories_enforce_their_limits() -> Result<(), TestError> {
    let black_box = fixture_box("links.axm")?;
    let too_many = Err(InferError::TooManySamples { limit: MAX_SAMPLES });
    ensure(
        hits_after(&black_box, 0, MAX_TRAJECTORY_STEPS, 1, 1, 0).is_ok(),
        "at the step limit",
    )?;
    ensure(
        hits_after(&black_box, 0, MAX_TRAJECTORY_STEPS + 1, 1, 1, 0) == too_many,
        "above the step limit",
    )?;
    ensure(
        hits_after(&black_box, 0, 0, MAX_SAMPLES, 1, 0).is_ok(),
        "at the sample limit",
    )?;
    ensure(
        hits_after(&black_box, 0, 0, MAX_SAMPLES + 1, 1, 0) == too_many,
        "above the sample limit",
    )?;
    ensure(
        hits_after(&black_box, 2, 1, 1, 1, 0)
            == Err(InferError::Sim(axiom_sim::SimError::StateOutOfRange(2))),
        "a start outside the chain",
    )
}

/// A faulty black box that steps to a state it does not have.
struct Escapes;

impl BlackBox for Escapes {
    fn states(&self) -> usize {
        2
    }

    fn step(&self, _state: usize, _rng: &mut SplitMix64) -> usize {
        2
    }
}

#[test]
fn a_step_outside_the_chain_is_refused() -> Result<(), TestError> {
    ensure(
        hits_after(&Escapes, 0, 1, 1, 0, 0) == Err(InferError::StateOutOfRange),
        "the escape is refused",
    )?;
    let config = axiom_infer::Config::new(ratio(1, 10), ratio(1, 10), 0);
    ensure(
        axiom_infer::propose(&Escapes, &config) == Err(InferError::StateOutOfRange),
        "the escape is refused while proposing",
    )
}

#[test]
fn initial_states_are_uniform() -> Result<(), TestError> {
    // 30000 draws over three states: by Hoeffding, a count is off its mean
    // 10000 by 600 or more with probability below 2 exp(-2 * 600^2 / 30000)
    // < 1e-10. The seed is fixed.
    let mut rng = SplitMix64::new(5);
    let mut counts = [0_u32; 3];
    for _ in 0..30_000 {
        if let Some(count) = counts.get_mut(sample_state(3, &mut rng)) {
            *count += 1;
        }
    }
    ensure(
        counts.iter().all(|c| c.abs_diff(10_000) < 600),
        format!("{counts:?}"),
    )
}
