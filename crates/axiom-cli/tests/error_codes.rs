//! Every command line error has a stable, distinct code, a fixed message
//! that names files by role and never by path, and an outcome. The table
//! pins all three, so a renamed code, a reworded message or a changed exit
//! code fails here.

mod common;

use std::collections::BTreeSet;
use std::io;

use axiom_cert::CertificateError;
use axiom_cli::checker_rule::CheckerRule;
use axiom_cli::error::{CheckerReason, CliError, FileRole, IoFailure};
use axiom_cli::outcome::Outcome;
use axiom_model::{ModelError, Position};
use common::{TestError, ensure};

fn every_error() -> Vec<CliError> {
    let mut errors = Vec::new();
    for file in FileRole::ALL {
        errors.push(CliError::ReadFailed {
            file,
            failure: IoFailure::NotFound,
        });
    }
    for failure in IoFailure::ALL {
        errors.push(CliError::WriteFailed {
            file: FileRole::Certificate,
            failure,
        });
    }
    errors.extend([
        CliError::ModelTooLarge { limit: 8 },
        CliError::ModelNotUtf8,
        CliError::ParametricChain,
        CliError::ParametricRetry,
        CliError::CheckerNotStarted,
        CliError::CheckerTimeout { seconds: 5 },
    ]);
    errors.extend(CheckerReason::ALL.map(CliError::CheckerRefused));
    errors.extend([
        CliError::CheckerNoVerdict,
        CliError::CheckerUnexpectedOutput,
        CliError::CheckerStatusMismatch,
    ]);
    errors
}

/// The line each error of [`every_error`] prints, and its outcome.
fn expected_reports() -> Vec<(String, Outcome)> {
    use Outcome::{Failed, Refused};
    let expected = [
        (
            "axiom: read-failed: cannot read the model file: not found",
            Failed,
        ),
        (
            "axiom: read-failed: cannot read the certificate file: not found",
            Failed,
        ),
        (
            "axiom: write-failed: cannot write the certificate file: not found",
            Failed,
        ),
        (
            "axiom: write-failed: cannot write the certificate file: permission denied",
            Failed,
        ),
        (
            "axiom: write-failed: cannot write the certificate file: is a directory",
            Failed,
        ),
        (
            "axiom: write-failed: cannot write the certificate file: input or output error",
            Failed,
        ),
        (
            "axiom: model-too-large: model file is larger than the limit of 8 bytes",
            Refused,
        ),
        (
            "axiom: model-not-utf8: model file is not valid UTF-8",
            Refused,
        ),
        (
            "axiom: parametric-chain: parametric chains are not yet supported: no claim kind covers them",
            Refused,
        ),
        (
            "axiom: parametric-retry: parametric retries are not yet supported: no claim kind covers them",
            Refused,
        ),
        (
            "axiom: checker-not-started: cannot run the Lean checker with lake",
            Failed,
        ),
        (
            "axiom: checker-timeout: the Lean checker was killed after the limit of 5 s",
            Failed,
        ),
        (
            "axiom: checker-not-regular-file: the checker refused the certificate: not a regular file",
            Refused,
        ),
        (
            "axiom: checker-too-large: the checker refused the certificate: larger than its limit",
            Refused,
        ),
        (
            "axiom: checker-not-utf8: the checker refused the certificate: not UTF-8",
            Refused,
        ),
        (
            "axiom: checker-unreadable: the checker could not read the certificate",
            Failed,
        ),
        (
            "axiom: checker-arguments: the checker was run with the wrong arguments",
            Failed,
        ),
        (
            "axiom: checker-invalid-certificate: the checker refused the certificate as invalid",
            Refused,
        ),
        (
            "axiom: checker-no-verdict: the Lean checker exited without a verdict",
            Failed,
        ),
        (
            "axiom: checker-unexpected-output: the Lean checker printed an unexpected line",
            Failed,
        ),
        (
            "axiom: checker-status-mismatch: the Lean checker's exit status disagrees with its verdicts",
            Failed,
        ),
    ];
    expected
        .iter()
        .map(|(line, outcome)| ((*line).to_owned(), *outcome))
        .collect()
}

#[test]
fn every_error_has_a_stable_code_fixed_message_and_outcome() -> Result<(), TestError> {
    let errors = every_error();
    let found: Vec<(String, Outcome)> = errors
        .iter()
        .map(|error| (error.report(), error.outcome()))
        .collect();
    ensure(found == expected_reports(), format!("{found:#?}"))?;
    let codes: BTreeSet<&str> = errors.iter().map(CliError::code).collect();
    // One code per variant: the six checker reasons and eleven others.
    ensure(
        codes.len() == 17,
        format!("codes are not distinct: {codes:?}"),
    )
}

#[test]
fn model_and_certificate_errors_keep_their_own_code_and_are_refusals() -> Result<(), TestError> {
    let model = CliError::from(ModelError::MissingRow {
        at: Position {
            line: 2,
            column: 14,
        },
    });
    ensure(
        model.report() == "axiom: missing-row: line 2, column 14: state has no row"
            && model.outcome() == Outcome::Refused,
        model.report(),
    )?;
    let certificate = CliError::from(CertificateError::NoClaims);
    ensure(
        certificate.report() == "axiom: no-claims: certificate makes no claims"
            && certificate.outcome() == Outcome::Refused,
        certificate.report(),
    )
}

