//! Version 3 certificates: writing `ConvergenceBound` and `Termination`
//! claims, and reading the retry model. Which version 3 certificates the
//! reader accepts is covered by the shared fixtures; the arithmetic of the
//! step search is covered in `axiom-core`.

mod common;

use std::fs;
use std::path::PathBuf;

use axiom_cert::{
    Certificate, CertificateError, Claim, ConvergenceBoundError, FailureAfter, MAX_COUNT, Model,
};
use axiom_core::{Probability, TwoStateChain};
use common::{TestError, link_chain, ratio};
use num_bigint::BigInt;
use num_rational::BigRational;
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

/// The golden version 3 certificate for the link model, generated
/// independently of the engine.
const LINKS_CONVERGENCE: &str =
    include_str!("../../../fixtures/certificates/links-convergence.json");

/// The golden version 3 certificate for the retry model with success
/// probability 3/10, generated independently of the engine.
const RETRY_TERMINATION: &str =
    include_str!("../../../fixtures/certificates/retry-termination.json");

#[test]
fn link_model_reproduces_the_convergence_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate =
        Certificate::two_state_with_bound(&link_chain()?, &Probability::one(), &ratio(1, 1000))?;
    assert_eq!(certificate.version(), 3);
    assert_eq!(certificate.to_json()?, LINKS_CONVERGENCE);
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
fn non_positive_tolerance_gets_no_convergence_bound() -> Result<(), TestError> {
    assert!(matches!(
        Certificate::two_state_with_bound(&link_chain()?, &Probability::one(), &ratio(0, 1)),
        Err(CertificateError::NoConvergenceBound(
            ConvergenceBoundError::NotReached
        ))
    ));
    Ok(())
}

#[test]
fn periodic_chain_convergence_bound_is_rejected_naming_the_claim() -> Result<(), TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/certificates/reject/periodic-convergence-bound.json");
    assert_eq!(
        Certificate::from_json(&fs::read_to_string(path)?).err(),
        Some(CertificateError::FalseClaim { claim: 0 })
    );
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
fn retry_golden_file_reads_as_ten_thirds_attempts_and_seven_tenths_to_the_twentieth()
-> Result<(), TestError> {
    let certificate = Certificate::from_json(RETRY_TERMINATION)?;
    assert_eq!(
        certificate.model(),
        &Model::Retry {
            success: Probability::try_from_fraction(3, 10)?
        }
    );
    let seven_tenths_to_the_twentieth = BigRational::new(
        BigInt::from(79_792_266_297_612_001_u64),
        BigInt::from(10_u128.pow(20)),
    );
    assert_eq!(
        certificate.claims(),
        &[Claim::Termination {
            probability: Probability::one(),
            expected_attempts: Some(ratio(10, 3)),
            failure_after: Some(FailureAfter {
                attempts: 20,
                probability: Probability::try_new(&seven_tenths_to_the_twentieth)?,
            }),
        }]
    );
    Ok(())
}

#[test]
fn a_retry_that_never_succeeds_has_no_expected_attempts() -> Result<(), TestError> {
    let certificate = Certificate::termination(Probability::try_from_fraction(0, 1)?, 3)?;
    assert_eq!(
        certificate.claims(),
        &[Claim::Termination {
            probability: Probability::try_from_fraction(0, 1)?,
            expected_attempts: None,
            failure_after: Some(FailureAfter {
                attempts: 3,
                probability: Probability::one(),
            }),
        }]
    );
    Ok(())
}

#[test]
fn the_engine_refuses_failure_counts_above_the_limit() {
    assert!(Certificate::termination(Probability::one(), MAX_COUNT).is_ok());
    assert!(matches!(
        Certificate::termination(Probability::one(), MAX_COUNT + 1),
        Err(CertificateError::CountOutOfRange {
            claim: 0,
            limit: MAX_COUNT
        })
    ));
}

#[test]
fn failure_after_is_written_only_while_it_fits_sixty_four_digits() -> Result<(), TestError> {
    let failure_after = |denominator| -> Result<bool, TestError> {
        let success = Probability::try_from_fraction(1, denominator)?;
        Ok(matches!(
            Certificate::termination(success, 16)?.claims(),
            [Claim::Termination {
                failure_after: Some(_),
                ..
            }]
        ))
    };
    // 10^4 to the 16th power has 65 digits; 9999 to the 16th has 64.
    assert!(failure_after(9_999)?);
    assert!(!failure_after(10_000)?);
    Ok(())
}

fn any_probability() -> impl Strategy<Value = Probability> {
    (0_i64..=1_000, 1_i64..=1_000).prop_filter_map(
        "fraction must not exceed one",
        |(numerator, denominator)| Probability::try_from_fraction(numerator, denominator).ok(),
    )
}

proptest! {
    #[test]
    fn every_convergence_bound_certificate_round_trips(
        a in any_probability(),
        b in any_probability(),
        start in any_probability(),
        denominator in 1_i64..=1_000_000,
    ) {
        let tolerance = ratio(1, denominator);
        if let Ok(written) =
            Certificate::two_state_with_bound(&TwoStateChain::new(a, b), &start, &tolerance)
        {
            let text = written.to_json()?;
            let read = Certificate::from_json(&text)?;
            prop_assert_eq!(&read, &written);
            prop_assert_eq!(read.to_json()?, text);
        }
    }

    #[test]
    fn every_termination_certificate_round_trips(
        success in any_probability(),
        attempts in 0_u64..=MAX_COUNT,
    ) {
        let written = Certificate::termination(success, attempts)?;
        let text = written.to_json()?;
        let read = Certificate::from_json(&text)?;
        prop_assert_eq!(&read, &written);
        prop_assert_eq!(read.to_json()?, text);
    }
}
