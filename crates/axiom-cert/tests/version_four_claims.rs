//! Version 4 certificates: writing the parametric model and its
//! `ParametricContraction` claim. Which version 4 certificates the reader
//! accepts or rejects is covered by the shared fixtures.

mod common;

use axiom_cert::{Certificate, CertificateError, End, Interval};
use axiom_core::Probability;
use common::TestError;
use proptest::prelude::{Strategy, prop_assert_eq, prop_oneof, proptest};

/// The golden version 4 certificate for the links model with `a` and `b` in
/// `]0, 1[`, generated independently of the engine.
const LINKS_PARAM: &str = include_str!("../../../fixtures/certificates/links-param.json");

/// The interval `lo..hi` of tenths with the given ends.
fn tenths(lo: i64, lo_end: End, hi: i64, hi_end: End) -> Result<Interval, TestError> {
    Ok(Interval::try_new(
        Probability::try_from_fraction(lo, 10)?,
        lo_end,
        Probability::try_from_fraction(hi, 10)?,
        hi_end,
    )?)
}

#[test]
fn open_unit_ranges_reproduce_the_golden_file_byte_for_byte() -> Result<(), TestError> {
    let open = tenths(0, End::Open, 10, End::Open)?;
    let certificate = Certificate::parametric(open.clone(), open)?;
    assert_eq!(certificate.version(), 4);
    assert_eq!(certificate.to_json()?, LINKS_PARAM);
    Ok(())
}

#[test]
fn the_engine_refuses_a_range_that_reaches_a_sum_of_zero_or_two() -> Result<(), TestError> {
    // Each pair reaches its extreme only because both ends there are closed;
    // opening either end is accepted, which the shared fixtures cover.
    let cases = [
        (
            tenths(0, End::Closed, 5, End::Closed)?,
            tenths(0, End::Closed, 5, End::Closed)?,
        ),
        (
            tenths(5, End::Closed, 10, End::Closed)?,
            tenths(5, End::Closed, 10, End::Closed)?,
        ),
    ];
    for (a, b) in cases {
        assert_eq!(
            Certificate::parametric(a, b),
            Err(CertificateError::NoParametricContraction)
        );
    }
    Ok(())
}

#[test]
fn an_interval_holds_at_least_one_value() -> Result<(), TestError> {
    let half = || Probability::try_from_fraction(1, 2);
    assert!(Interval::try_new(half()?, End::Closed, half()?, End::Closed).is_ok());
    for (lo_end, hi_end) in [(End::Open, End::Closed), (End::Closed, End::Open)] {
        assert_eq!(
            Interval::try_new(half()?, lo_end, half()?, hi_end),
            Err(CertificateError::EmptyInterval)
        );
    }
    assert!(matches!(
        tenths(6, End::Closed, 4, End::Closed),
        Err(TestError::Certificate(CertificateError::EmptyInterval))
    ));
    Ok(())
}

/// An interval of tenths with arbitrary ends, if it is not empty.
fn interval() -> impl Strategy<Value = Option<Interval>> {
    let end = || {
        prop_oneof![
            proptest::strategy::Just(End::Open),
            proptest::strategy::Just(End::Closed)
        ]
    };
    (0_i64..=10, 0_i64..=10, end(), end())
        .prop_map(|(lo, hi, lo_end, hi_end)| tenths(lo, lo_end, hi, hi_end).ok())
}

proptest! {
    #[test]
    fn parametric_certificates_round_trip(a in interval(), b in interval()) {
        if let (Some(a), Some(b)) = (a, b)
            && let Ok(certificate) = Certificate::parametric(a, b)
        {
            let text = certificate.to_json().map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            let read = Certificate::from_json(&text).map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(read, certificate);
        }
    }
}
