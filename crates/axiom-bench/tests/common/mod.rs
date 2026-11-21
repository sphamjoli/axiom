//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use std::path::PathBuf;

use axiom_bench::engine::{self, Built};
use axiom_cert::Certificate;
use num_bigint::BigInt;
use num_rational::BigRational;
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?` instead of
/// panicking.
#[derive(Debug, Error)]
pub enum TestError {
    /// A file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The harness failed.
    #[error(transparent)]
    Bench(#[from] axiom_bench::error::BenchError),
    /// A result is not what the test expects.
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

/// `n / d`.
pub fn fraction(n: u64, d: u64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

/// The certificate the engine builds for `source` with `tolerance`.
pub fn certificate(source: &str, tolerance: &BigRational) -> Result<Certificate, TestError> {
    match engine::build(source, tolerance) {
        Built::Certificate(certificate) => Ok(certificate),
        Built::Refused(code) => Err(TestError::Unexpected(format!("engine refused: {code}"))),
    }
}
