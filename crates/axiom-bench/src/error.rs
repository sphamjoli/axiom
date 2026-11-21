//! Errors reported by the benchmark harness.
//!
//! Like the command line's, each error has a stable kebab-case code and a
//! fixed message that names files by role, never by path, and never quotes
//! input or operating system messages. Each is printed as
//! `axiom-bench: <code>: <message>`.

use std::fmt;

use axiom_cert::CertificateError;
use axiom_cli::error::{CliError, IoFailure};
use axiom_sim::SimError;
use thiserror::Error;

/// Which output file an operation was about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRole {
    /// The per-case measurements, `results.csv`.
    Results,
    /// The per-claim comparison, `comparison.csv`.
    Comparison,
    /// The Markdown table, `comparison.md`.
    Table,
    /// An SVG plot.
    Plot,
    /// A certificate handed to the checker.
    Certificate,
}

impl OutputRole {
    /// Every role.
    pub const ALL: [Self; 5] = [
        Self::Results,
        Self::Comparison,
        Self::Table,
        Self::Plot,
        Self::Certificate,
    ];
}

impl fmt::Display for OutputRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Results => "results",
            Self::Comparison => "comparison",
            Self::Table => "table",
            Self::Plot => "plot",
            Self::Certificate => "certificate",
        })
    }
}

/// A failure that ends a benchmark run.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BenchError {
    /// The fixtures directory could not be listed.
    #[error("cannot list the fixtures directory: {0}")]
    ListFixtures(IoFailure),
    /// A fixture model could not be read.
    #[error("cannot read a fixture model: {0}")]
    ReadModel(IoFailure),
    /// The fixtures directory holds no model.
    #[error("the fixtures directory holds no model")]
    NoFixtures,
    /// An output directory could not be created.
    #[error("cannot create an output directory: {0}")]
    CreateDir(IoFailure),
    /// An output file could not be written.
    #[error("cannot write the {0} file: {1}")]
    WriteFailed(OutputRole, IoFailure),
    /// The engine refused a model of the generated scaling family.
    #[error("the engine refused a model of the scaling family")]
    FamilyRefused,
    /// No simulation reaches the conclusion of a claim the engine wrote.
    #[error("no simulation plan reaches the conclusion of a claim")]
    NoSimulationPlan,
    /// A simulation needs more samples than this machine can count.
    #[error("a simulation needs more samples than this machine can count")]
    WorkTooLarge,
    /// The Lean checker rejected a claim the engine wrote.
    #[error("the Lean checker rejected a claim the engine wrote")]
    CheckRejected,
    /// The Lean checker could not give a verdict.
    #[error(transparent)]
    Check(CliError),
    /// A Hoeffding sample count could not be found.
    #[error(transparent)]
    Simulation(SimError),
    /// A certificate could not be built or encoded.
    #[error(transparent)]
    Certificate(CertificateError),
}

impl BenchError {
    /// A stable kebab-case name for the error. A wrapped error keeps its own
    /// crate's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ListFixtures(_) => "list-fixtures-failed",
            Self::ReadModel(_) => "read-model-failed",
            Self::NoFixtures => "no-fixtures",
            Self::CreateDir(_) => "create-dir-failed",
            Self::WriteFailed(..) => "write-failed",
            Self::FamilyRefused => "family-refused",
            Self::NoSimulationPlan => "no-simulation-plan",
            Self::WorkTooLarge => "work-too-large",
            Self::CheckRejected => "check-rejected",
            Self::Check(error) => error.code(),
            Self::Simulation(error) => error.code(),
            Self::Certificate(error) => error.code(),
        }
    }

    /// The line printed on the error stream: `axiom-bench: <code>: <message>`.
    #[must_use]
    pub fn report(&self) -> String {
        format!("axiom-bench: {}: {self}", self.code())
    }
}
