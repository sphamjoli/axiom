//! Reading certificates: every rule of the version 1 and 2 formats is enforced.

mod common;

use axiom_cert::{Certificate, CertificateError, Claim, MAX_CERTIFICATE_BYTES, MAX_STATES};
use axiom_core::{ProbabilityError, TransitionMatrix, TransitionMatrixError, TransitionRowError};
use common::{LINKS_MASS, TestError, link_chain};

/// A two-state certificate whose first row has the given entries, whose
/// second row is `[1/2, 1/2]`, and whose single claim refers to `row`.
fn certificate_text(entries: &str, row: usize) -> String {
    format!(
        r#"{{"format":"axiom-certificate","version":1,"model":{{"rows":[[{entries}],[{}]]}},"claims":[{{"kind":"Mass","row":{row}}}]}}"#,
        halves()
    )
}

fn halves() -> &'static str {
    r#"{"num":"1","den":"2"},{"num":"1","den":"2"}"#
}

#[test]
fn reads_the_golden_file_as_the_link_model() -> Result<(), TestError> {
    let certificate = Certificate::from_json(LINKS_MASS)?;
    assert_eq!(certificate.matrix(), Some(&link_chain()?.matrix()));
    assert_eq!(
        certificate.claims(),
        &[Claim::Mass { row: 0 }, Claim::Mass { row: 1 }]
    );
    Ok(())
}

#[test]
fn accepts_a_minimal_valid_certificate() -> Result<(), TestError> {
    let certificate = Certificate::from_json(&certificate_text(halves(), 0))?;
    assert_eq!(certificate.matrix().map(TransitionMatrix::states), Some(2));
    Ok(())
}

#[test]
fn rejects_a_certificate_with_no_claims() {
    let text = certificate_text(halves(), 0).replace(r#"{"kind":"Mass","row":0}"#, "");
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::NoClaims)
    ));
}

#[test]
fn rejects_a_repeated_claim() {
    let text = certificate_text(halves(), 0).replace(
        r#"{"kind":"Mass","row":0}"#,
        r#"{"kind":"Mass","row":0},{"kind":"Mass","row":0}"#,
    );
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::DuplicateClaim { claim: 1, first: 0 })
    ));
}

#[test]
fn rejects_a_repeated_key() {
    let text =
        certificate_text(halves(), 0).replace("\"version\":1", "\"version\":1,\"version\":1");
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::Json { .. })
    ));
}

#[test]
fn ignores_whitespace_and_key_order() -> Result<(), TestError> {
    let reordered = format!(
        "{{ \"claims\": [ {{ \"row\": 0, \"kind\": \"Mass\" }} ],\n  \"model\": {{ \"rows\": [ [ {h} ], [ {h} ] ] }}, \"version\": 1, \"format\": \"axiom-certificate\" }}",
        h = halves()
    );
    assert_eq!(
        Certificate::from_json(&reordered)?,
        Certificate::from_json(&certificate_text(halves(), 0))?
    );
    Ok(())
}

#[test]
fn accepts_zero_written_as_zero_over_one() -> Result<(), TestError> {
    let entries = r#"{"num":"0","den":"1"},{"num":"1","den":"1"}"#;
    assert_eq!(
        Certificate::from_json(&certificate_text(entries, 0))?
            .matrix()
            .map(TransitionMatrix::states),
        Some(2)
    );
    Ok(())
}

#[test]
fn rejects_a_matrix_that_is_not_square() {
    let text = certificate_text(r#"{"num":"1","den":"1"}"#, 0);
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::TransitionMatrix(
            TransitionMatrixError::NotSquare {
                row: 0,
                entries: 1,
                states: 2
            }
        ))
    ));
}

#[test]
fn rejects_an_unrelated_format() {
    let text = certificate_text(halves(), 0).replace("axiom-certificate", "other");
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::UnsupportedFormat)
    ));
}

#[test]
fn rejects_versions_other_than_one_to_three() {
    for version in ["0", "4"] {
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
                Err(CertificateError::Json { .. })
            ),
            "{text}"
        );
    }
}

#[test]
fn rejects_unknown_claim_kinds_and_missing_fields() {
    let unknown = certificate_text(halves(), 0).replace("\"Mass\"", "\"Unknown\"");
    assert!(matches!(
        Certificate::from_json(&unknown),
        Err(CertificateError::Json { .. })
    ));
    let missing =
        certificate_text(halves(), 0).replace(r#","claims":[{"kind":"Mass","row":0}]"#, "");
    assert!(matches!(
        Certificate::from_json(&missing),
        Err(CertificateError::Json { .. })
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
        Err(CertificateError::Json { .. })
    ));
}

#[test]
fn accepts_64_digits_and_rejects_65() -> Result<(), TestError> {
    let ten_to_63 = format!("1{}", "0".repeat(63));
    let nines = "9".repeat(63);
    let within =
        format!(r#"{{"num":"{nines}","den":"{ten_to_63}"}},{{"num":"1","den":"{ten_to_63}"}}"#);
    let certificate = Certificate::from_json(&certificate_text(&within, 0))?;
    assert_eq!(certificate.matrix().map(TransitionMatrix::states), Some(2));
    let ten_to_64 = format!("1{}", "0".repeat(64));
    let beyond = format!(r#"{{"num":"1","den":"{ten_to_64}"}},{{"num":"1","den":"1"}}"#);
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
            source: TransitionRowError::NotNormalised
        })
    ));
    assert!(matches!(
        Certificate::from_json(&certificate_text("", 0)),
        Err(CertificateError::TransitionMatrix(
            TransitionMatrixError::NotSquare {
                row: 0,
                entries: 0,
                states: 2
            }
        ))
    ));
}

