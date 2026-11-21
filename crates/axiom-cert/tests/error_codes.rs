//! Every error of the crate has a stable, distinct code and a fixed message
//! built from indices, counts and limits only. The table pins both, so a
//! renamed code or a reworded message fails here. That hostile certificate
//! text never reaches a message is tested in `reading.rs`.

use std::collections::BTreeSet;

use axiom_cert::{CertificateError, ConvergenceBoundError};
use axiom_core::{ProbabilityError, TransitionMatrixError, TransitionRowError, TwoStateChainError};

fn every_certificate_error() -> Vec<CertificateError> {
    vec![
        CertificateError::TooLarge { bytes: 9, limit: 8 },
        CertificateError::Json { line: 2, column: 3 },
        CertificateError::JsonLimit,
        CertificateError::UnsupportedFormat,
        CertificateError::UnsupportedVersion(5),
        CertificateError::TooManyStates {
            states: 5,
            limit: 4,
        },
        CertificateError::NonCanonicalFraction { row: 1, entry: 0 },
        CertificateError::Probability {
            row: 1,
            entry: 0,
            source: ProbabilityError::AboveOne,
        },
        CertificateError::TransitionRow {
            row: 1,
            source: TransitionRowError::NotNormalised,
        },
        CertificateError::TransitionMatrix(TransitionMatrixError::Empty),
        CertificateError::NoClaims,
        CertificateError::DuplicateClaim { claim: 1, first: 0 },
        CertificateError::InvalidClaimFraction { claim: 2 },
        CertificateError::ClaimNeedsVersion {
            claim: 2,
            needed: 3,
        },
        CertificateError::FalseClaim { claim: 2 },
        CertificateError::NotTwoState {
            claim: 2,
            states: 3,
        },
        CertificateError::ModelForm,
        CertificateError::ModelNeedsVersion { needed: 3 },
        CertificateError::InvalidInterval,
        CertificateError::EmptyInterval,
        CertificateError::NoParametricContraction,
        CertificateError::NotParametric { claim: 2 },
        CertificateError::InvalidRetrySuccess,
        CertificateError::NotChain { claim: 2 },
        CertificateError::NotRetry { claim: 2 },
        CertificateError::CountOutOfRange { claim: 2, limit: 7 },
        CertificateError::NoConvergenceBound(ConvergenceBoundError::NotReached),
        CertificateError::RowOutOfRange {
            claim: 2,
            row: 5,
            rows: 3,
        },
    ]
}

#[test]
fn every_certificate_error_has_a_stable_distinct_code_and_fixed_message() {
    let errors = every_certificate_error();
    let found: Vec<(&str, String)> = errors
        .iter()
        .map(|error| (error.code(), error.to_string()))
        .collect();
    let expected = [
        (
            "certificate-too-large",
            "certificate is 9 bytes; the limit is 8",
        ),
        (
            "invalid-json",
            "certificate is not valid certificate JSON (line 2, column 3)",
        ),
        (
            "json-limit",
            "certificate JSON exceeds a nesting, member or item limit",
        ),
        (
            "unsupported-format",
            "certificate format is not axiom-certificate",
        ),
        ("unsupported-version", "unsupported certificate version 5"),
        ("too-many-states", "model has 5 states; the limit is 4"),
        (
            "non-canonical-fraction",
            "row 1, entry 0: fraction is not in canonical form",
        ),
        (
            "entry-not-probability",
            "row 1, entry 0: probability is above one",
        ),
        ("invalid-row", "row 1: transition row does not sum to 1"),
        ("invalid-matrix", "transition matrix has no rows"),
        ("no-claims", "certificate makes no claims"),
        ("duplicate-claim", "claim 1 repeats claim 0"),
        (
            "invalid-claim-fraction",
            "claim 2: fraction is not in canonical form or out of range",
        ),
        ("claim-needs-version", "claim 2 needs certificate version 3"),
        ("false-claim", "claim 2 does not hold for the model"),
        (
            "not-two-state",
            "claim 2 needs a two-state model, but the model has 3 states",
        ),
        (
            "model-form",
            "model must have exactly one of rows, retry and parametric",
        ),
        (
            "model-needs-version",
            "model kind needs certificate version 3",
        ),
        (
            "invalid-interval",
            "interval end is not a canonical probability",
        ),
        ("empty-interval", "interval is empty"),
        (
            "no-parametric-contraction",
            "the parameter range includes a chain that does not contract",
        ),
        ("not-parametric", "claim 2 needs a parametric model"),
        (
            "invalid-retry-success",
            "retry success is not a canonical probability",
        ),
        ("not-chain", "claim 2 needs a chain model"),
        ("not-retry", "claim 2 needs a retry model"),
        ("count-out-of-range", "claim 2: count is above the limit 7"),
        (
            "no-convergence-bound",
            "no convergence bound can be certified for the chain",
        ),
        (
            "row-out-of-range",
            "claim 2 refers to row 5, but the model has 3 rows",
        ),
    ];
    let expected: Vec<(&str, String)> = expected
        .iter()
        .map(|(code, message)| (*code, (*message).to_owned()))
        .collect();
    assert_eq!(found, expected);
    let codes: BTreeSet<&str> = found.iter().map(|(code, _)| *code).collect();
    assert_eq!(codes.len(), found.len(), "codes are distinct");
}

#[test]
fn a_convergence_bound_error_keeps_the_chain_code() {
    let codes = [
        ConvergenceBoundError::Chain(TwoStateChainError::NotContracting).code(),
        ConvergenceBoundError::NotReached.code(),
    ];
    assert_eq!(codes, ["not-contracting", "tolerance-not-reached"]);
    assert_eq!(
        ConvergenceBoundError::NotReached.to_string(),
        "tolerance is not reached within the step limit"
    );
}
