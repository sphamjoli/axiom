//! The verification coverage report: what it reads from the source tree,
//! that it refuses a tree with a `sorry`, and its fixed error lines.

mod common;

use std::path::Path;

use axiom_bench::coverage::{
    self, CoverageError, CoverageRole, FunctionStatus, count_sorry, rust_functions, strip_comments,
};
use axiom_cli::error::IoFailure;
use common::{TestError, ensure, repo_root, scratch};

fn coverage_error(error: &CoverageError) -> TestError {
    TestError::Unexpected(error.report())
}

/// Copies `from` into `to`, skipping `.lake`.
fn copy_tree(from: &Path, to: &Path) -> Result<(), TestError> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        if name == ".lake" {
            continue;
        }
        if path.is_dir() {
            copy_tree(&path, &to.join(name))?;
        } else {
            std::fs::copy(&path, to.join(name))?;
        }
    }
    Ok(())
}

/// A copy of the inputs the report reads.
fn copy_inputs(name: &str) -> Result<std::path::PathBuf, TestError> {
    let root = repo_root();
    let copy = scratch(name)?;
    for dir in ["lean", "lean-engine", "crates/axiom-core/src"] {
        copy_tree(&root.join(dir), &copy.join(dir))?;
    }
    std::fs::copy(root.join("proof3.toml"), copy.join("proof3.toml"))?;
    Ok(copy)
}

#[test]
fn the_repository_has_no_sorry_and_every_kind_and_kernel_function_proved() -> Result<(), TestError>
{
    let report = coverage::generate(&repo_root()).map_err(|error| coverage_error(&error))?;
    ensure(
        report.kinds.len() == 7 && report.kinds.iter().all(|kind| kind.proved),
        "a claim kind is computation only",
    )?;
    for project in &report.projects {
        ensure(
            project.sorry == 0,
            format!("{} has a sorry", project.project),
        )?;
        ensure(
            project.allowed_axioms == ["propext", "Classical.choice", "Quot.sound"],
            format!("{} allows {:?}", project.project, project.allowed_axioms),
        )?;
    }
    let kernel: Vec<&str> = report
        .functions
        .iter()
        .filter(|row| row.module == "kernel")
        .map(|row| row.function.as_str())
        .collect();
    ensure(
        kernel
            == [
                "complement",
                "contraction_factor",
                "steady_state",
                "affine_apply",
            ],
        format!("unexpected kernel functions {kernel:?}"),
    )?;
    ensure(
        report.count(FunctionStatus::Proved) == kernel.len(),
        "a kernel function lacks a correctness theorem",
    )?;
    let hoeffding = report
        .functions
        .iter()
        .find(|row| row.function == "hoeffding_holds")
        .ok_or_else(|| TestError::Unexpected("hoeffding_holds is missing".to_owned()))?;
    ensure(
        hoeffding.status == FunctionStatus::Uncovered,
        "an uncovered function is reported as covered",
    )
}

#[test]
fn a_sorry_outside_comments_fails_generation() -> Result<(), TestError> {
    let copy = copy_inputs("coverage-sorry")?;
    let extra = copy.join("lean-engine/AxiomEngine/Extra.lean");
    std::fs::write(&extra, "-- sorry\n/- sorry /- sorry -/ -/\n")?;
    coverage::generate(&copy).map_err(|error| coverage_error(&error))?;
    std::fs::write(&extra, "theorem t : True := by sorry\n")?;
    let outcome = coverage::generate(&copy);
    ensure(
        outcome == Err(CoverageError::SorryFound(1)),
        format!("unexpected outcome {outcome:?}"),
    )
}

#[test]
fn sorry_is_counted_as_a_whole_word_only() -> Result<(), TestError> {
    let code = strip_comments("def sorryAx := 1\nexample : x := sorry\n-- sorry\n");
    ensure(count_sorry(&code) == 1, format!("counted in {code:?}"))
}

#[test]
fn methods_are_qualified_by_their_type_and_tests_are_skipped() -> Result<(), TestError> {
    let source = "pub(crate) fn free() {}\n\
                  impl Thing {\n    pub const fn code(&self) {}\n    fn private() {}\n}\n\
                  fn after() {}\n#[cfg(test)]\nmod tests {\n    fn hidden() {}\n}\n";
    let found = rust_functions(source);
    ensure(
        found == ["free", "Thing::code", "Thing::private", "after"],
        format!("found {found:?}"),
    )
}

#[test]
fn every_error_prints_its_fixed_line() -> Result<(), TestError> {
    let mut expected = vec![
        (
            CoverageError::NoAllowedAxioms,
            "axiom-coverage: no-allowed-axioms: an axiom audit names no allowed axioms".to_owned(),
        ),
        (
            CoverageError::SorryFound(2),
            "axiom-coverage: sorry-found: the Lean sources contain 2 sorry".to_owned(),
        ),
    ];
    for role in CoverageRole::ALL {
        expected.push((
            CoverageError::Io(role, IoFailure::NotFound),
            format!("axiom-coverage: coverage-io-failed: cannot access the {role}: not found"),
        ));
    }
    for (error, line) in expected {
        ensure(
            error.report() == line,
            format!("{error:?} printed {}", error.report()),
        )?;
    }
    Ok(())
}
