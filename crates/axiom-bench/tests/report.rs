//! The verdict rule, the CSV encoding and the plots: the verdict is decided
//! on the fixtures by the least ratio, fields with commas survive the CSV,
//! and the same rows always draw the same SVG.

mod common;

use axiom_bench::cases::Family;
use axiom_bench::report::{self, ComparisonRow, Outcome};
use axiom_bench::simulate::Kind;
use axiom_bench::svg;
use common::{TestError, ensure};

fn row(case: &str, family: Family, kind: Kind, check_ms: f64, sim_ms: f64) -> ComparisonRow {
    ComparisonRow {
        case: case.to_owned(),
        family,
        parameter: 0,
        kind,
        conclusion: "within 1/1000 after 13 steps".to_owned(),
        check_ms,
        sim_samples: 1,
        sim_draws: 1,
        sim_ms,
        sim_extrapolated: false,
        lower_bound: false,
    }
}

#[test]
fn a_kind_is_confirmed_only_when_every_fixture_is_ten_times_cheaper() -> Result<(), TestError> {
    let rows = [
        row("a", Family::Fixture, Kind::Mass, 100.0, 1200.0),
        row("b", Family::Fixture, Kind::Mass, 100.0, 900.0),
        row("c", Family::Fixture, Kind::Termination, 100.0, 1000.0),
        row("d", Family::Horizon, Kind::Termination, 100.0, 10.0),
        row("e", Family::Tolerance, Kind::Contraction, 100.0, 5000.0),
    ];
    let verdicts = report::verdicts(&rows);
    let summary: Vec<(Kind, Outcome, usize, usize)> = verdicts
        .iter()
        .map(|v| (v.kind, v.outcome, v.family_rows, v.family_confirmed))
        .collect();
    ensure(
        summary
            == [
                (Kind::Mass, Outcome::Refuted, 0, 0),
                (Kind::Termination, Outcome::Confirmed, 1, 0),
            ],
        format!("unexpected verdicts {summary:?}"),
    )?;
    ensure(
        verdicts
            .first()
            .is_some_and(|v| (v.fixture_ratio - 9.0).abs() < 1e-9),
        "the Mass verdict does not report the least fixture ratio",
    )
}

#[test]
fn a_conclusion_with_a_comma_stays_one_csv_field() -> Result<(), TestError> {
    let mut quoted = row("links", Family::Fixture, Kind::Mass, 2.0, 4.0);
    quoted.conclusion = "rows 0, 1 sum to \"one\"".to_owned();
    let text = report::comparison_csv(&[quoted]);
    let mut lines = text.lines();
    ensure(
        lines.next() == Some(report::COMPARISON_HEADER),
        "the header is missing",
    )?;
    ensure(
        lines.next()
            == Some(
                "links,fixture,0,Mass,\"rows 0, 1 sum to \"\"one\"\"\",2.0,1,1,4.0,false,false,2.000",
            ),
        format!("unexpected line in {text}"),
    )
}

#[test]
fn the_ratio_chart_is_deterministic_with_one_bar_per_row() -> Result<(), TestError> {
    let rows = [
        row(
            "links",
            Family::Fixture,
            Kind::ConvergenceBound,
            900.0,
            90.0,
        ),
        row(
            "tolerance-6",
            Family::Tolerance,
            Kind::ConvergenceBound,
            900.0,
            9.0e8,
        ),
    ];
    let first = svg::ratio_chart(&rows);
    ensure(first == svg::ratio_chart(&rows), "two draws differ")?;
    ensure(
        first.starts_with("<svg ") && first.ends_with("</svg>\n"),
        "not one SVG document",
    )?;
    ensure(
        first.matches("<title>").count() == rows.len(),
        "not one bar per row",
    )?;
    ensure(
        first.contains("fill=\"#c92a2a\"><title>links ConvergenceBound")
            && first.contains("fill=\"#2b8a3e\"><title>tolerance-6 ConvergenceBound"),
        "a bar's colour does not follow its ratio",
    )
}
