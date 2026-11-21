//! Every harness error has a stable, distinct code and a fixed message that
//! names outputs by role and never by path. The table pins both.

mod common;

use std::collections::BTreeSet;

use axiom_bench::error::{BenchError, OutputRole};
use axiom_cli::error::IoFailure;
use common::{TestError, ensure};

#[test]
fn every_error_prints_its_fixed_line() -> Result<(), TestError> {
    let mut expected: Vec<(BenchError, String)> = vec![
        (
            BenchError::ListFixtures(IoFailure::NotFound),
            "list-fixtures-failed: cannot list the fixtures directory: not found".to_owned(),
        ),
        (
            BenchError::ReadModel(IoFailure::PermissionDenied),
            "read-model-failed: cannot read a fixture model: permission denied".to_owned(),
        ),
        (
            BenchError::NoFixtures,
            "no-fixtures: the fixtures directory holds no model".to_owned(),
        ),
        (
            BenchError::CreateDir(IoFailure::Other),
            "create-dir-failed: cannot create an output directory: input or output error"
                .to_owned(),
        ),
        (
            BenchError::FamilyRefused,
            "family-refused: the engine refused a model of the scaling family".to_owned(),
        ),
        (
            BenchError::NoSimulationPlan,
            "no-simulation-plan: no simulation plan reaches the conclusion of a claim".to_owned(),
        ),
        (
            BenchError::WorkTooLarge,
            "work-too-large: a simulation needs more samples than this machine can count"
                .to_owned(),
        ),
        (
            BenchError::CheckRejected,
            "check-rejected: the Lean checker rejected a claim the engine wrote".to_owned(),
        ),
        (
            BenchError::Simulation(axiom_sim::SimError::InvalidEpsilon),
            "invalid-epsilon: epsilon must lie strictly between zero and one".to_owned(),
        ),
        (
            BenchError::Check(axiom_cli::error::CliError::CheckerNotStarted),
            "checker-not-started: cannot run the Lean checker with lake".to_owned(),
        ),
    ];
    for (role, name) in
        OutputRole::ALL
            .into_iter()
            .zip(["results", "comparison", "table", "plot", "certificate"])
    {
        expected.push((
            BenchError::WriteFailed(role, IoFailure::IsADirectory),
            format!("write-failed: cannot write the {name} file: is a directory"),
        ));
    }
    for (error, line) in &expected {
        let report = error.report();
        ensure(
            report == format!("axiom-bench: {line}"),
            format!("unexpected report {report}"),
        )?;
    }
    let own: BTreeSet<&str> = expected
        .iter()
        .filter(|(error, _)| !matches!(error, BenchError::WriteFailed(..)))
        .map(|(error, _)| error.code())
        .collect();
    ensure(
        own.len() == expected.len() - OutputRole::ALL.len(),
        "two errors share a code",
    )
}
