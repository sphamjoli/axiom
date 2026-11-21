//! The `check` command: runs the Lean checker on a certificate.
//!
//! The checker is `AxiomCheckCli/Main.lean` in the Lean project, run with
//! `lake env lean --run` from the project directory. The certificate path is
//! passed as a separate argument, never through a shell, the checker's
//! output is read up to a fixed limit, and the checker is killed when it runs
//! longer than a wall-clock limit.
//!
//! The Lean project directory decides which checker code runs. It is the
//! directory named by [`LEAN_DIR_VARIABLE`] when that is set, so whoever
//! controls the environment controls the checker; otherwise it is the `lean`
//! directory of the source tree the binary was built from, a path fixed at
//! compile time. An installed binary whose source tree has moved or been
//! deleted therefore needs [`LEAN_DIR_VARIABLE`] set.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::CliError;

/// The environment variable that overrides the Lean project directory, and
/// with it the checker code that runs.
pub const LEAN_DIR_VARIABLE: &str = "AXIOM_LEAN_DIR";

/// The environment variable that overrides the checker's wall-clock limit,
/// in whole seconds.
pub const TIMEOUT_VARIABLE: &str = "AXIOM_CHECK_TIMEOUT_SECS";

/// The checker's wall-clock limit when [`TIMEOUT_VARIABLE`] is unset.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(1);

/// The Lean project directory used when [`LEAN_DIR_VARIABLE`] is unset.
const DEFAULT_LEAN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../lean");

/// The checker entry point, relative to the Lean project directory.
const CHECKER: &str = "AxiomCheckCli/Main.lean";

/// The most bytes read from each of the checker's output streams.
pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024;

/// The longest error text from the checker that is shown.
pub const MAX_ERROR_CHARS: usize = 200;

/// How often the checker is polled for exit while it runs.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// The checker's verdict on one claim.
#[derive(Debug, PartialEq, Eq)]
pub struct Verdict {
    /// The claim's line as printed by the checker, such as
    /// `claim 0 Mass row 0`.
    claim: String,
    /// Whether the checker accepted the claim.
    accepted: bool,
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
        self.verdicts.iter().all(|verdict| verdict.accepted)
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
            let word = if verdict.accepted {
                "accepted"
            } else {
                "rejected"
            };
            writeln!(formatter, "{}: {word}", verdict.claim)?;
        }
        let accepted = self.verdicts.iter().filter(|v| v.accepted).count();
        writeln!(
            formatter,
            "{accepted} of {} claims accepted in {} ms",
            self.verdicts.len(),
            self.elapsed.as_millis()
        )
    }
}

/// The Lean project directory: [`LEAN_DIR_VARIABLE`] if set, otherwise the
/// `lean` directory of the repository this binary was built from.
fn lean_dir() -> PathBuf {
    std::env::var_os(LEAN_DIR_VARIABLE)
        .map_or_else(|| PathBuf::from(DEFAULT_LEAN_DIR), PathBuf::from)
}

/// The checker's wall-clock limit: [`TIMEOUT_VARIABLE`] if set, otherwise
/// [`DEFAULT_TIMEOUT`].
///
/// # Errors
///
/// Returns [`CliError::Usage`] when the variable is set to anything but a
/// positive whole number of seconds, so a typo never disables the limit.
pub fn timeout() -> Result<Duration, CliError> {
    match std::env::var_os(TIMEOUT_VARIABLE) {
        None => Ok(DEFAULT_TIMEOUT),
        Some(value) => value
            .to_str()
            .and_then(|text| text.parse::<u64>().ok())
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs)
            .ok_or(CliError::Usage(
                "AXIOM_CHECK_TIMEOUT_SECS must be a positive whole number of seconds",
            )),
    }
}

