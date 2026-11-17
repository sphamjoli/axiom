//! Writing certificates: canonical output and round trips.

mod common;

use axiom_cert::{Certificate, CertificateError, Claim};
use axiom_core::{Probability, TransitionMatrix, TransitionRow, TwoStateChain};
use common::{LINKS_MASS, TestError, link_chain};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};
use serde_json::{Value, json};

#[test]
fn link_model_reproduces_the_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate = Certificate::mass(link_chain()?.matrix());
    assert_eq!(certificate.to_json()?, LINKS_MASS);
    Ok(())
}

#[test]
fn mass_claims_cover_every_row_in_order() -> Result<(), TestError> {
    let certificate = Certificate::mass(link_chain()?.matrix());
    assert_eq!(
        certificate.claims(),
        &[Claim::Mass { row: 0 }, Claim::Mass { row: 1 }]
    );
    Ok(())
}

#[test]
fn zero_and_one_are_written_canonically() -> Result<(), TestError> {
    let always_switch = Probability::try_from_fraction(1, 1)?;
    let matrix = TwoStateChain::new(always_switch.clone(), always_switch).matrix();
    let text = Certificate::mass(matrix).to_json()?;
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

#[test]
fn a_three_state_certificate_round_trips() -> Result<(), TestError> {
    let third = Probability::try_from_fraction(1, 3)?;
    let row = TransitionRow::try_new(vec![third.clone(), third.clone(), third])?;
    let written = Certificate::mass(TransitionMatrix::try_new(vec![
        row.clone(),
        row.clone(),
        row,
    ])?);
    assert_eq!(Certificate::from_json(&written.to_json()?)?, written);
    assert_eq!(written.claims().len(), 3);
    Ok(())
}

proptest! {
    #[test]
    fn every_two_state_certificate_round_trips(a in any_probability(), b in any_probability()) {
        let written = Certificate::mass(TwoStateChain::new(a, b).matrix());
        let text = written.to_json()?;
        let read = Certificate::from_json(&text)?;
        prop_assert_eq!(&read, &written);
        prop_assert_eq!(read.to_json()?, text);
    }
}
