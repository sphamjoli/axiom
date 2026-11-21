//! Version 4 certificates: writing the parametric model, its
//! `ParametricContraction` claim and `Estimate` claims. Which version 4
//! certificates the reader accepts or rejects is covered by the shared
//! fixtures; the Hoeffding decision itself is covered in `axiom-sim`.

mod common;

use axiom_cert::{Certificate, CertificateError, End, Estimate, Interval, MAX_SAMPLES, Quantity};
use axiom_core::{Probability, TransitionMatrix, TransitionRow};
use common::{TestError, ratio};
use proptest::prelude::{Strategy, prop_assert_eq, prop_oneof, proptest};

/// The golden version 4 certificate for the links model with `a` and `b` in
/// `]0, 1[`, generated independently of the engine.
const LINKS_PARAM: &str = include_str!("../../../fixtures/certificates/links-param.json");

/// The golden version 4 certificate for the links model with a steady-state
/// estimate from seed 42, generated independently of the engine.
const LINKS_ESTIMATE: &str = include_str!("../../../fixtures/certificates/links-estimate.json");

/// The links model's transition matrix.
fn links() -> Result<TransitionMatrix, TestError> {
    let row = |stay: i64, leave: i64, first_stays: bool| -> Result<TransitionRow, TestError> {
        let (first, second) = if first_stays {
            (stay, leave)
        } else {
            (leave, stay)
        };
        Ok(TransitionRow::try_new(vec![
            Probability::try_from_fraction(first, 10)?,
            Probability::try_from_fraction(second, 10)?,
        ])?)
    };
    Ok(TransitionMatrix::try_new(vec![
        row(9, 1, true)?,
        row(7, 3, false)?,
    ])?)
}

/// A steady-state estimate with the golden file's parameters and `samples`.
fn estimate(samples: u64) -> Result<Estimate, TestError> {
    Ok(Estimate {
        quantity: Quantity::SteadyState,
        value: Probability::try_new(&ratio(9583, 38005))?,
        samples,
        epsilon: ratio(1, 100),
        delta: ratio(1, 1000),
        seed: 42,
    })
}

#[test]
fn a_steady_state_estimate_reproduces_the_golden_file_byte_for_byte() -> Result<(), TestError> {
    let certificate = Certificate::strongest(links()?).with_estimate(estimate(38_005)?)?;
    assert_eq!(certificate.version(), 4);
    assert_eq!(certificate.to_json()?, LINKS_ESTIMATE);
    Ok(())
}

#[test]
fn the_engine_refuses_an_estimate_the_reader_would_reject() -> Result<(), TestError> {
    let links = || -> Result<Certificate, TestError> { Ok(Certificate::strongest(links()?)) };
    assert_eq!(
        links()?.with_estimate(estimate(38_004)?),
        Err(CertificateError::FalseClaim { claim: 5 })
    );
    assert_eq!(
        links()?.with_estimate(estimate(MAX_SAMPLES + 1)?),
        Err(CertificateError::SamplesOutOfRange {
            claim: 5,
            limit: MAX_SAMPLES
        })
    );
    let success = Estimate {
        quantity: Quantity::Success,
        ..estimate(38_005)?
    };
    assert_eq!(
        links()?.with_estimate(success),
        Err(CertificateError::NotRetry { claim: 5 })
    );
    Ok(())
}

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
    fn estimate_certificates_round_trip(
        hits in 0_u64..=1000,
        extra in 0_u64..=1000,
        seed in proptest::num::u64::ANY,
    ) {
        let fail = |e: TestError| proptest::test_runner::TestCaseError::fail(e.to_string());
        let samples = 38_005 + extra;
        let claim = Estimate {
            quantity: Quantity::Success,
            value: Probability::try_new(&ratio(i64::try_from(hits).unwrap_or(0), 1000)).map_err(|e| fail(e.into()))?,
            samples,
            epsilon: ratio(1, 100),
            delta: ratio(1, 1000),
            seed,
        };
        let success = Probability::try_from_fraction(3, 10).map_err(|e| fail(e.into()))?;
        let certificate = Certificate::termination(success, 20)
            .and_then(|c| c.with_estimate(claim))
            .map_err(|e| fail(e.into()))?;
        let text = certificate.to_json().map_err(|e| fail(e.into()))?;
        let read = Certificate::from_json(&text).map_err(|e| fail(e.into()))?;
        prop_assert_eq!(read, certificate);
    }

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

/// A model's bounds become the interval with the same values and ends; an
/// end that is not a probability is refused before the emptiness check.
#[test]
fn bounds_become_an_interval_with_the_same_ends() -> Result<(), TestError> {
    assert_eq!(
        Interval::from_bounds(&ratio(1, 10), true, &ratio(3, 10), false)?,
        tenths(1, End::Closed, 3, End::Open)?
    );
    assert_eq!(
        Interval::from_bounds(&ratio(0, 1), false, &ratio(3, 2), true),
        Err(CertificateError::InvalidInterval)
    );
    assert_eq!(
        Interval::from_bounds(&ratio(1, 2), true, &ratio(1, 2), false),
        Err(CertificateError::EmptyInterval)
    );
    Ok(())
}
