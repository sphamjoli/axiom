//! Tests of `analyse` on the shared model fixtures. They cover the wiring
//! only, the JSON shape, the claim statuses and the error mapping; the
//! values come from `axiom-core` and `axiom-cert` and are tested there.

use std::path::PathBuf;

use axiom_cert::{Certificate, CertificateError};
use axiom_model::{CheckedModel, ModelError};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
enum TestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Certificate(#[from] CertificateError),
    #[error("{0}")]
    Failed(String),
}

fn ensure(condition: bool, message: &str) -> Result<(), TestError> {
    if condition {
        Ok(())
    } else {
        Err(TestError::Failed(message.to_owned()))
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/models")
        .join(name)
}

fn analyse_fixture(name: &str) -> Result<(String, Value), TestError> {
    let source = std::fs::read_to_string(fixture(name))?;
    let value = serde_json::from_str(&axiom_wasm::analyse(&source))?;
    Ok((source, value))
}

/// A converging chain fills the chain, convergence and certificate sections
/// and leaves the retry section null. The series runs from step 0 to seven
/// steps past the horizon of 13, so it has 21 entries.
#[test]
fn links_analysis_has_the_converging_chain_shape() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("links.axm")?;
    ensure(result["ok"] == true, "links was rejected")?;
    let analysis = &result["analysis"];
    ensure(analysis["chain"].is_object(), "chain section")?;
    ensure(analysis["certificate"].is_object(), "certificate section")?;
    ensure(analysis["retry"].is_null(), "no retry")?;
    let convergence = &analysis["convergence"];
    ensure(convergence["horizon"] == 13, "horizon")?;
    ensure(
        convergence["series"].as_array().map(Vec::len) == Some(21),
        "series length",
    )
}

/// The page shows exactly the certificate `axiom-cert` writes for the model,
/// with every claim unchecked because the Lean checker has not run.
#[test]
fn links_certificate_is_the_strongest_one_and_unchecked() -> Result<(), TestError> {
    let (source, result) = analyse_fixture("links.axm")?;
    let CheckedModel::Chain(chain) = axiom_model::parse(&source)? else {
        return Err(TestError::Failed(String::from("links is not a chain")));
    };
    let matrix = chain
        .matrix
        .ok_or_else(|| TestError::Failed(String::from("links has no matrix")))?;
    let wanted = Certificate::strongest(matrix).to_json()?;
    let certificate = &result["analysis"]["certificate"];
    ensure(certificate["json"] == wanted.as_str(), "certificate json")?;
    let claims = certificate["claims"]
        .as_array()
        .ok_or_else(|| TestError::Failed(String::from("claims")))?;
    ensure(!claims.is_empty(), "claims are listed")?;
    ensure(
        claims.iter().all(|claim| claim["status"] == "unchecked"),
        "every claim unchecked",
    )
}

/// Every claim the engine writes has a statement the page can show: none
/// falls through to the generic text, for chains and for retries.
#[test]
fn every_claim_kind_has_its_own_statement() -> Result<(), TestError> {
    for (name, kind) in [
        ("links.axm", "convergence-bound"),
        ("retry.axm", "termination"),
    ] {
        let (_, result) = analyse_fixture(name)?;
        let claims = result["analysis"]["certificate"]["claims"]
            .as_array()
            .ok_or_else(|| TestError::Failed(format!("{name}: claims")))?;
        ensure(
            claims.iter().any(|claim| claim["id"] == kind),
            &format!("{name}: {kind} claim shown"),
        )?;
        ensure(
            claims.iter().all(|claim| {
                !claim["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("claim-"))
            }),
            &format!("{name}: no claim falls through"),
        )?;
    }
    Ok(())
}

#[test]
fn retry_fills_the_retry_section_only() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("retry.axm")?;
    ensure(result["ok"] == true, "retry was rejected")?;
    let retry = &result["analysis"]["retry"];
    for key in ["success", "successProbability", "expectedAttempts"] {
        ensure(retry[key]["num"].is_string(), key)?;
        ensure(retry[key]["den"].is_string(), key)?;
    }
    ensure(result["analysis"]["chain"].is_null(), "no chain")
}

#[test]
fn periodic_gets_mass_claims_only() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("periodic.axm")?;
    ensure(result["ok"] == true, "periodic was rejected")?;
    let analysis = &result["analysis"];
    ensure(analysis["convergence"].is_null(), "no convergence")?;
    ensure(analysis["certificate"]["version"] == 1, "version 1")?;
    ensure(
        analysis["certificate"]["claims"].as_array().map(Vec::len) == Some(2),
        "two mass claims",
    )
}

#[test]
fn reject_maps_the_error_code_and_position() -> Result<(), TestError> {
    let (_, result) = analyse_fixture("reject/missing-row.axm")?;
    ensure(result["ok"] == false, "missing row was accepted")?;
    let diagnostic = &result["diagnostics"][0];
    ensure(diagnostic["code"] == "missing-row", "code")?;
    ensure(diagnostic["line"] == 2, "line")?;
    ensure(diagnostic["column"] == 14, "column")
}
