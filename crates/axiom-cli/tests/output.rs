//! Interpreting the Lean checker's output, with forged output so that every
//! malformed case is covered without running Lean.

use std::time::Duration;

use axiom_cli::check::{MAX_OUTPUT_BYTES, interpret, read_bounded};
use axiom_cli::error::{CheckerReason, CliError};

const NO_TIME: Duration = Duration::ZERO;

/// The error for the checker output, or `None` for a report.
fn failure(code: i32, stdout: &str) -> Option<CliError> {
    interpret(code, stdout, NO_TIME).err()
}

#[test]
fn accepts_verdicts_that_agree_with_the_exit_status() -> Result<(), CliError> {
    let all = "claim 0 Mass row 0: accepted (proven)\nclaim 1 Estimate: accepted (estimated)\n";
    let report = interpret(0, all, NO_TIME)?;
    assert!(report.all_accepted());
    assert_eq!(report.claims(), 2);
    let one_rejected = "claim 0 Termination: rejected\n";
    assert!(!interpret(1, one_rejected, NO_TIME)?.all_accepted());
    Ok(())
}

#[test]
fn prints_each_verdict_with_its_certainty_and_a_summary() -> Result<(), CliError> {
    let lines = "claim 0 FixedPoint: accepted (proven)\nclaim 1 Contraction: rejected\nclaim 2 Estimate: accepted (estimated)\nclaim 3 ParametricContraction: accepted (proven)\n";
    let report = interpret(1, lines, NO_TIME)?;
    assert_eq!(
        report.to_string(),
        format!("{lines}3 of 4 claims accepted (2 proven, 1 estimated) in 0 ms\n")
    );
    Ok(())
}

#[test]
fn rejects_a_certainty_that_does_not_match_the_claim_kind() {
    for stdout in [
        "claim 0 Estimate: accepted (proven)\n",
        "claim 0 Mass row 0: accepted (estimated)\n",
        "claim 0 Termination: accepted\n",
        "claim 0 Termination: rejected (proven)\n",
    ] {
        assert_eq!(
            failure(0, stdout),
            Some(CliError::CheckerUnexpectedOutput),
            "{stdout}"
        );
    }
}

#[test]
fn rejects_an_exit_status_that_disagrees_with_the_verdicts() {
    let mismatch = Some(CliError::CheckerStatusMismatch);
    assert_eq!(
        failure(1, "claim 0 Mass row 0: accepted (proven)\n"),
        mismatch
    );
    assert_eq!(failure(0, "claim 0 Mass row 0: rejected\n"), mismatch);
}

#[test]
fn rejects_an_unexpected_line() {
    for stdout in [
        "claim 0 Mass row 0: accepted (proven)\nhello\n",
        "claim 0 Unknown: accepted (proven)\n",
        "claim x Termination: accepted (proven)\n",
        "claim 0 Mass row: accepted (proven)\n",
        "claim 0 Mass row 0: maybe\n",
    ] {
        assert_eq!(
            failure(0, stdout),
            Some(CliError::CheckerUnexpectedOutput),
            "{stdout}"
        );
    }
}

#[test]
fn reports_no_verdict_without_verdicts_or_an_error_line() {
    assert_eq!(failure(0, ""), Some(CliError::CheckerNoVerdict));
    assert_eq!(failure(2, ""), Some(CliError::CheckerNoVerdict));
}

#[test]
fn maps_the_first_checker_error_line_and_never_forwards_it() {
    assert_eq!(
        failure(
            2,
            "error: certificate is too large\nerror: certificate is not UTF-8\n"
        ),
        Some(CliError::CheckerRefused(CheckerReason::TooLarge))
    );
}

#[test]
fn reads_at_most_the_output_limit() {
    let limit = usize::try_from(MAX_OUTPUT_BYTES).unwrap_or(usize::MAX);
    let at_limit = vec![b'a'; limit];
    assert_eq!(read_bounded(at_limit.as_slice()).len(), limit);
    let past_limit = vec![b'a'; limit + 1];
    assert_eq!(read_bounded(past_limit.as_slice()).len(), limit);
}
