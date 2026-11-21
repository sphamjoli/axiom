//! The `check` command: runs the Lean checker on a certificate.
//!
//! The checker is `AxiomCheckCli/Main.lean` in the Lean project, run with
//! `lake env lean --run` from the project directory. The certificate path is
//! passed as a separate argument, never through a shell, the checker's
//! output is read up to a fixed limit, and the checker is killed when it runs
//! longer than a wall-clock limit.
//!
//! The Lean project directory decides which checker code runs. It is the
//! directory given by `--lean-dir` or [`LEAN_DIR_VARIABLE`], so whoever
//! controls the command line or the environment controls the checker;
//! otherwise it is the `lean` directory of the source tree the binary was
//! built from, a path fixed at compile time. An installed binary whose source
//! tree has moved or been deleted therefore needs one of them set.
//!
//! The checker's `error:` lines are mapped onto fixed codes, and its error
//! stream is never shown, so the same failure reads the same everywhere.

use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{CheckerReason, CliError, FileRole, IoFailure};

/// The environment variable that names the Lean project directory when
/// `--lean-dir` is not given, and with it the checker code that runs.
pub const LEAN_DIR_VARIABLE: &str = "AXIOM_LEAN_DIR";

/// The environment variable that sets the checker's wall-clock limit, in
/// whole seconds, when `--timeout-secs` is not given.
pub const TIMEOUT_VARIABLE: &str = "AXIOM_CHECK_TIMEOUT_SECS";

/// The checker's wall-clock limit in seconds when neither `--timeout-secs`
/// nor [`TIMEOUT_VARIABLE`] sets one.
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// The Lean project directory used when neither `--lean-dir` nor
/// [`LEAN_DIR_VARIABLE`] names one.
const DEFAULT_LEAN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../lean");

/// The checker entry point, relative to the Lean project directory.
const CHECKER: &str = "AxiomCheckCli/Main.lean";

/// The most bytes read from each of the checker's output streams.
pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024;

/// How often the checker is polled for exit while it runs.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How certain an accepted claim is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    /// Proven for the model by the checker: every claim kind but `Estimate`.
    Proven,
    /// An `Estimate` claim: the checker accepted its sample count, not its
    /// value.
    Estimated,
}

impl Certainty {
    /// The certainty of a claim, decided by its kind alone, so the word the
    /// checker prints is only ever compared against it, never trusted.
    fn of(claim: &str) -> Self {
        if claim.ends_with(" Estimate") {
            Self::Estimated
        } else {
            Self::Proven
        }
    }

    /// The word printed after an accepted verdict.
    const fn word(self) -> &'static str {
        match self {
            Self::Proven => "proven",
            Self::Estimated => "estimated",
        }
    }
}

/// The checker's verdict on one claim.
#[derive(Debug, PartialEq, Eq)]
pub struct Verdict {
    /// The claim's line as printed by the checker, such as
    /// `claim 0 Mass row 0`.
    claim: String,
    /// The certainty of the claim if the checker accepted it, or `None` if
    /// it rejected it.
    accepted: Option<Certainty>,
}

/// The verdicts on every claim and how long the check took.
#[derive(Debug)]
pub struct Report {
    verdicts: Vec<Verdict>,
    elapsed: Duration,
}

impl Report {
    /// Whether every claim was accepted.
    #[must_use]
    pub fn all_accepted(&self) -> bool {
        self.verdicts
            .iter()
            .all(|verdict| verdict.accepted.is_some())
    }

    /// The number of claims the checker gave a verdict on.
    #[must_use]
    pub fn claims(&self) -> usize {
        self.verdicts.len()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for verdict in &self.verdicts {
            match verdict.accepted {
                Some(certainty) => {
                    writeln!(
                        formatter,
                        "{}: accepted ({})",
                        verdict.claim,
                        certainty.word()
                    )?;
                }
                None => writeln!(formatter, "{}: rejected", verdict.claim)?,
            }
        }
        let count = |certainty| {
            self.verdicts
                .iter()
                .filter(|v| v.accepted == Some(certainty))
                .count()
        };
        let (proven, estimated) = (count(Certainty::Proven), count(Certainty::Estimated));
        writeln!(
            formatter,
            "{} of {} claims accepted ({proven} proven, {estimated} estimated) in {} ms",
            proven + estimated,
            self.verdicts.len(),
            self.elapsed.as_millis()
        )
    }
}

/// Runs the Lean checker on the certificate at `cert`, from the Lean project
/// in `lean_dir`, or the source tree's when that is `None`, and kills it
/// after `limit`.
///
/// # Errors
///
/// Returns [`CliError::ReadFailed`] when the certificate path cannot be
/// resolved, [`CliError::CheckerNotStarted`] when `lake` cannot be run,
/// [`CliError::CheckerTimeout`] when the checker runs longer than `limit`,
/// and the errors of [`interpret`] for its output.
pub fn run(cert: &Path, lean_dir: Option<&Path>, limit: Duration) -> Result<Report, CliError> {
    let cert = cert.canonicalize().map_err(|error| CliError::ReadFailed {
        file: FileRole::Certificate,
        failure: IoFailure::from(&error),
    })?;
    let dir = lean_dir.unwrap_or(Path::new(DEFAULT_LEAN_DIR));
    let start = Instant::now();
    let start_error = |_: std::io::Error| CliError::CheckerNotStarted;
    let mut command = Command::new("lake");
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .args(["env", "lean", "--run", CHECKER])
        .arg(&cert)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(start_error)?;
    let stdout = child
        .stdout
        .take()
        .map(|pipe| thread::spawn(move || read_bounded(pipe)));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| thread::spawn(move || read_bounded(pipe)));
    let status = wait_with_limit(&mut child, start, limit).map_err(start_error)?;
    let collect = |handle: Option<thread::JoinHandle<String>>| {
        handle
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    };
    let stdout = collect(stdout);
    // The error stream is drained so the checker never blocks on it, but it
    // is never shown: it holds paths and messages that vary by machine.
    drop(collect(stderr));
    let Some(code) = status else {
        return Err(CliError::CheckerTimeout {
            seconds: limit.as_secs(),
        });
    };
    interpret(code, &stdout, start.elapsed())
}

