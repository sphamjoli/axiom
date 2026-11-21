//! The cases the harness measures, and what the engine makes of them: the
//! fixtures it finds, and a scaling family in which each dimension really
//! grows.

mod common;

use axiom_bench::cases::{self, Family};
use axiom_bench::engine::{self, Built};
use axiom_bench::error::BenchError;
use axiom_cert::Claim;
use common::{TestError, certificate, ensure, repo_root, scratch};

fn bound_steps(source: &str, tolerance: &num_rational::BigRational) -> Result<u64, TestError> {
    certificate(source, tolerance)?
        .claims()
        .iter()
        .find_map(|claim| match claim {
            Claim::ConvergenceBound { steps, .. } => Some(*steps),
            _ => None,
        })
        .ok_or_else(|| TestError::Unexpected("no convergence bound".to_owned()))
}

#[test]
fn fixtures_are_the_models_and_templates_but_not_the_rejects() -> Result<(), TestError> {
    let found = cases::fixtures(&repo_root().join("fixtures/models"))?;
    let names: Vec<&str> = found.iter().map(|case| case.name.as_str()).collect();
    ensure(
        names
            == [
                "links",
                "links-param",
                "periodic",
                "retry",
                "templates/broken",
                "templates/flip",
                "templates/server",
                "templates/slow-repair",
            ],
        format!("unexpected fixtures {names:?}"),
    )?;
    ensure(
        found.iter().all(|case| case.family == Family::Fixture),
        "a fixture outside the fixture family",
    )
}

#[test]
fn a_directory_without_models_is_refused() -> Result<(), TestError> {
    let dir = scratch("no-models")?;
    std::fs::write(dir.join("notes.txt"), "not a model")?;
    ensure(
        cases::fixtures(&dir) == Err(BenchError::NoFixtures),
        "an empty fixtures directory was accepted",
    )
}

#[test]
fn the_engine_refuses_the_broken_template_with_its_model_code() -> Result<(), TestError> {
    let source = std::fs::read_to_string(repo_root().join("fixtures/models/templates/broken.axm"))?;
    ensure(
        engine::build(&source, &cases::default_tolerance()) == Built::Refused("row-not-normalised"),
        "the broken template was not refused as not normalised",
    )
}

#[test]
fn the_states_family_grows_the_chain_and_gets_mass_claims_only() -> Result<(), TestError> {
    for case in cases::scaling_family()
        .iter()
        .filter(|case| case.family == Family::States)
    {
        let built = certificate(&case.source, &case.tolerance)?;
        let rows = built
            .matrix()
            .map(axiom_core::TransitionMatrix::states)
            .unwrap_or_default();
        ensure(
            u64::try_from(rows).ok() == Some(case.parameter),
            format!("{} has {rows} states", case.name),
        )?;
        ensure(
            built.claims().len() == rows
                && built
                    .claims()
                    .iter()
                    .all(|claim| matches!(claim, Claim::Mass { .. })),
            format!("{} has claims other than one Mass per row", case.name),
        )?;
    }
    Ok(())
}

#[test]
fn the_horizon_and_tolerance_families_need_ever_more_steps() -> Result<(), TestError> {
    for family in [Family::Horizon, Family::Tolerance] {
        let steps = cases::scaling_family()
            .iter()
            .filter(|case| case.family == family)
            .map(|case| bound_steps(&case.source, &case.tolerance))
            .collect::<Result<Vec<u64>, TestError>>()?;
        ensure(
            steps.len() > 1 && steps.is_sorted_by(|left, right| left < right),
            format!("{} steps do not grow: {steps:?}", family.label()),
        )?;
    }
    let links = bound_steps(cases::LINKS, &common::fraction(1, 1000))?;
    ensure(links == 13, format!("links needs {links} steps, not 13"))
}
