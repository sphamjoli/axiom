//! The measurements as rows, their CSV and Markdown forms, and the verdict
//! on the hypothesis that checking is much cheaper than simulating.

use std::fmt::Write as _;

use crate::cases::Family;
use crate::simulate::Kind;

/// How many times cheaper checking must be than simulating for the
/// hypothesis "checking is much cheaper" to count as confirmed: an order of
/// magnitude.
pub const MUCH_CHEAPER: f64 = 10.0;

/// The measurements of one case: T8.1.
#[derive(Debug, Clone, PartialEq)]
pub struct RunRow {
    /// The case's name.
    pub case: String,
    /// The case's family.
    pub family: Family,
    /// What grows within the family.
    pub parameter: u64,
    /// `ok`, or the code of the engine's refusal.
    pub status: String,
    /// The number of claims written, zero when refused.
    pub claims: usize,
    /// The engine's median time to parse, build and encode, in microseconds.
    pub engine_us: f64,
    /// The certificate's size in bytes.
    pub cert_bytes: usize,
    /// The median wall-clock time of `axiom check`, in milliseconds.
    pub check_ms: f64,
    /// The time to simulate every claim's conclusion, in milliseconds.
    pub sim_ms: f64,
    /// Whether any of that time was extrapolated.
    pub sim_extrapolated: bool,
}

/// One claim kind of one case compared: T8.2.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonRow {
    /// The case's name.
    pub case: String,
    /// The case's family.
    pub family: Family,
    /// What grows within the family.
    pub parameter: u64,
    /// The claim kind.
    pub kind: Kind,
    /// The conclusion both sides reach.
    pub conclusion: String,
    /// The median time of `axiom check` on the whole certificate, in
    /// milliseconds, charged in full to every kind it holds.
    pub check_ms: f64,
    /// The samples simulation needs.
    pub sim_samples: u64,
    /// The random draws simulation needs, at most.
    pub sim_draws: u64,
    /// The time simulation needs, in milliseconds.
    pub sim_ms: f64,
    /// Whether that time was extrapolated from a prefix of the samples.
    pub sim_extrapolated: bool,
    /// Whether simulation covers only part of the conclusion, so its time
    /// is a lower bound.
    pub lower_bound: bool,
}

impl ComparisonRow {
    /// Simulation time over checking time: how many times cheaper checking
    /// is.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        self.sim_ms / self.check_ms
    }
}

/// Whether the hypothesis held for a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Checking is at least [`MUCH_CHEAPER`] times cheaper on every fixture
    /// with a claim of the kind.
    Confirmed,
    /// It is not, on some fixture.
    Refuted,
}

impl Outcome {
    /// The word written in the table.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Refuted => "refuted",
        }
    }
}

/// The verdict on one claim kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    /// The kind.
    pub kind: Kind,
    /// The least ratio over the fixtures.
    pub fixture_ratio: f64,
    /// The verdict, decided on the fixtures alone, whose conclusions are the
    /// ones the engine states by default.
    pub outcome: Outcome,
    /// The rows of the scaling family with the kind.
    pub family_rows: usize,
    /// How many of them have a ratio of at least [`MUCH_CHEAPER`].
    pub family_confirmed: usize,
}

/// The verdict on every kind that appears on a fixture, in [`Kind::ALL`]
/// order.
#[must_use]
pub fn verdicts(rows: &[ComparisonRow]) -> Vec<Verdict> {
    Kind::ALL
        .into_iter()
        .filter_map(|kind| {
            let of_kind = || rows.iter().filter(move |row| row.kind == kind);
            let fixture_ratio = of_kind()
                .filter(|row| row.family == Family::Fixture)
                .map(ComparisonRow::ratio)
                .reduce(f64::min)?;
            let family: Vec<&ComparisonRow> = of_kind()
                .filter(|row| row.family != Family::Fixture)
                .collect();
            Some(Verdict {
                kind,
                fixture_ratio,
                outcome: if fixture_ratio >= MUCH_CHEAPER {
                    Outcome::Confirmed
                } else {
                    Outcome::Refuted
                },
                family_rows: family.len(),
                family_confirmed: family
                    .iter()
                    .filter(|row| row.ratio() >= MUCH_CHEAPER)
                    .count(),
            })
        })
        .collect()
}

