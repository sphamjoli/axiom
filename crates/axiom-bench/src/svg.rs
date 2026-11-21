//! SVG plots of the comparison, written by hand so they need no network,
//! no plotting library and no fonts beyond the viewer's sans-serif. The
//! same rows always give the same bytes: coordinates are printed to one
//! decimal place and nothing depends on the time or the machine.

use std::fmt::Write as _;

use crate::report::{ComparisonRow, MUCH_CHEAPER, ratio_text};

const WIDTH: f64 = 960.0;
const LABEL_WIDTH: f64 = 360.0;
const RIGHT_MARGIN: f64 = 40.0;
const TOP: f64 = 60.0;
const BAR: f64 = 20.0;
const AXIS_SPACE: f64 = 40.0;

/// Escapes text for an SVG text node or attribute.
#[must_use]
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The whole decades that cover `values`, given as base-ten logarithms,
/// widened to include `always`.
fn decades(values: impl Iterator<Item = f64>, always: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = always
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    for value in values.filter(|value| value.is_finite()) {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    let (lo, hi) = (lo.floor(), hi.ceil());
    if hi > lo { (lo, hi) } else { (lo, lo + 1.0) }
}

/// Every whole number from `lo` to `hi`, which are whole numbers.
fn whole_steps(lo: f64, hi: f64) -> Vec<f64> {
    let mut steps = Vec::new();
    let mut value = lo;
    while value <= hi {
        steps.push(value);
        value += 1.0;
    }
    steps
}

fn header(text: &mut String, height: f64, title: &str) {
    let _ = writeln!(
        text,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {WIDTH:.0} {height:.0}\" font-family=\"sans-serif\" font-size=\"12\">"
    );
    let _ = writeln!(
        text,
        "<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n<text x=\"{:.1}\" y=\"28\" font-size=\"16\" text-anchor=\"middle\">{}</text>",
        WIDTH / 2.0,
        escape(title)
    );
}

/// A horizontal bar per row: the base-ten logarithm of the ratio of
/// simulating to checking, from ratio one, with a dashed line at
/// [`MUCH_CHEAPER`]. A bar right of one is a row where checking is cheaper.
#[must_use]
pub fn ratio_chart(rows: &[ComparisonRow]) -> String {
    let logs: Vec<f64> = rows.iter().map(|row| row.ratio().log10()).collect();
    let (lo, hi) = decades(logs.iter().copied(), &[0.0, MUCH_CHEAPER.log10()]);
    let plot = WIDTH - LABEL_WIDTH - RIGHT_MARGIN;
    let x = |value: f64| LABEL_WIDTH + (value - lo) / (hi - lo) * plot;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a chart has far fewer than 2^52 rows"
    )]
    let bars_height = BAR * rows.len() as f64;
    let height = TOP + bars_height + AXIS_SPACE + 20.0;
    let mut text = String::new();
    header(
        &mut text,
        height,
        "Simulation time over checking time, per claim kind (log scale)",
    );
    let bottom = TOP + bars_height;
    for decade in whole_steps(lo, hi) {
        let at = x(decade);
        let _ = writeln!(
            text,
            "<line x1=\"{at:.1}\" y1=\"{TOP:.1}\" x2=\"{at:.1}\" y2=\"{bottom:.1}\" stroke=\"#dddddd\"/>\n<text x=\"{at:.1}\" y=\"{:.1}\" text-anchor=\"middle\">1e{decade:.0}</text>",
            bottom + 16.0
        );
    }
    let (one, ten) = (x(0.0), x(MUCH_CHEAPER.log10()));
    let _ = writeln!(
        text,
        "<line x1=\"{one:.1}\" y1=\"{TOP:.1}\" x2=\"{one:.1}\" y2=\"{bottom:.1}\" stroke=\"#333333\"/>\n<line x1=\"{ten:.1}\" y1=\"{TOP:.1}\" x2=\"{ten:.1}\" y2=\"{bottom:.1}\" stroke=\"#333333\" stroke-dasharray=\"4 3\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">ratio (simulate / check); dashed line: 10x</text>",
        LABEL_WIDTH + plot / 2.0,
        bottom + 34.0
    );
    let mut y = TOP;
    for (row, log) in rows.iter().zip(&logs) {
        let (start, end) = (one.min(x(*log)), one.max(x(*log)));
        let colour = if *log >= MUCH_CHEAPER.log10() {
            "#2b8a3e"
        } else if *log >= 0.0 {
            "#e8a33d"
        } else {
            "#c92a2a"
        };
        let label = format!("{} {}", row.case, row.kind.label());
        let _ = writeln!(
            text,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n<rect x=\"{start:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{colour}\"><title>{}</title></rect>",
            LABEL_WIDTH - 8.0,
            y + BAR * 0.7,
            escape(&label),
            y + 3.0,
            (end - start).max(1.0),
            BAR - 6.0,
            escape(&format!("{label}: {}", ratio_text(row.ratio())))
        );
        y += BAR;
    }
    text.push_str("</svg>\n");
    text
}