#[test]
fn rejects_an_empty_model() {
    let text = r#"{"format":"axiom-certificate","version":1,"model":{"rows":[]},"claims":[{"kind":"Mass","row":0}]}"#;
    assert!(matches!(
        Certificate::from_json(text),
        Err(CertificateError::TransitionMatrix(
            TransitionMatrixError::Empty
        ))
    ));
}

#[test]
fn rejects_claims_about_missing_rows() {
    assert!(matches!(
        Certificate::from_json(&certificate_text(halves(), 2)),
        Err(CertificateError::RowOutOfRange {
            claim: 0,
            row: 2,
            rows: 2
        })
    ));
}

#[test]
fn passes_text_of_exactly_the_limit_to_the_parser() {
    let at_limit = " ".repeat(MAX_CERTIFICATE_BYTES);
    assert!(matches!(
        Certificate::from_json(&at_limit),
        Err(CertificateError::Json { .. })
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
        Err(CertificateError::Json { .. })
    ));
    assert!(matches!(
        Certificate::from_json("not json"),
        Err(CertificateError::Json { .. })
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
        CertificateError::NoClaims.to_string(),
        "certificate makes no claims"
    );
    assert_eq!(
        CertificateError::DuplicateClaim { claim: 1, first: 0 }.to_string(),
        "claim 1 repeats claim 0"
    );
    assert_eq!(
        CertificateError::UnsupportedVersion(3).to_string(),
        "unsupported certificate version 3"
    );
}

/// A certificate whose model has `states` rows, each with `states` entries
/// of `1/states`.
fn uniform_certificate(states: usize) -> String {
    let entry = format!(r#"{{"num":"1","den":"{states}"}}"#);
    let row = format!("[{}]", vec![entry; states].join(","));
    let rows = vec![row; states].join(",");
    format!(
        r#"{{"format":"axiom-certificate","version":1,"model":{{"rows":[{rows}]}},"claims":[{{"kind":"Mass","row":0}}]}}"#
    )
}

#[test]
fn accepts_the_largest_allowed_model() -> Result<(), TestError> {
    let certificate = Certificate::from_json(&uniform_certificate(MAX_STATES))?;
    assert_eq!(
        certificate.matrix().map(TransitionMatrix::states),
        Some(MAX_STATES)
    );
    Ok(())
}

#[test]
fn rejects_one_state_more_than_the_limit() {
    assert!(matches!(
        Certificate::from_json(&uniform_certificate(MAX_STATES + 1)),
        Err(CertificateError::TooManyStates { states, limit }) if states == MAX_STATES + 1 && limit == MAX_STATES
    ));
}

#[test]
fn rejects_a_long_row_of_large_denominators_before_summing_it() {
    let entries: Vec<String> = (0..3_800)
        .map(|offset| format!(r#"{{"num":"1","den":"1{}{offset:04}"}}"#, "0".repeat(245)))
        .collect();
    let text = certificate_text(&entries.join(","), 0);
    assert!(text.len() <= MAX_CERTIFICATE_BYTES);
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::TransitionMatrix(
            TransitionMatrixError::NotSquare {
                row: 0,
                entries: 3_800,
                states: 2
            }
        ))
    ));
}

#[test]
fn error_messages_never_echo_certificate_text() {
    let escape = "\\u001b[31m";
    let hostile = [
        certificate_text(
            &format!(r#"{{"num":"{escape}","den":"2"}},{{"num":"1","den":"2"}}"#),
            0,
        ),
        certificate_text(halves(), 0).replace("axiom-certificate", escape),
        certificate_text(halves(), 0).replace("\"version\"", &format!("\"{escape}\"")),
        certificate_text(halves(), 0).replace("\"Mass\"", &format!("\"{escape}\"")),
    ];
    for text in hostile {
        let message = match Certificate::from_json(&text) {
            Ok(_) => String::new(),
            Err(error) => error.to_string(),
        };
        assert!(!message.is_empty(), "{text}");
        assert!(
            !message.contains('\u{1b}') && !message.contains("31m"),
            "{message}"
        );
    }
}

#[test]
fn rejects_row_indices_beyond_any_model() {
    assert!(matches!(
        Certificate::from_json(&certificate_text(halves(), 1 << 40)),
        Err(CertificateError::RowOutOfRange { claim: 0, row, rows: 2 }) if row == 1 << 40
    ));
}

#[test]
fn rejects_numbers_written_with_a_fraction_or_exponent() {
    let variants = [
        certificate_text(halves(), 0).replace("\"version\":1", "\"version\":1.0"),
        certificate_text(halves(), 0).replace("\"version\":1", "\"version\":1e0"),
        certificate_text(halves(), 0).replace("\"row\":0", "\"row\":0.0"),
        certificate_text(halves(), 0).replace("\"row\":0", "\"row\":-1"),
    ];
    for text in variants {
        assert!(
            matches!(
                Certificate::from_json(&text),
                Err(CertificateError::Json { .. })
            ),
            "{text}"
        );
    }
}

#[test]
fn rejects_a_byte_order_mark() {
    let text = format!("\u{feff}{}", certificate_text(halves(), 0));
    assert!(matches!(
        Certificate::from_json(&text),
        Err(CertificateError::Json { .. })
    ));
}
