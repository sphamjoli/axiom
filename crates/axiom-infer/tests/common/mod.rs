//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use std::path::PathBuf;

use axiom_infer::{InferError, MatrixBox};
use axiom_model::{Chain, CheckedModel, ModelError};
use num_bigint::BigInt;
use num_rational::BigRational;
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?`.
#[derive(Debug, Error)]
pub enum TestError {
    /// A fixture model was rejected.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// The analysis refused its input.
    #[error(transparent)]
    Infer(#[from] InferError),
    /// The simulator refused its input.
    #[error(transparent)]
    Sim(#[from] axiom_sim::SimError),
    /// A fixture file could not be read.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A result differs from the expected one.
    #[error("{0}")]
    Mismatch(String),
}

/// Fails with `message` unless `condition` holds.
pub fn ensure(condition: bool, message: impl Into<String>) -> Result<(), TestError> {
    if condition {
        Ok(())
    } else {
        Err(TestError::Mismatch(message.into()))
    }
}

/// The rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The checked chain of the model `source`.
pub fn chain_of(source: &str) -> Result<Chain, TestError> {
    match axiom_model::parse(source)? {
        CheckedModel::Chain(chain) => Ok(chain),
        CheckedModel::Retry(_) => Err(TestError::Mismatch("not a chain".into())),
    }
}

/// The checked chain of the shared model fixture `name`.
pub fn fixture_chain(name: &str) -> Result<Chain, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/models")
        .join(name);
    chain_of(&std::fs::read_to_string(path)?)
}

/// The exact matrix of a chain without parameters.
pub fn matrix_of(chain: &Chain) -> Result<axiom_core::TransitionMatrix, TestError> {
    chain
        .matrix
        .clone()
        .ok_or(TestError::Mismatch("the chain has parameters".into()))
}

/// The black box of the shared model fixture `name`.
pub fn fixture_box(name: &str) -> Result<MatrixBox, TestError> {
    Ok(MatrixBox::new(&matrix_of(&fixture_chain(name)?)?))
}
