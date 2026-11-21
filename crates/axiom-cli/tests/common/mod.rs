//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use std::path::PathBuf;
use std::process::{Command, Output};

use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?` instead of
/// panicking.
#[derive(Debug, Error)]
pub enum TestError {
    /// A file could not be read or written, or the binary could not run.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The binary's output is not what the test expects.
    #[error("{0}")]
    Unexpected(String),
}

/// Fails with `why` unless `ok` holds.
pub fn ensure(ok: bool, why: impl Into<String>) -> Result<(), TestError> {
    if ok {
        Ok(())
    } else {
        Err(TestError::Unexpected(why.into()))
    }
}

/// The repository root.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch directory for one test.
pub fn scratch(name: &str) -> Result<PathBuf, TestError> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Runs the `axiom` binary with `args`.
pub fn axiom(args: &[&std::ffi::OsStr]) -> Result<Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_axiom"))
        .args(args)
        .output()?)
}

/// Whether `lake` can be run. When it cannot, prints why the calling test is
/// skipped, or fails when the `CI` environment variable is set, so a CI job
/// never passes these tests by skipping them.
pub fn lake_available(test: &str) -> Result<bool, TestError> {
    let found = Command::new("lake")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !found {
        if std::env::var_os("CI").is_some() {
            return Err(TestError::Unexpected(format!(
                "{test}: lake is not on PATH and CI is set"
            )));
        }
        eprintln!("skipping {test}: lake is not on PATH, so the Lean checker cannot run");
    }
    Ok(found)
}