/// A plotted series: its name, its colour and the value it plots.
type Series = (&'static str, &'static str, fn(&Point) -> f64);

/// One measured point of a scaling plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// What grows.
    pub x: f64,
    /// The checking time in milliseconds.
    pub check_ms: f64,
    /// The simulation time in milliseconds.
    pub sim_ms: f64,
}

/// A log-log plot of checking time and simulation time against what grows
/// in one scaling family.
#[must_use]
pub fn scaling_chart(title: &str, x_label: &str, points: &[Point]) -> String {
    let (left, right, top, bottom) = (90.0, WIDTH - RIGHT_MARGIN, TOP, 420.0);
    let height = bottom + 70.0;
    let xs = decades(points.iter().map(|point| point.x.log10()), &[]);
    let ys = decades(
        points
            .iter()
            .flat_map(|point| [point.check_ms.log10(), point.sim_ms.log10()]),
        &[],
    );
    let px = |value: f64| left + (value.log10() - xs.0) / (xs.1 - xs.0) * (right - left);
    let py = |value: f64| bottom - (value.log10() - ys.0) / (ys.1 - ys.0) * (bottom - top);
    let mut text = String::new();
    header(&mut text, height, title);
    for decade in whole_steps(xs.0, xs.1) {
        let at = left + (decade - xs.0) / (xs.1 - xs.0) * (right - left);
        let _ = writeln!(
            text,
            "<line x1=\"{at:.1}\" y1=\"{top:.1}\" x2=\"{at:.1}\" y2=\"{bottom:.1}\" stroke=\"#eeeeee\"/>\n<text x=\"{at:.1}\" y=\"{:.1}\" text-anchor=\"middle\">1e{decade:.0}</text>",
            bottom + 16.0
        );
    }
    for decade in whole_steps(ys.0, ys.1) {
        let at = bottom - (decade - ys.0) / (ys.1 - ys.0) * (bottom - top);
        let _ = writeln!(
            text,
            "<line x1=\"{left:.1}\" y1=\"{at:.1}\" x2=\"{right:.1}\" y2=\"{at:.1}\" stroke=\"#eeeeee\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">1e{decade:.0} ms</text>",
            left - 6.0,
            at + 4.0
        );
    }
    let _ = writeln!(
        text,
        "<rect x=\"{left:.1}\" y=\"{top:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333333\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{} (log scale)</text>",
        right - left,
        bottom - top,
        f64::midpoint(left, right),
        bottom + 36.0,
        escape(x_label)
    );
    let series: [Series; 2] = [
        ("Lean check", "#1c7ed6", |point| point.check_ms),
        ("simulation", "#c92a2a", |point| point.sim_ms),
    ];
    for (index, (name, colour, value)) in series.iter().enumerate() {
        let path: Vec<String> = points
            .iter()
            .map(|point| format!("{:.1},{:.1}", px(point.x), py(value(point))))
            .collect();
        let _ = writeln!(
            text,
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{colour}\" stroke-width=\"2\"/>",
            path.join(" ")
        );
        for point in points {
            let _ = writeln!(
                text,
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.5\" fill=\"{colour}\"/>",
                px(point.x),
                py(value(point))
            );
        }
        let legend_y = bottom + 56.0;
        #[expect(clippy::cast_precision_loss, reason = "two series")]
        let legend_x = left + 160.0 * index as f64;
        let _ = writeln!(
            text,
            "<rect x=\"{legend_x:.1}\" y=\"{:.1}\" width=\"14\" height=\"4\" fill=\"{colour}\"/>\n<text x=\"{:.1}\" y=\"{legend_y:.1}\">{name}</text>",
            legend_y - 6.0,
            legend_x + 20.0
        );
    }
    text.push_str("</svg>\n");
    text
}
