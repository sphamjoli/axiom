//! Helpers shared by the integration tests.

#![allow(
    dead_code,
    reason = "each test binary compiles this module and uses only some of its items"
)]

use std::fs;
use std::path::PathBuf;

use axiom_core::{Probability, ProbabilityError};
use axiom_model::{Chain, CheckedModel, ModelError, Retry};
use num_bigint::BigInt;
use num_rational::BigRational;
use serde_json::Value;
use thiserror::Error;

/// A failure inside a test, so tests return `Result` and use `?` instead of
/// panicking on setup errors.
#[derive(Debug, Error)]
pub enum TestError {
    /// A fixture could not be read.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A fixture's expected results are not valid JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A model the test expects to be accepted was rejected.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// A fraction used by the test is not a probability.
    #[error(transparent)]
    Probability(#[from] ProbabilityError),
    /// An expected-results file lacks a field or has the wrong shape.
    #[error("expected-results field {0} is missing or malformed")]
    Field(&'static str),
    /// A model the test expects to be rejected was accepted.
    #[error("model was accepted but should be rejected")]
    Accepted,
    /// The model has a different form than the test expects.
    #[error("model has the wrong form")]
    WrongForm,
    /// A chain without parameters has no matrix.
    #[error("chain has no matrix")]
    NoMatrix,
}

/// The `fixtures/models` directory.
pub fn models_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/models")
}

/// The text of `fixtures/models/<name>`.
pub fn fixture(name: &str) -> Result<String, TestError> {
    Ok(fs::read_to_string(models_dir().join(name))?)
}

/// The parsed JSON of `fixtures/models/<name>`.
pub fn expected(name: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&fixture(name)?)?)
}

/// The exact rational `numerator / denominator`.
pub fn ratio(numerator: i64, denominator: i64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The rational written as `{"num": "...", "den": "..."}`.
pub fn fraction(value: &Value, field: &'static str) -> Result<BigRational, TestError> {
    let part = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .and_then(|text| BigInt::parse_bytes(text.as_bytes(), 10))
            .ok_or(TestError::Field(field))
    };
    let denominator = part("den")?;
    if denominator == BigInt::from(0) {
        return Err(TestError::Field(field));
    }
    Ok(BigRational::new(part("num")?, denominator))
}

/// The field `key` of `value`.
pub fn field<'a>(value: &'a Value, key: &'static str) -> Result<&'a Value, TestError> {
    value.get(key).ok_or(TestError::Field(key))
}

/// The array field `key` of `value`.
pub fn array<'a>(value: &'a Value, key: &'static str) -> Result<&'a [Value], TestError> {
    field(value, key)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or(TestError::Field(key))
}

/// Parses `source`, expecting a chain.
pub fn chain(source: &str) -> Result<Chain, TestError> {
    match axiom_model::parse(source)? {
        CheckedModel::Chain(chain) => Ok(chain),
        CheckedModel::Retry(_) => Err(TestError::WrongForm),
    }
}

/// Parses `source`, expecting a retry.
pub fn retry(source: &str) -> Result<Retry, TestError> {
    match axiom_model::parse(source)? {
        CheckedModel::Retry(retry) => Ok(retry),
        CheckedModel::Chain(_) => Err(TestError::WrongForm),
    }
}

/// Parses `source`, expecting a rejection.
pub fn rejection(source: &str) -> Result<ModelError, TestError> {
    match axiom_model::parse(source) {
        Ok(_) => Err(TestError::Accepted),
        Err(error) => Ok(error),
    }
}

/// The matrix of a chain without parameters, as exact rationals.
pub fn matrix(chain: &Chain) -> Result<Vec<Vec<BigRational>>, TestError> {
    let matrix = chain.matrix.as_ref().ok_or(TestError::NoMatrix)?;
    Ok(matrix
        .rows()
        .iter()
        .map(|row| {
            row.probabilities()
                .iter()
                .map(|probability| probability.value().clone())
                .collect()
        })
        .collect())
}

/// The probability `numerator / denominator`.
pub fn probability(numerator: i64, denominator: i64) -> Result<Probability, ProbabilityError> {
    Probability::try_from_fraction(numerator, denominator)
}
