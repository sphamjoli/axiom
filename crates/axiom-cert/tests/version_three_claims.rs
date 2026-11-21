//! Version 3 certificates: `ConvergenceBound` claims about two-state chains
//! and `Termination` claims about retry models, written and read.

mod common;

use std::fs;
use std::path::PathBuf;

use axiom_cert::{
    Certificate, CertificateError, Claim, ConvergenceBoundError, FailureAfter, MAX_COUNT, Model,
};
use axiom_core::{Probability, TwoStateChain};
use common::{LINKS_FIXED_POINT, LINKS_MASS, TestError, link_chain, ratio};

/// The golden version 3 certificate for the link model.
const LINKS_CONVERGENCE: &str =
    include_str!("../../../fixtures/certificates/links-convergence.json");

/// The golden version 3 certificate for the retry model with success
/// probability 3/10.
const RETRY_TERMINATION: &str =
    include_str!("../../../fixtures/certificates/retry-termination.json");

/// Reads a shared fixture by its path below `fixtures/certificates/`.
fn fixture(path: &str) -> Result<String, TestError> {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/certificates")
        .join(path);
    Ok(fs::read_to_string(full)?)
}

/// The error from reading the fixture at `path`, which must be rejected.
fn rejection(path: &str) -> Result<CertificateError, TestError> {
    match Certificate::from_json(&fixture(path)?) {
        Ok(_) => Err(TestError::Accepted),
        Err(error) => Ok(error),
    }
}

#[test]
fn link_model_reproduces_the_convergence_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate =
        Certificate::two_state_with_bound(&link_chain()?, &Probability::one(), &ratio(1, 1000))?;
    assert_eq!(certificate.version(), 3);
    assert_eq!(certificate.to_json()?, LINKS_CONVERGENCE);
    Ok(())
}

#[test]
fn link_model_needs_thirteen_steps_from_one_to_within_one_thousandth() -> Result<(), TestError> {
    let certificate = Certificate::from_json(LINKS_CONVERGENCE)?;
    assert_eq!(
        certificate.claims().last(),
        Some(&Claim::ConvergenceBound {
            start: Probability::one(),
            steps: 13,
            tolerance: ratio(1, 1000),
        })
    );
    Ok(())
}

#[test]
fn twelve_steps_are_rejected_and_thirteen_accepted() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/convergence-bound-too-few-steps.json")?,
        CertificateError::FalseClaim { claim: 0 }
    );
    let thirteen = Certificate::from_json(&fixture("accept/convergence-bound-only.json")?)?;
    assert_eq!(thirteen.claims().len(), 1);
    Ok(())
}

#[test]
fn periodic_chain_convergence_bound_is_rejected_naming_the_claim() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/periodic-convergence-bound.json")?,
        CertificateError::FalseClaim { claim: 0 }
    );
    Ok(())
}

#[test]
fn periodic_chain_gets_no_convergence_bound_from_the_engine() {
    let always = Probability::one();
    let chain = TwoStateChain::new(always.clone(), always.clone());
    assert!(matches!(
        Certificate::two_state_with_bound(&chain, &always, &ratio(1, 1000)),
        Err(CertificateError::NoConvergenceBound(
            ConvergenceBoundError::Chain(_)
        ))
    ));
}

#[test]
fn slow_chain_beyond_the_step_limit_gets_no_convergence_bound() -> Result<(), TestError> {
    let slow = Probability::try_from_fraction(1, 1_000_000)?;
    let chain = TwoStateChain::new(slow.clone(), slow);
    assert!(matches!(
        Certificate::two_state_with_bound(&chain, &Probability::one(), &ratio(1, 1000)),
        Err(CertificateError::NoConvergenceBound(
            ConvergenceBoundError::NotReached
        ))
    ));
    Ok(())
}

#[test]
fn retry_model_reproduces_the_termination_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate = Certificate::termination(Probability::try_from_fraction(3, 10)?, 20)?;
    assert_eq!(certificate.version(), 3);
    assert_eq!(certificate.matrix(), None);
    assert_eq!(certificate.to_json()?, RETRY_TERMINATION);
    Ok(())
}

#[test]
fn retry_model_terminates_after_ten_thirds_attempts_on_average() -> Result<(), TestError> {
    let certificate = Certificate::from_json(RETRY_TERMINATION)?;
    assert_eq!(
        certificate.model(),
        &Model::Retry {
            success: Probability::try_from_fraction(3, 10)?
        }
    );
    assert_eq!(
        certificate.claims(),
        &[Claim::Termination {
            probability: Probability::one(),
            expected_attempts: Some(ratio(10, 3)),
            failure_after: Some(FailureAfter {
                attempts: 20,
                probability: Probability::try_from_fraction(7, 10)?.pow(20),
            }),
        }]
    );
    Ok(())
}

#[test]
fn three_expected_attempts_are_rejected() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/termination-wrong-expected-attempts.json")?,
        CertificateError::FalseClaim { claim: 0 }
    );
    Ok(())
}

#[test]
fn a_retry_that_never_succeeds_has_no_expected_attempts() -> Result<(), TestError> {
    let certificate = Certificate::termination(Probability::try_from_fraction(0, 1)?, 3)?;
    assert!(matches!(
        certificate.claims(),
        [Claim::Termination {
            expected_attempts: None,
            ..
        }]
    ));
    assert_eq!(
        rejection("reject/termination-expected-attempts-never-succeeds.json")?,
        CertificateError::FalseClaim { claim: 0 }
    );
    Ok(())
}

#[test]
fn claims_about_the_wrong_kind_of_model_are_rejected() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/retry-mass.json")?,
        CertificateError::NotChain { claim: 0 }
    );
    assert_eq!(
        rejection("reject/convergence-bound-on-retry.json")?,
        CertificateError::NotChain { claim: 0 }
    );
    assert_eq!(
        rejection("reject/termination-on-chain.json")?,
        CertificateError::NotRetry { claim: 0 }
    );
    Ok(())
}

#[test]
fn retry_models_need_version_three_and_exactly_one_model_form() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/v2-retry-model.json")?,
        CertificateError::ModelNeedsVersion { needed: 3 }
    );
    assert_eq!(
        rejection("reject/model-rows-and-retry.json")?,
        CertificateError::ModelForm
    );
    assert_eq!(
        rejection("reject/model-empty.json")?,
        CertificateError::ModelForm
    );
    Ok(())
}

#[test]
fn counts_above_the_limit_are_rejected() -> Result<(), TestError> {
    let limit = MAX_COUNT;
    assert_eq!(
        rejection("reject/convergence-bound-too-many-steps.json")?,
        CertificateError::CountOutOfRange { claim: 0, limit }
    );
    assert_eq!(
        rejection("reject/termination-too-many-attempts.json")?,
        CertificateError::CountOutOfRange { claim: 0, limit }
    );
    assert!(matches!(
        Certificate::termination(Probability::one(), MAX_COUNT + 1),
        Err(CertificateError::CountOutOfRange { .. })
    ));
    Ok(())
}

#[test]
fn versions_one_and_two_are_still_written_byte_for_byte() -> Result<(), TestError> {
    assert_eq!(Certificate::from_json(LINKS_MASS)?.to_json()?, LINKS_MASS);
    assert_eq!(
        Certificate::from_json(LINKS_FIXED_POINT)?.to_json()?,
        LINKS_FIXED_POINT
    );
    Ok(())
}
