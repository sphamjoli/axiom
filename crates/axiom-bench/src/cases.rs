//! The models the harness measures: every fixture model, and a generated
//! scaling family that grows one dimension at a time.

use std::fmt::Write as _;
use std::path::Path;

use axiom_cli::error::IoFailure;
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::error::BenchError;

/// The tolerance of the convergence bound the engine states when a case
/// asks for no other, the one `axiom analyse` uses.
pub const DEFAULT_TOLERANCE: (u64, u64) = (1, 1000);

/// The state counts of the `states` family.
pub const STATE_COUNTS: [u64; 5] = [3, 4, 8, 12, 16];

/// The `m` of the `horizon` family: chains that leave each state with
/// probability `1/m`, so the bound needs more steps as `m` grows.
pub const HORIZONS: [u64; 6] = [5, 10, 20, 50, 100, 200];

/// The exponents `k` of the `tolerance` family: the links chain with the
/// convergence bound stated to `1/10^k`.
pub const TOLERANCE_EXPONENTS: [u32; 5] = [2, 3, 4, 5, 6];

/// Which group of models a case belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// A model from the fixtures directory.
    Fixture,
    /// A chain with more states, which the engine certifies with `Mass`
    /// claims only.
    States,
    /// A two-state chain that mixes ever more slowly.
    Horizon,
    /// The links chain with an ever tighter convergence tolerance.
    Tolerance,
}

impl Family {
    /// The name written in the CSV files.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::States => "states",
            Self::Horizon => "horizon",
            Self::Tolerance => "tolerance",
        }
    }
}

/// One model to measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// The case's name: the fixture's path below the fixtures directory
    /// without `.axm`, or a generated name such as `states-8`.
    pub name: String,
    /// The group it belongs to.
    pub family: Family,
    /// What grows within the family: the state count, `m` or `k`; zero for
    /// a fixture.
    pub parameter: u64,
    /// The model's source text.
    pub source: String,
    /// The tolerance of the convergence bound the engine states.
    pub tolerance: BigRational,
}

fn fraction(numerator: u64, denominator: u64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

/// The default tolerance, [`DEFAULT_TOLERANCE`].
#[must_use]
pub fn default_tolerance() -> BigRational {
    fraction(DEFAULT_TOLERANCE.0, DEFAULT_TOLERANCE.1)
}

/// Every `.axm` model directly in `dir` and in its `templates`
/// subdirectory, sorted by name. The `reject` subdirectory holds models the
/// parser must refuse and is not benchmarked.
///
/// # Errors
///
/// Returns [`BenchError::ListFixtures`] when `dir` cannot be listed,
/// [`BenchError::ReadModel`] when a model cannot be read, and
/// [`BenchError::NoFixtures`] when there is none.
pub fn fixtures(dir: &Path) -> Result<Vec<Case>, BenchError> {
    let mut cases = models_in(dir, "")?;
    let templates = dir.join("templates");
    if templates.is_dir() {
        cases.extend(models_in(&templates, "templates/")?);
    }
    if cases.is_empty() {
        return Err(BenchError::NoFixtures);
    }
    cases.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(cases)
}

fn models_in(dir: &Path, prefix: &str) -> Result<Vec<Case>, BenchError> {
    let list_error = |error: std::io::Error| BenchError::ListFixtures(IoFailure::from(&error));
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(list_error)? {
        let path = entry.map_err(list_error)?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("axm") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let source = std::fs::read_to_string(&path)
            .map_err(|error| BenchError::ReadModel(IoFailure::from(&error)))?;
        cases.push(Case {
            name: format!("{prefix}{stem}"),
            family: Family::Fixture,
            parameter: 0,
            source,
            tolerance: default_tolerance(),
        });
    }
    Ok(cases)
}

/// The generated scaling family: [`states_model`] for each of
/// [`STATE_COUNTS`], [`horizon_model`] for each of [`HORIZONS`], and the
/// links chain for each tolerance `1/10^k` of [`TOLERANCE_EXPONENTS`].
#[must_use]
pub fn scaling_family() -> Vec<Case> {
    let states = STATE_COUNTS.into_iter().map(|n| Case {
        name: format!("states-{n}"),
        family: Family::States,
        parameter: n,
        source: states_model(n),
        tolerance: default_tolerance(),
    });
    let horizons = HORIZONS.into_iter().map(|m| Case {
        name: format!("horizon-{m}"),
        family: Family::Horizon,
        parameter: m,
        source: horizon_model(m),
        tolerance: default_tolerance(),
    });
    let tolerances = TOLERANCE_EXPONENTS.into_iter().map(|k| Case {
        name: format!("tolerance-{k}"),
        family: Family::Tolerance,
        parameter: u64::from(k),
        source: LINKS.to_owned(),
        tolerance: fraction(1, 10_u64.pow(k)),
    });
    states.chain(horizons).chain(tolerances).collect()
}

/// Alice's links chain, the model of `fixtures/models/links.axm`.
pub const LINKS: &str = "chain links {
  states up, down
  from up:   up 9/10, down 1/10
  from down: up 3/10, down 7/10
}
";

/// A lazy random walk on `n` states in a line: each state moves to either
/// neighbour with probability one half, and an end state stays put instead
/// of moving off the line.
#[must_use]
pub fn states_model(n: u64) -> String {
    let last = n.saturating_sub(1);
    let names: Vec<String> = (0..n).map(|state| format!("s{state}")).collect();
    let mut text = format!("chain walk{n} {{\n  states {}\n", names.join(", "));
    for state in 0..n {
        let left = state.saturating_sub(1);
        let right = (state + 1).min(last);
        let _ = writeln!(text, "  from s{state}: s{left} 1/2, s{right} 1/2");
    }
    text.push_str("}\n");
    text
}

/// The two-state chain that leaves each state with probability `1/m`, whose
/// contraction factor `1 - 2/m` approaches one as `m` grows.
#[must_use]
pub fn horizon_model(m: u64) -> String {
    let stay = m.saturating_sub(1);
    format!(
        "chain slow{m} {{\n  states up, down\n  from up:   up {stay}/{m}, down 1/{m}\n  from down: up 1/{m}, down {stay}/{m}\n}}\n"
    )
}
