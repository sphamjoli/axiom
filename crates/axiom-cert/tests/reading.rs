//! Reading certificates: every rule of the version 1 format is enforced.

mod common;

use axiom_cert::{Certificate, CertificateError, Claim, MAX_CERTIFICATE_BYTES};
use axiom_core::{ProbabilityError, TransitionRowError};
use common::{LINKS_MASS, TestError, link_chain};

/// A one-row certificate whose row has the given entries and whose single
/// claim refers to `row`.
fn certificate_text(entries: &str, row: usize) -> String {
    format!(
        r#"{{"format":"axiom-certificate","version":1,"model":{{"rows":[[{entries}]]}},"claims":[{{"kind":"Mass","row":{row}}}]}}"#
    )
}

fn halves() -> &'static str {
    r#"{"num":"1","den":"2"},{"num":"1","den":"2"}"#
}

#[test]
fn reads_the_golden_file_as_the_link_model() -> Result<(), TestError> {
    let certificate = Certificate::from_json(LINKS_MASS)?;
    assert_eq!(certificate.rows(), link_chain()?.rows().as_slice());
    assert_eq!(
        certificate.claims(),
        &[Claim::Mass { row: 0 }, Claim::Mass { row: 1 }]
    );
    Ok(())
}

#[test]
fn accepts_a_minimal_valid_certificate() -> Result<(), TestError> {
    let certificate = Certificate::from_json(&certificate_text(halves(), 0))?;
    assert_eq!(certificate.rows().len(), 1);
    Ok(())
}

#[test]
fn accepts_a_certificate_with_no_claims() -> Result<(), TestError> {
    let text = format!(
        r#"{{"format":"axiom-certificate","version":1,"model":{{"rows":[[{}]]}},"claims":[]}}"#,
        halves()
    );
    assert!(Certificate::from_json(&text)?.claims().is_empty());
    Ok(())
}

#[test]
fn rejects_an_unrelated_format() {
    let text = certificate_text(halves(), 0).replace("axiom-certificate", "other");
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::UnsupportedFormat(format)) if format == "other"
    ));
}

#[test]
fn rejects_versions_other_than_one() {
    for version in ["0", "2"] {
        let text = certificate_text(halves(), 0)
            .replace("\"version\":1", &format!("\"version\":{version}"));
        assert!(matches!(
            Certificate::from_json(&text),
            Err(CertificateError::UnsupportedVersion(_))
        ));
    }
}