/// A CSV field, quoted when it holds a comma, a quote or a line break.
#[must_use]
pub fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The header of `results.csv`.
pub const RESULTS_HEADER: &str =
    "case,family,parameter,status,claims,engine_us,cert_bytes,check_ms,sim_ms,sim_extrapolated";

/// The header of `comparison.csv`.
pub const COMPARISON_HEADER: &str = "case,family,parameter,kind,conclusion,check_ms,sim_samples,sim_draws,sim_ms,sim_extrapolated,lower_bound,ratio";

/// `results.csv`: one line per case.
#[must_use]
pub fn results_csv(rows: &[RunRow]) -> String {
    let mut text = format!("{RESULTS_HEADER}\n");
    for row in rows {
        let _ = writeln!(
            text,
            "{},{},{},{},{},{:.1},{},{:.1},{:.1},{}",
            csv_field(&row.case),
            row.family.label(),
            row.parameter,
            csv_field(&row.status),
            row.claims,
            row.engine_us,
            row.cert_bytes,
            row.check_ms,
            row.sim_ms,
            row.sim_extrapolated
        );
    }
    text
}

/// `comparison.csv`: one line per claim kind of each case.
#[must_use]
pub fn comparison_csv(rows: &[ComparisonRow]) -> String {
    let mut text = format!("{COMPARISON_HEADER}\n");
    for row in rows {
        let _ = writeln!(
            text,
            "{},{},{},{},{},{:.1},{},{},{:.1},{},{},{:.3}",
            csv_field(&row.case),
            row.family.label(),
            row.parameter,
            row.kind.label(),
            csv_field(&row.conclusion),
            row.check_ms,
            row.sim_samples,
            row.sim_draws,
            row.sim_ms,
            row.sim_extrapolated,
            row.lower_bound,
            row.ratio()
        );
    }
    text
}

/// A ratio for the table: three significant figures, or a power of ten for
/// a large one.
#[must_use]
pub fn ratio_text(ratio: f64) -> String {
    if ratio >= 1000.0 {
        format!("{ratio:.2e}")
    } else if ratio >= 10.0 {
        format!("{ratio:.0}")
    } else {
        format!("{ratio:.2}")
    }
}

/// `comparison.md`: the verdict per kind, then every comparison row.
#[must_use]
pub fn markdown(rows: &[ComparisonRow]) -> String {
    let mut text = String::from("# Checking against simulating\n\n");
    let _ = writeln!(
        text,
        "Generated by `make bench`. The ratio is the time to simulate a claim's conclusion to tolerance 1/1000 (or the bound's own tolerance) with failure probability 1/1000, using Hoeffding's sample count, over the wall-clock time of `axiom check` on the whole certificate. A ratio above one means checking is cheaper. The hypothesis \"checking is much cheaper\" is confirmed for a kind when the ratio is at least {MUCH_CHEAPER:.0} on every fixture with a claim of that kind, and refuted otherwise. Times marked `*` were extrapolated linearly from a prefix of the samples. Ratios marked `>=` are lower bounds, because no finite simulation covers a whole parameter range.\n"
    );
    text.push_str("| Kind | Least ratio on the fixtures | Verdict | Scaling rows at least 10x |\n|---|---|---|---|\n");
    for verdict in verdicts(rows) {
        let _ = writeln!(
            text,
            "| {} | {} | {} | {} of {} |",
            verdict.kind.label(),
            ratio_text(verdict.fixture_ratio),
            verdict.outcome.label(),
            verdict.family_confirmed,
            verdict.family_rows
        );
    }
    text.push_str("\n| Case | Kind | Conclusion | Check (ms) | Samples | Simulate (ms) | Ratio |\n|---|---|---|---|---|---|---|\n");
    for row in rows {
        let _ = writeln!(
            text,
            "| {} | {} | {} | {:.0} | {} | {:.0}{} | {}{} |",
            row.case,
            row.kind.label(),
            row.conclusion,
            row.check_ms,
            row.sim_samples,
            row.sim_ms,
            if row.sim_extrapolated { "*" } else { "" },
            if row.lower_bound { ">= " } else { "" },
            ratio_text(row.ratio())
        );
    }
    text
}