#[test]
fn every_checker_error_line_maps_to_its_reason() -> Result<(), TestError> {
    let lines = [
        (
            "certificate is not a regular file",
            CheckerReason::NotRegularFile,
        ),
        ("certificate is too large", CheckerReason::TooLarge),
        ("certificate is not UTF-8", CheckerReason::NotUtf8),
        ("certificate cannot be read", CheckerReason::Unreadable),
        (
            "expected exactly one argument, the certificate path",
            CheckerReason::Arguments,
        ),
        ("a reason no checker prints", CheckerReason::Invalid),
        (
            "invalid JSON at character x: leading zero",
            CheckerReason::Invalid,
        ),
        (
            "invalid JSON at character 3: a claim is repeated",
            CheckerReason::Invalid,
        ),
        ("leading zero", CheckerReason::Invalid),
    ];
    for (line, reason) in lines {
        ensure(CheckerReason::from_line(line) == reason, line)?;
    }
    Ok(())
}

#[test]
fn checker_rules_name_their_rule_with_a_distinct_code() -> Result<(), TestError> {
    let cases = [
        (
            "invalid JSON at character 3: leading zero",
            "checker-json-leading-zero",
            "axiom: checker-json-leading-zero: the checker refused the certificate: leading zero",
        ),
        (
            "a claim is repeated",
            "checker-duplicate-claim",
            "axiom: checker-duplicate-claim: the checker refused the certificate: a claim is repeated",
        ),
    ];
    for (line, code, report) in cases {
        let error = CliError::CheckerRefused(CheckerReason::from_line(line));
        ensure(error.code() == code && error.report() == report, line)?;
        ensure(error.outcome() == Outcome::Refused, line)?;
    }
    let codes: BTreeSet<&str> = CheckerRule::ALL.iter().map(|rule| rule.code()).collect();
    let texts: BTreeSet<&str> = CheckerRule::ALL.iter().map(|rule| rule.text()).collect();
    ensure(
        codes.len() == CheckerRule::ALL.len() && texts.len() == CheckerRule::ALL.len(),
        "rule codes and texts are distinct",
    )
}

/// The reason strings the Lean reader can print, taken from its source:
/// every string literal in an `.error` or a `throw` there.
fn lean_reasons() -> Result<BTreeSet<String>, TestError> {
    let mut reasons = BTreeSet::new();
    for file in ["lean/AxiomCheck/Parse.lean", "lean/AxiomCheck/Json.lean"] {
        let source = std::fs::read_to_string(common::repo_root().join(file))?;
        for line in source.lines() {
            let Some(at) = line.find(".error ").or_else(|| line.find("throw ")) else {
                continue;
            };
            let Some((_, rest)) = line[at..].split_once('"') else {
                continue;
            };
            if let Some((text, _)) = rest.split_once('"') {
                reasons.insert(text.to_owned());
            }
        }
    }
    Ok(reasons)
}

#[test]
fn every_reason_the_lean_reader_prints_names_a_rule() -> Result<(), TestError> {
    let reasons = lean_reasons()?;
    ensure(reasons.len() >= 40, "found the reader's reasons")?;
    for reason in reasons {
        if reason == "certificate is too large" || reason.starts_with("invalid JSON at character") {
            continue;
        }
        let line = match reason.as_str() {
            "claim kind needs certificate version {needed}" => {
                for needed in ["2", "3"] {
                    let line = format!("claim kind needs certificate version {needed}");
                    ensure(CheckerRule::from_line(&line).is_some(), &line)?;
                }
                continue;
            }
            text => text.to_owned(),
        };
        let json = format!("invalid JSON at character 0: {line}");
        ensure(
            CheckerRule::from_line(&line).is_some() || CheckerRule::from_line(&json).is_some(),
            &line,
        )?;
    }
    Ok(())
}

#[test]
fn every_reader_rejection_fixture_names_a_rule() -> Result<(), TestError> {
    let table = std::fs::read_to_string(
        common::repo_root().join("fixtures/certificates/reject-reasons.tsv"),
    )?;
    for row in table.lines() {
        let Some(reason) = row.split('\t').nth(2) else {
            continue;
        };
        if reason == "check" {
            continue;
        }
        ensure(CheckerRule::from_line(reason).is_some(), reason)?;
    }
    Ok(())
}

#[test]
fn io_failures_keep_only_the_error_kind() -> Result<(), TestError> {
    let kinds = [
        (io::ErrorKind::NotFound, IoFailure::NotFound),
        (io::ErrorKind::PermissionDenied, IoFailure::PermissionDenied),
        (io::ErrorKind::IsADirectory, IoFailure::IsADirectory),
        (io::ErrorKind::StorageFull, IoFailure::Other),
    ];
    for (kind, failure) in kinds {
        let error = io::Error::new(kind, "/private/secret/path: os text");
        ensure(IoFailure::from(&error) == failure, format!("{kind:?}"))?;
    }
    Ok(())
}
