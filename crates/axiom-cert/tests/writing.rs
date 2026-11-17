//! Writing certificates: canonical output and round trips.

mod common;

use axiom_cert::{Certificate, CertificateError, Claim};
use axiom_core::{Probability, TransitionRow, TwoStateChain};
use common::{LINKS_MASS, TestError, link_chain};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};
use serde_json::{Value, json};

#[test]
fn link_model_reproduces_the_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate = Certificate::mass(link_chain()?.rows().to_vec())?;
    assert_eq!(certificate.to_json()?, LINKS_MASS);
    Ok(())
}

#[test]
fn mass_claims_cover_every_row_in_order() -> Result<(), TestError> {
    let certificate = Certificate::mass(link_chain()?.rows().to_vec())?;
    assert_eq!(
        certificate.claims(),
        &[Claim::Mass { row: 0 }, Claim::Mass { row: 1 }]
    );
    Ok(())
}

#[test]
fn refuses_to_certify_an_empty_model() {
    assert!(matches!(
        Certificate::mass(Vec::new()),
        Err(CertificateError::EmptyModel)
    ));
}

#[test]
fn zero_and_one_are_written_canonically() -> Result<(), TestError> {
    let certain = TransitionRow::two_state(Probability::try_from_fraction(1, 1)?);
    let text = Certificate::mass(vec![certain])?.to_json()?;
    let document: Value = serde_json::from_str(&text).map_err(CertificateError::from)?;
    assert_eq!(
        document["model"]["rows"][0],
        json!([{ "num": "0", "den": "1" }, { "num": "1", "den": "1" }])
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
    fn every_two_state_certificate_round_trips(a in any_probability(), b in any_probability()) {
        let written = Certificate::mass(TwoStateChain::new(a, b).rows().to_vec())?;
        let text = written.to_json()?;
        let read = Certificate::from_json(&text)?;
        prop_assert_eq!(&read, &written);
        prop_assert_eq!(read.to_json()?, text);
    }
}
