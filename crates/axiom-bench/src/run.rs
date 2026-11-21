//! A whole benchmark run: measure every case, then write the CSV files,
//! the Markdown table and the plots.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axiom_cli::error::IoFailure;

use crate::cases::{self, Case, Family};
use crate::engine::{self, Built};
use crate::error::{BenchError, OutputRole};
use crate::report::{self, ComparisonRow, RunRow};
use crate::simulate::{self, Kind};
use crate::svg::{self, Point};

/// How many times the engine step is timed per case; the median is kept.
pub const ENGINE_REPEATS: usize = 25;

/// What a run reads, writes and how hard it measures.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The fixtures directory.
    pub fixtures: PathBuf,
    /// Where the CSV files, the table and the plots go.
    pub out: PathBuf,
    /// Where the certificates handed to the checker go.
    pub work: PathBuf,
    /// The Lean project the checker runs from, or the source tree's.
    pub lean_dir: Option<PathBuf>,
    /// How many times each certificate is checked; the median is kept.
    pub repeat: usize,
    /// The most random draws one simulation job runs before its time is
    /// extrapolated.
    pub sim_budget: u64,
    /// The seed every simulation draws from.
    pub seed: u64,
    /// The checker's wall-clock limit.
    pub timeout: Duration,
}

/// The rows a run produced.
#[derive(Debug, Clone, Default)]
pub struct Measurements {
    /// One per case.
    pub runs: Vec<RunRow>,
    /// One per claim kind of each case.
    pub comparisons: Vec<ComparisonRow>,
}

/// Measures every fixture and every case of the scaling family, then
/// writes `results.csv`, `comparison.csv`, `comparison.md` and the plots to
/// the output directory. `progress` is told each case's name as it starts.
///
/// # Errors
///
/// Returns the first failure: unreadable fixtures, a scaling model the
/// engine refuses, a checker that cannot run or rejects a claim, or an
/// output that cannot be written.
pub fn run(
    settings: &Settings,
    progress: &mut dyn FnMut(&str),
) -> Result<Measurements, BenchError> {
    let mut all = cases::fixtures(&settings.fixtures)?;
    all.extend(cases::scaling_family());
    create_dir(&settings.out)?;
    create_dir(&settings.work)?;
    let mut measurements = Measurements::default();
    for case in &all {
        progress(&case.name);
        measure(case, settings, &mut measurements)?;
    }
    write(&settings.out, &measurements)?;
    Ok(measurements)
}

fn create_dir(dir: &Path) -> Result<(), BenchError> {
    std::fs::create_dir_all(dir).map_err(|error| BenchError::CreateDir(IoFailure::from(&error)))
}

