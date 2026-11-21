//! Interpreting the Lean checker's output, with forged output so that every
//! malformed case is covered without running Lean.

use std::time::Duration;

use axiom_cli::check::{MAX_ERROR_CHARS, MAX_OUTPUT_BYTES, interpret, read_bounded};
use axiom_cli::error::CliError;

const NO_TIME: Duration = Duration::ZERO;

/// The message of a `CliError::Lean`, or `None` for any other outcome.
fn lean_error(code: i32, stdout: &str, stderr: &str) -> Option<String> {
    match interpret(code, stdout, stderr, NO_TIME) {
        Err(CliError::Lean(message)) => Some(message),
        _ => None,
    }
}

#[test]
fn accepts_verdicts_that_agree_with_the_exit_status() -> Result<(), CliError> {
    let all = "claim 0 Mass row 0: accepted\nclaim 1 ConvergenceBound: accepted\n";
    let report = interpret(0, all, "", NO_TIME)?;
    assert!(report.all_accepted());
    assert_eq!(report.claims(), 2);
    let one_rejected = "claim 0 Termination: rejected\n";
    assert!(!interpret(1, one_rejected, "", NO_TIME)?.all_accepted());
    Ok(())
}

#[test]
fn prints_each_verdict_and_a_summary() -> Result<(), CliError> {
    let report = interpret(
        1,
        "claim 0 FixedPoint: accepted\nclaim 1 Contraction: rejected\n",
        "",
        NO_TIME,
    )?;
    assert_eq!(
        report.to_string(),
        "claim 0 FixedPoint: accepted\nclaim 1 Contraction: rejected\n1 of 2 claims accepted in 0 ms\n"
    );
    Ok(())
}

#[test]
fn rejects_an_exit_status_that_disagrees_with_the_verdicts() {
    let disagreement = Some("exit status disagrees with the verdicts".to_owned());
    assert_eq!(
        lean_error(1, "claim 0 Mass row 0: accepted\n", ""),
        disagreement
    );
    assert_eq!(
        lean_error(0, "claim 0 Mass row 0: rejected\n", ""),
        disagreement
    );
}

#[test]
fn rejects_an_unexpected_line() {
    let unexpected = Some("unexpected output line".to_owned());
    for stdout in [
        "claim 0 Mass row 0: accepted\nhello\n",
        "claim 0 Unknown: accepted\n",
        "claim x Termination: accepted\n",
        "claim 0 Mass row: accepted\n",
        "claim 0 Mass row 0: maybe\n",
    ] {
        assert_eq!(lean_error(0, stdout, ""), unexpected, "{stdout}");
    }
}

#[test]
fn rejects_exit_zero_without_verdicts() {
    assert_eq!(
        lean_error(0, "", ""),
        Some("exited without a verdict".to_owned())
    );
}

#[test]
fn reports_the_checker_error_line_or_the_error_stream() {
    assert_eq!(
        lean_error(2, "error: certificate is too large\n", "noise"),
        Some("certificate is too large".to_owned())
    );
    assert_eq!(
        lean_error(2, "", "\n  \nlake: unknown target\nmore"),
        Some("lake: unknown target".to_owned())
    );
}

#[test]
fn truncates_long_error_text() {
    let long = "x".repeat(MAX_ERROR_CHARS + 1);
    assert_eq!(lean_error(2, "", &long), Some("x".repeat(MAX_ERROR_CHARS)));
    let exact = "y".repeat(MAX_ERROR_CHARS);
    assert_eq!(lean_error(2, "", &exact), Some(exact));
}

#[test]
fn reads_at_most_the_output_limit() {
    let limit = usize::try_from(MAX_OUTPUT_BYTES).unwrap_or(usize::MAX);
    let at_limit = vec![b'a'; limit];
    assert_eq!(read_bounded(at_limit.as_slice()).len(), limit);
    let past_limit = vec![b'a'; limit + 1];
    assert_eq!(read_bounded(past_limit.as_slice()).len(), limit);
}