#[test]
fn rejects_unknown_fields_at_every_level() {
    let base = certificate_text(halves(), 0);
    let variants = [
        base.replacen('{', r#"{"extra":1,"#, 1),
        base.replace(r#""rows":"#, r#""extra":1,"rows":"#),
        base.replacen(r#""den":"2""#, r#""den":"2","extra":1"#, 1),
        base.replace(r#""row":0"#, r#""row":0,"extra":1"#),
    ];
    for text in variants {
        assert!(
            matches!(
                Certificate::from_json(&text),
                Err(CertificateError::Json(_))
            ),
            "{text}"
        );
    }
}

#[test]
fn rejects_unknown_claim_kinds_and_missing_fields() {
    let unknown = certificate_text(halves(), 0).replace("\"Mass\"", "\"FixedPoint\"");
    assert!(matches!(
        Certificate::from_json(&unknown),
        Err(CertificateError::Json(_))
    ));
    let missing =
        certificate_text(halves(), 0).replace(r#","claims":[{"kind":"Mass","row":0}]"#, "");
    assert!(matches!(
        Certificate::from_json(&missing),
        Err(CertificateError::Json(_))
    ));
}

#[test]
fn rejects_fractions_that_are_not_canonical() {
    let non_canonical = [
        r#"{"num":"2","den":"4"},{"num":"1","den":"2"}"#,
        r#"{"num":"01","den":"2"},{"num":"1","den":"2"}"#,
        r#"{"num":"+1","den":"2"},{"num":"1","den":"2"}"#,
        r#"{"num":" 1","den":"2"},{"num":"1","den":"2"}"#,
        r#"{"num":"","den":"2"},{"num":"1","den":"2"}"#,
        r#"{"num":"1.0","den":"2"},{"num":"1","den":"2"}"#,
        r#"{"num":"0","den":"2"},{"num":"1","den":"1"}"#,
        r#"{"num":"1","den":"0"},{"num":"1","den":"2"}"#,
        r#"{"num":"-1","den":"2"},{"num":"3","den":"2"}"#,
        r#"{"num":"1","den":"２"},{"num":"1","den":"2"}"#,
    ];
    for entries in non_canonical {
        assert!(
            matches!(
                Certificate::from_json(&certificate_text(entries, 0)),
                Err(CertificateError::NonCanonicalFraction { .. })
            ),
            "{entries}"
        );
    }
}

#[test]
fn rejects_fractions_written_as_json_numbers() {
    let numbers = r#"{"num":1,"den":2},{"num":1,"den":2}"#;
    assert!(matches!(
        Certificate::from_json(&certificate_text(numbers, 0)),
        Err(CertificateError::Json(_))
    ));
}

#[test]
fn accepts_256_digits_and_rejects_257() -> Result<(), TestError> {
    let ten_to_255 = format!("1{}", "0".repeat(255));
    let nines = "9".repeat(255);
    let within =
        format!(r#"{{"num":"{nines}","den":"{ten_to_255}"}},{{"num":"1","den":"{ten_to_255}"}}"#);
    let certificate = Certificate::from_json(&certificate_text(&within, 0))?;
    assert_eq!(certificate.rows().len(), 1);
    let ten_to_256 = format!("1{}", "0".repeat(256));
    let beyond = format!(r#"{{"num":"1","den":"{ten_to_256}"}},{{"num":"1","den":"1"}}"#);
    assert!(matches!(
        Certificate::from_json(&certificate_text(&beyond, 0)),
        Err(CertificateError::NonCanonicalFraction { .. })
    ));
    Ok(())
}

#[test]
fn rejects_entries_above_one_with_their_position() {
    let entries = r#"{"num":"3","den":"2"},{"num":"1","den":"2"}"#;
    assert!(matches!(
        Certificate::from_json(&certificate_text(entries, 0)),
        Err(CertificateError::Probability {
            row: 0,
            entry: 0,
            source: ProbabilityError::AboveOne
        })
    ));
}

#[test]
fn rejects_rows_that_do_not_sum_to_one() {
    let entries = r#"{"num":"1","den":"2"},{"num":"1","den":"3"}"#;
    assert!(matches!(
        Certificate::from_json(&certificate_text(entries, 0)),
        Err(CertificateError::TransitionRow {
            row: 0,
            source: TransitionRowError::NotNormalised { .. }
        })
    ));
    assert!(matches!(
        Certificate::from_json(&certificate_text("", 0)),
        Err(CertificateError::TransitionRow {
            row: 0,
            source: TransitionRowError::Empty
        })
    ));
}

#[test]
fn rejects_an_empty_model() {
    let text = r#"{"format":"axiom-certificate","version":1,"model":{"rows":[]},"claims":[]}"#;
    assert!(matches!(
        Certificate::from_json(text),
        Err(CertificateError::EmptyModel)
    ));
}

#[test]
fn rejects_claims_about_missing_rows() {
    assert!(matches!(
        Certificate::from_json(&certificate_text(halves(), 1)),
        Err(CertificateError::RowOutOfRange {
            claim: 0,
            row: 1,
            rows: 1
        })
    ));
}

#[test]
fn rejects_oversized_text_before_parsing() {
    let oversized = " ".repeat(MAX_CERTIFICATE_BYTES + 1);
    assert!(matches!(
        Certificate::from_json(&oversized),
        Err(CertificateError::TooLarge { bytes, limit }) if bytes == MAX_CERTIFICATE_BYTES + 1 && limit == MAX_CERTIFICATE_BYTES
    ));
}

#[test]
fn rejects_trailing_data_and_malformed_json() {
    let trailing = format!("{} {{}}", certificate_text(halves(), 0));
    assert!(matches!(
        Certificate::from_json(&trailing),
        Err(CertificateError::Json(_))
    ));
    assert!(matches!(
        Certificate::from_json("not json"),
        Err(CertificateError::Json(_))
    ));
}

#[test]
fn every_error_names_what_went_wrong() {
    let message = CertificateError::RowOutOfRange {
        claim: 2,
        row: 5,
        rows: 3,
    }
    .to_string();
    assert_eq!(message, "claim 2 refers to row 5, but the model has 3 rows");
    assert_eq!(
        CertificateError::EmptyModel.to_string(),
        "model has no rows"
    );
    assert_eq!(
        CertificateError::UnsupportedVersion(2).to_string(),
        "unsupported certificate version 2"
    );
}
