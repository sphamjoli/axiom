//! The simulation side: each kind's plan is sized by Hoeffding's count for
//! the same conclusion, and a plan over budget is timed on a prefix and
//! scaled up.

mod common;

use axiom_bench::cases;
use axiom_bench::simulate::{self, GRID, Job, Kind, Plan};
use axiom_sim::required_samples;
use common::{TestError, certificate, ensure, fraction, repo_root};

fn plan_for(source: &str, kind: Kind) -> Result<Plan, TestError> {
    let built = certificate(source, &cases::default_tolerance())?;
    Ok(simulate::plan(kind, &built)?)
}

fn hoeffding(epsilon: (u64, u64), delta: (u64, u64)) -> Result<u64, TestError> {
    required_samples(&fraction(epsilon.0, epsilon.1), &fraction(delta.0, delta.1))
        .map_err(|error| TestError::Unexpected(error.to_string()))
}

#[test]
fn the_links_bound_is_simulated_for_thirteen_steps_from_down_to_its_tolerance()
-> Result<(), TestError> {
    let plan = plan_for(cases::LINKS, Kind::ConvergenceBound)?;
    let samples = hoeffding((1, 1000), (1, 1000))?;
    ensure(
        matches!(
            plan.jobs.as_slice(),
            [Job::Chain { start: 1, steps: 13, samples: n, .. }] if *n == samples
        ),
        format!("unexpected plan {plan:?}"),
    )?;
    ensure(
        plan.draws() == samples * 13 && !plan.lower_bound,
        "the plan's draws are not 13 per sample",
    )
}

#[test]
fn mass_shares_delta_over_every_entry_of_the_matrix() -> Result<(), TestError> {
    let plan = plan_for(&cases::states_model(4), Kind::Mass)?;
    let per_row = hoeffding((1, 1000), (1, 16_000))?;
    ensure(
        plan.jobs.len() == 4
            && plan.jobs.iter().enumerate().all(|(row, job)| {
                matches!(job, Job::Chain { start, steps: 1, samples, .. } if *start == row && *samples == per_row)
            }),
        format!("unexpected plan {plan:?}"),
    )
}

#[test]
fn a_parametric_claim_is_sampled_on_a_grid_as_a_lower_bound() -> Result<(), TestError> {
    let source = std::fs::read_to_string(repo_root().join("fixtures/models/links-param.axm"))?;
    let plan = plan_for(&source, Kind::ParametricContraction)?;
    let per_exit = hoeffding((1, 2000), (1, 2000))?;
    let points = usize::try_from(GRID * GRID).unwrap_or(usize::MAX);
    ensure(
        plan.lower_bound && plan.jobs.len() == 2 * points,
        "the grid plan is not two jobs per point and a lower bound",
    )?;
    ensure(
        plan.samples() == per_exit * 2 * GRID * GRID,
        "each exit is not estimated to epsilon / 2 with delta / 2",
    )
}

#[test]
fn a_kind_the_model_cannot_have_has_no_plan() -> Result<(), TestError> {
    ensure(
        plan_for(cases::LINKS, Kind::Termination).is_err(),
        "a chain was given a termination plan",
    )
}

#[test]
fn a_plan_over_budget_is_extrapolated_and_one_within_is_not() -> Result<(), TestError> {
    let plan = Plan {
        jobs: vec![Job::Chain {
            matrix: certificate(cases::LINKS, &cases::default_tolerance())?
                .matrix()
                .cloned()
                .ok_or_else(|| TestError::Unexpected("no matrix".to_owned()))?,
            start: 0,
            steps: 10,
            samples: 1000,
        }],
        lower_bound: false,
        conclusion: String::new(),
    };
    let within = simulate::execute(&plan, 10_000, 7)?;
    let over = simulate::execute(&plan, 100, 7)?;
    ensure(
        !within.extrapolated,
        "a plan within budget was extrapolated",
    )?;
    ensure(
        over.extrapolated && over.ms > 0.0,
        "a plan over budget was not extrapolated",
    )
}