/// Runs the Lean checker on the certificate at `cert`.
///
/// # Errors
///
/// Returns [`CliError::Io`] when the certificate path cannot be resolved,
/// [`CliError::LeanStart`] when `lake` cannot be run, [`CliError::LeanTimeout`]
/// when the checker runs longer than [`timeout`], and the errors of
/// [`interpret`] for its output.
pub fn run(cert: &Path) -> Result<Report, CliError> {
    let limit = timeout()?;
    let cert = cert.canonicalize().map_err(|source| CliError::Io {
        path: cert.to_path_buf(),
        source,
    })?;
    let dir = lean_dir();
    let start = Instant::now();
    let start_error = |source| CliError::LeanStart {
        dir: dir.clone(),
        source,
    };
    let mut command = Command::new("lake");
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .args(["env", "lean", "--run", CHECKER])
        .arg(&cert)
        .current_dir(&dir)
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
    let (stdout, stderr) = (collect(stdout), collect(stderr));
    let Some(code) = status else {
        return Err(CliError::LeanTimeout {
            seconds: limit.as_secs(),
        });
    };
    interpret(code, &stdout, &stderr, start.elapsed())
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

/// Turns the checker's exit code and output into a report. Exit code 0 or 1
/// with at least one verdict and verdicts that agree with the code is a
/// report; anything else is an error.
///
/// # Errors
///
/// Returns [`CliError::Lean`] for an output line that is neither a verdict
/// nor an `error:` line, for an exit code that disagrees with the verdicts,
/// and for any other exit code or no verdicts, with the checker's own error
/// message or the start of its error stream, at most [`MAX_ERROR_CHARS`]
/// characters long.
pub fn interpret(
    code: i32,
    stdout: &str,
    stderr: &str,
    elapsed: Duration,
) -> Result<Report, CliError> {
    let verdicts = parse_output(stdout)?;
    match code {
        0 | 1 if !verdicts.is_empty() => {
            if (code == 0) == verdicts.iter().all(|verdict| verdict.accepted) {
                Ok(Report { verdicts, elapsed })
            } else {
                Err(CliError::Lean(
                    "exit status disagrees with the verdicts".to_owned(),
                ))
            }
        }
        _ => Err(CliError::Lean(failure_text(stdout, stderr))),
    }
}

/// Reads the verdict lines. Lines starting with `error:` are skipped here
/// and reported by [`failure_text`]; any other line is unexpected.
fn parse_output(stdout: &str) -> Result<Vec<Verdict>, CliError> {
    let mut verdicts = Vec::new();
    for line in stdout.lines() {
        if line.starts_with("error:") {
            continue;
        }
        let verdict = line
            .strip_suffix(": accepted")
            .map(|claim| (claim, true))
            .or_else(|| line.strip_suffix(": rejected").map(|claim| (claim, false)))
            .filter(|(claim, _)| is_claim(claim))
            .ok_or_else(|| CliError::Lean("unexpected output line".to_owned()))?;
        verdicts.push(Verdict {
            claim: verdict.0.to_owned(),
            accepted: verdict.1,
        });
    }
    Ok(verdicts)
}

/// Whether `text` has the form `claim <n> Mass row <r>` or `claim <n> <kind>`
/// for one of the kinds `FixedPoint`, `Contraction`, `ConvergenceBound`
/// and `Termination`.
fn is_claim(text: &str) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    let number = |word: &str| !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit());
    match words.as_slice() {
        ["claim", index, "Mass", "row", row] => number(index) && number(row),
        [
            "claim",
            index,
            "FixedPoint" | "Contraction" | "ConvergenceBound" | "Termination",
        ] => number(index),
        _ => false,
    }
}

/// The checker's own error message if it printed one, otherwise a short
/// excerpt of its error stream.
fn failure_text(stdout: &str, stderr: &str) -> String {
    let message = stdout
        .lines()
        .find_map(|line| line.strip_prefix("error: "))
        .or_else(|| stderr.lines().find(|line| !line.trim().is_empty()))
        .unwrap_or("exited without a verdict");
    message.chars().take(MAX_ERROR_CHARS).collect()
}
