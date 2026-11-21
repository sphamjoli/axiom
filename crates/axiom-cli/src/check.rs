//! The `check` command: runs the Lean checker on a certificate.
//!
//! The checker is `AxiomCheckCli/Main.lean` in the Lean project, run with
//! `lake env lean --run` from the project directory. The certificate path is
//! passed as a separate argument, never through a shell, and the checker's
//! output is read up to a fixed limit.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::CliError;

/// The environment variable that overrides the Lean project directory.
pub const LEAN_DIR_VARIABLE: &str = "AXIOM_LEAN_DIR";

/// The Lean project directory used when [`LEAN_DIR_VARIABLE`] is unset.
const DEFAULT_LEAN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../lean");

/// The checker entry point, relative to the Lean project directory.
const CHECKER: &str = "AxiomCheckCli/Main.lean";

/// The most bytes read from each of the checker's output streams.
const MAX_OUTPUT_BYTES: u64 = 64 * 1024;

/// The longest error text from the checker that is shown.
const MAX_ERROR_CHARS: usize = 200;

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
    pub fn all_accepted(&self) -> bool {
        self.verdicts.iter().all(|verdict| verdict.accepted)
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

/// Runs the Lean checker on the certificate at `cert`.
pub fn run(cert: &Path) -> Result<Report, CliError> {
    let cert = cert.canonicalize().map_err(|source| CliError::Io {
        path: cert.to_path_buf(),
        source,
    })?;
    let dir = lean_dir();
    let start = Instant::now();
    let mut child = Command::new("lake")
        .args(["env", "lean", "--run", CHECKER])
        .arg(&cert)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| CliError::LeanStart {
            dir: dir.clone(),
            source,
        })?;
    let stderr = child
        .stderr
        .take()
        .map(|pipe| thread::spawn(move || read_bounded(pipe)));
    let stdout = child.stdout.take().map(read_bounded).unwrap_or_default();
    let stderr = stderr
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let status = child.wait().map_err(|source| CliError::LeanStart {
        dir: dir.clone(),
        source,
    })?;
    let elapsed = start.elapsed();
    let verdicts = parse_output(&stdout)?;
    match status.code() {
        Some(0 | 1) if !verdicts.is_empty() => {
            let consistent =
                (status.code() == Some(0)) == verdicts.iter().all(|verdict| verdict.accepted);
            if consistent {
                Ok(Report { verdicts, elapsed })
            } else {
                Err(CliError::Lean(
                    "exit status disagrees with the verdicts".to_owned(),
                ))
            }
        }
        _ => Err(CliError::Lean(failure_text(&stdout, &stderr))),
    }
}

/// Reads at most [`MAX_OUTPUT_BYTES`] from `pipe`, discarding the rest so
/// the child never blocks on a full pipe, and decodes it lossily.
fn read_bounded(pipe: impl Read) -> String {
    let mut pipe = pipe;
    let mut bytes = Vec::new();
    let _ = (&mut pipe).take(MAX_OUTPUT_BYTES).read_to_end(&mut bytes);
    let _ = std::io::copy(&mut pipe, &mut std::io::sink());
    String::from_utf8_lossy(&bytes).into_owned()
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

/// Whether `text` has the form `claim <n> Mass row <r>`.
fn is_claim(text: &str) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    let number = |word: &str| !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit());
    matches!(words.as_slice(), ["claim", index, "Mass", "row", row] if number(index) && number(row))
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