fn write_file(path: &Path, role: OutputRole, text: &str) -> Result<(), BenchError> {
    std::fs::write(path, text)
        .map_err(|error| BenchError::WriteFailed(role, IoFailure::from(&error)))
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

fn measure(case: &Case, settings: &Settings, out: &mut Measurements) -> Result<(), BenchError> {
    let mut engine_times = Vec::with_capacity(ENGINE_REPEATS);
    let mut built = Built::Refused("not-run");
    for _ in 0..ENGINE_REPEATS {
        let started = Instant::now();
        built = engine::build(&case.source, &case.tolerance);
        if let Built::Certificate(certificate) = &built {
            certificate.to_json().map_err(BenchError::Certificate)?;
        }
        engine_times.push(started.elapsed().as_secs_f64() * 1e6);
    }
    let certificate = match built {
        Built::Certificate(certificate) => certificate,
        Built::Refused(code) => {
            if case.family != Family::Fixture {
                return Err(BenchError::FamilyRefused);
            }
            out.runs.push(RunRow {
                case: case.name.clone(),
                family: case.family,
                parameter: case.parameter,
                status: code.to_owned(),
                claims: 0,
                engine_us: median(engine_times),
                cert_bytes: 0,
                check_ms: 0.0,
                sim_ms: 0.0,
                sim_extrapolated: false,
            });
            return Ok(());
        }
    };
    let json = certificate.to_json().map_err(BenchError::Certificate)?;
    let path = settings
        .work
        .join(format!("{}.json", case.name.replace('/', "-")));
    write_file(&path, OutputRole::Certificate, &json)?;
    let mut check_times = Vec::with_capacity(settings.repeat);
    for _ in 0..settings.repeat {
        let started = Instant::now();
        let verdicts = axiom_cli::check::run(&path, settings.lean_dir.as_deref(), settings.timeout)
            .map_err(BenchError::Check)?;
        check_times.push(started.elapsed().as_secs_f64() * 1e3);
        if !verdicts.all_accepted() {
            return Err(BenchError::CheckRejected);
        }
    }
    let check_ms = median(check_times);
    let mut kinds: Vec<Kind> = certificate.claims().iter().filter_map(Kind::of).collect();
    kinds.sort();
    kinds.dedup();
    let (mut sim_ms, mut sim_extrapolated) = (0.0, false);
    for kind in kinds {
        let plan = simulate::plan(kind, &certificate)?;
        let timing = simulate::execute(&plan, settings.sim_budget, settings.seed)?;
        sim_ms += timing.ms;
        sim_extrapolated |= timing.extrapolated;
        out.comparisons.push(ComparisonRow {
            case: case.name.clone(),
            family: case.family,
            parameter: case.parameter,
            kind,
            conclusion: plan.conclusion.clone(),
            check_ms,
            sim_samples: plan.samples(),
            sim_draws: plan.draws(),
            sim_ms: timing.ms,
            sim_extrapolated: timing.extrapolated,
            lower_bound: plan.lower_bound,
        });
    }
    out.runs.push(RunRow {
        case: case.name.clone(),
        family: case.family,
        parameter: case.parameter,
        status: "ok".to_owned(),
        claims: certificate.claims().len(),
        engine_us: median(engine_times),
        cert_bytes: json.len(),
        check_ms,
        sim_ms,
        sim_extrapolated,
    });
    Ok(())
}

/// The points of one scaling family for one kind, with `x` from each row.
#[must_use]
pub fn family_points(
    rows: &[ComparisonRow],
    family: Family,
    kind: Kind,
    x: impl Fn(&ComparisonRow) -> f64,
) -> Vec<Point> {
    rows.iter()
        .filter(|row| row.family == family && row.kind == kind)
        .map(|row| Point {
            x: x(row),
            check_ms: row.check_ms,
            sim_ms: row.sim_ms,
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "family parameters are small whole numbers"
)]
fn parameter(row: &ComparisonRow) -> f64 {
    row.parameter as f64
}

fn write(out: &Path, measurements: &Measurements) -> Result<(), BenchError> {
    let rows = &measurements.comparisons;
    write_file(
        &out.join("results.csv"),
        OutputRole::Results,
        &report::results_csv(&measurements.runs),
    )?;
    write_file(
        &out.join("comparison.csv"),
        OutputRole::Comparison,
        &report::comparison_csv(rows),
    )?;
    write_file(
        &out.join("comparison.md"),
        OutputRole::Table,
        &report::markdown(rows),
    )?;
    write_file(
        &out.join("ratio.svg"),
        OutputRole::Plot,
        &svg::ratio_chart(rows),
    )?;
    let tolerance = family_points(rows, Family::Tolerance, Kind::ConvergenceBound, |row| {
        10_f64.powf(parameter(row))
    });
    write_file(
        &out.join("scaling-tolerance.svg"),
        OutputRole::Plot,
        &svg::scaling_chart(
            "ConvergenceBound on links: check against simulate as the tolerance tightens",
            "1 / tolerance",
            &tolerance,
        ),
    )?;
    let horizon = family_points(rows, Family::Horizon, Kind::ConvergenceBound, parameter);
    write_file(
        &out.join("scaling-horizon.svg"),
        OutputRole::Plot,
        &svg::scaling_chart(
            "ConvergenceBound on chains leaving each state with probability 1/m",
            "m",
            &horizon,
        ),
    )?;
    let states = family_points(rows, Family::States, Kind::Mass, parameter);
    write_file(
        &out.join("scaling-states.svg"),
        OutputRole::Plot,
        &svg::scaling_chart(
            "Mass on a random walk: check against simulate as the states grow",
            "states",
            &states,
        ),
    )
}