/// Waits for `child` until `limit` after `start`. Returns its exit code, or
/// `None` after killing it when the limit passes first. A child killed by a
/// signal has no exit code and is reported as exit code `-1`.
fn wait_with_limit(
    child: &mut Child,
    start: Instant,
    limit: Duration,
) -> std::io::Result<Option<i32>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status.code().unwrap_or(-1)));
        }
        if start.elapsed() >= limit {
            kill_group(child);
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Kills every process in the child's process group, so that the Lean
/// process `lake` started dies with it and releases the output pipes. The
/// child leads its own group, whose id is its process id. Failure is
/// ignored: the caller kills the child itself afterwards.
#[cfg(unix)]
fn kill_group(child: &Child) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Elsewhere the child is killed alone.
#[cfg(not(unix))]
fn kill_group(_child: &Child) {}

/// Reads at most [`MAX_OUTPUT_BYTES`] from `pipe`, discarding the rest so
/// the child never blocks on a full pipe, and decodes it lossily.
pub fn read_bounded(pipe: impl Read) -> String {
    let mut pipe = pipe;
    let mut bytes = Vec::new();
    let _ = (&mut pipe).take(MAX_OUTPUT_BYTES).read_to_end(&mut bytes);
    let _ = std::io::copy(&mut pipe, &mut std::io::sink());
    String::from_utf8_lossy(&bytes).into_owned()
}

/// What the checker printed: its verdicts and its first error reason.
struct Output<'a> {
    verdicts: Vec<Verdict>,
    error: Option<&'a str>,
}

/// Turns the checker's exit code and output into a report. Exit code 0 or 1
/// with at least one verdict and verdicts that agree with the code is a
/// report; anything else is an error.
///
/// # Errors
///
/// Returns [`CliError::CheckerUnexpectedOutput`] for a line that is neither
/// a verdict nor an `error:` line, [`CliError::CheckerStatusMismatch`] for
/// an exit code that disagrees with the verdicts,
/// [`CliError::CheckerRefused`] with the mapped reason when there is no
/// report and the checker printed an `error:` line, and
/// [`CliError::CheckerNoVerdict`] otherwise.
pub fn interpret(code: i32, stdout: &str, elapsed: Duration) -> Result<Report, CliError> {
    let Output { verdicts, error } = parse_output(stdout)?;
    match (code, error) {
        (0 | 1, _) if !verdicts.is_empty() => {
            if (code == 0) == verdicts.iter().all(|verdict| verdict.accepted.is_some()) {
                Ok(Report { verdicts, elapsed })
            } else {
                Err(CliError::CheckerStatusMismatch)
            }
        }
        (_, Some(reason)) => Err(CliError::CheckerRefused(CheckerReason::from_line(reason))),
        (_, None) => Err(CliError::CheckerNoVerdict),
    }
}

/// Reads the verdict lines and the first `error:` line; any other line is
/// unexpected, including an accepted verdict whose certainty word does not
/// match its claim kind.
fn parse_output(stdout: &str) -> Result<Output<'_>, CliError> {
    let mut output = Output {
        verdicts: Vec::new(),
        error: None,
    };
    for line in stdout.lines() {
        if let Some(reason) = line.strip_prefix("error: ") {
            output.error = output.error.or(Some(reason));
            continue;
        }
        let accepted_as = |certainty: Certainty| {
            line.strip_suffix(&format!(": accepted ({})", certainty.word()))
                .filter(|claim| Certainty::of(claim) == certainty)
                .map(|claim| (claim, Some(certainty)))
        };
        let (claim, accepted) = accepted_as(Certainty::Proven)
            .or_else(|| accepted_as(Certainty::Estimated))
            .or_else(|| line.strip_suffix(": rejected").map(|claim| (claim, None)))
            .filter(|(claim, _)| is_claim(claim))
            .ok_or(CliError::CheckerUnexpectedOutput)?;
        output.verdicts.push(Verdict {
            claim: claim.to_owned(),
            accepted,
        });
    }
    Ok(output)
}

/// Whether `text` has the form `claim <n> Mass row <r>` or `claim <n> <kind>`
/// for one of the kinds `FixedPoint`, `Contraction`, `ConvergenceBound`,
/// `Termination`, `ParametricContraction` and `Estimate`.
fn is_claim(text: &str) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    let number = |word: &str| !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit());
    match words.as_slice() {
        ["claim", index, "Mass", "row", row] => number(index) && number(row),
        [
            "claim",
            index,
            "FixedPoint"
            | "Contraction"
            | "ConvergenceBound"
            | "Termination"
            | "ParametricContraction"
            | "Estimate",
        ] => number(index),
        _ => false,
    }
}
