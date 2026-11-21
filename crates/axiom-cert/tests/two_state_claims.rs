//! Version 2 certificates: `FixedPoint` and `Contraction` claims about
//! two-state models, written and read.

mod common;

use std::fs;
use std::path::PathBuf;

use axiom_cert::{Certificate, CertificateError, Claim};
use axiom_core::{Probability, TwoStateChain, TwoStateChainError};
use common::{LINKS_FIXED_POINT, LINKS_MASS, TestError, link_chain, ratio};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

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
fn link_model_reproduces_the_fixed_point_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate = Certificate::two_state(&link_chain()?)?;
    assert_eq!(certificate.version(), 2);
    assert_eq!(certificate.to_json()?, LINKS_FIXED_POINT);
    Ok(())
}

#[test]
fn link_model_claims_mass_steady_state_one_quarter_and_factor_three_fifths() -> Result<(), TestError>
{
    let certificate = Certificate::from_json(LINKS_FIXED_POINT)?;
    assert_eq!(
        certificate.claims(),
        &[
            Claim::Mass { row: 0 },
            Claim::Mass { row: 1 },
            Claim::FixedPoint {
                value: Probability::try_from_fraction(1, 4)?
            },
            Claim::Contraction {
                factor: ratio(3, 5)
            },
        ]
    );
    Ok(())
}

#[test]
fn mass_only_certificates_are_still_written_as_version_1() -> Result<(), TestError> {
    let certificate = Certificate::mass(link_chain()?.matrix());
    assert_eq!(certificate.version(), 1);
    assert_eq!(certificate.to_json()?, LINKS_MASS);
    Ok(())
}

#[test]
fn a_chain_that_never_moves_has_no_fixed_point_certificate() -> Result<(), TestError> {
    let stay = Probability::try_from_fraction(0, 1)?;
    assert_eq!(
        Certificate::two_state(&TwoStateChain::new(stay.clone(), stay)),
        Err(TwoStateChainError::NoUniqueFixedPoint)
    );
    Ok(())
}

#[test]
fn a_negative_factor_is_written_with_a_leading_minus() -> Result<(), TestError> {
    let three_quarters = Probability::try_from_fraction(3, 4)?;
    let written =
        Certificate::two_state(&TwoStateChain::new(three_quarters.clone(), three_quarters))?;
    assert!(written.to_json()?.contains(r#""num": "-1","#));
    assert_eq!(
        written,
        Certificate::from_json(&fixture("accept/negative-factor.json")?)?
    );
    Ok(())
}

#[test]
fn a_periodic_chain_has_no_contraction_certificate() -> Result<(), TestError> {
    let switch = Probability::try_from_fraction(1, 1)?;
    assert_eq!(
        Certificate::two_state(&TwoStateChain::new(switch.clone(), switch)),
        Err(TwoStateChainError::NotContracting)
    );
    Ok(())
}

#[test]
fn version_2_claim_kinds_are_rejected_in_version_1() -> Result<(), TestError> {
    for path in ["reject/v1-fixed-point.json", "reject/v1-contraction.json"] {
        assert_eq!(
            rejection(path)?,
            CertificateError::ClaimNeedsVersion {
                claim: 2,
                needed: 2
            }
        );
    }
    Ok(())
}

#[test]
fn two_state_claims_are_rejected_for_other_models() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/three-state-fixed-point.json")?,
        CertificateError::NotTwoState {
            claim: 0,
            states: 3
        }
    );
    assert_eq!(
        rejection("reject/one-state-contraction.json")?,
        CertificateError::NotTwoState {
            claim: 0,
            states: 1
        }
    );
    Ok(())
}

#[test]
fn non_canonical_claim_fractions_are_rejected() -> Result<(), TestError> {
    for path in [
        "reject/factor-negative-zero.json",
        "reject/factor-plus-sign.json",
        "reject/factor-double-minus.json",
        "reject/factor-negative-leading-zero.json",
        "reject/factor-negative-not-lowest-terms.json",
        "reject/factor-negative-denominator.json",
        "reject/factor-minus-only.json",
        "reject/fixed-point-negative-value.json",
        "reject/fixed-point-above-one.json",
        "reject/fixed-point-not-lowest-terms.json",
    ] {
        assert_eq!(
            rejection(path)?,
            CertificateError::InvalidClaimFraction { claim: 0 },
            "{path}"
        );
    }
    Ok(())
}

#[test]
fn false_claims_are_rejected() -> Result<(), TestError> {
    for path in [
        "reject/wrong-fixed-point.json",
        "reject/wrong-contraction-factor.json",
        "reject/contraction-factor-one.json",
        "reject/periodic-contraction.json",
    ] {
        assert_eq!(
            rejection(path)?,
            CertificateError::FalseClaim { claim: 2 },
            "{path}"
        );
    }
    Ok(())
}

#[test]
fn a_signed_entry_is_not_canonical_in_a_row() -> Result<(), TestError> {
    assert_eq!(
        rejection("reject/mass-with-signed-entry.json")?,
        CertificateError::NonCanonicalFraction { row: 0, entry: 0 }
    );
    Ok(())
}

fn any_probability() -> impl Strategy<Value = Probability> {
    (0_i64..=1_000_000, 1_i64..=1_000_000).prop_filter_map(
        "fraction must not exceed one",
        |(numerator, denominator)| Probability::try_from_fraction(numerator, denominator).ok(),
    )
}

proptest! {
    #[test]
    fn every_moving_two_state_certificate_round_trips(a in any_probability(), b in any_probability()) {
        if let Ok(written) = Certificate::two_state(&TwoStateChain::new(a, b)) {
            let text = written.to_json()?;
            let read = Certificate::from_json(&text)?;
            prop_assert_eq!(&read, &written);
            prop_assert_eq!(read.to_json()?, text);
        }
    }
}
