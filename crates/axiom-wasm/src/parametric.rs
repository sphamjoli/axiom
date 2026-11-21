//! Parametric two-state chains for the browser.
//!
//! A two-state chain whose exit probabilities are its two parameters `a`
//! and `b` gets the `ParametricContraction` certificate, which holds for
//! every point of the box, and one point of the box analysed as a fixed
//! chain: the midpoint of each interval by default, or a point the page
//! picks. The point is untrusted input: [`parse_point`] bounds its size and
//! digits and checks it lies inside both intervals, open ends excluded.

use axiom_cert::{Certificate, End, Interval};
use axiom_core::{Probability, TwoStateChain};
use axiom_model::{Chain, Parameter};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Fraction, fraction_text};

/// The longest point request accepted, in bytes.
pub const MAX_POINT_BYTES: usize = 256;

/// The most decimal digits of each part of a point's fractions.
const MAX_POINT_DIGITS: usize = 9;

/// A point the engine refuses. Every message is fixed and never repeats the
/// request or the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PointError {
    /// The point request is longer than [`MAX_POINT_BYTES`].
    #[error("the point is longer than 256 bytes")]
    TooLarge,
    /// The point is not JSON of the shape `{"a": fraction, "b": fraction}`.
    #[error("the point is not a valid point request")]
    Malformed,
    /// A value is not a fraction of short decimal integers.
    #[error("a and b must be fractions of decimal integers of at most 9 digits")]
    InvalidFraction,
    /// A value lies outside its parameter's interval, or on an open end.
    #[error("the point lies outside the parameter ranges")]
    OutOfRange,
    /// The model has no parameters the browser can pick a point for.
    #[error("the model has no parameters the browser can pick a point for")]
    NotParametric,
}

impl PointError {
    /// A stable kebab-case name for the error, the same on every machine.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "point-too-large",
            Self::Malformed => "malformed-point",
            Self::InvalidFraction => "invalid-point-fraction",
            Self::OutOfRange => "point-out-of-range",
            Self::NotParametric => "not-parametric",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPoint {
    a: RawFraction,
    b: RawFraction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFraction {
    num: String,
    den: String,
}

/// Exact values of `a` and `b`.
pub(crate) struct Point {
    a: BigRational,
    b: BigRational,
}

fn decimal(text: &str) -> Option<BigInt> {
    let valid = !text.is_empty()
        && text.len() <= MAX_POINT_DIGITS
        && text.bytes().all(|byte| byte.is_ascii_digit());
    if valid { text.parse().ok() } else { None }
}

fn fraction(raw: &RawFraction) -> Result<BigRational, PointError> {
    let num = decimal(&raw.num).ok_or(PointError::InvalidFraction)?;
    let den = decimal(&raw.den)
        .filter(|den| !den.is_zero())
        .ok_or(PointError::InvalidFraction)?;
    Ok(BigRational::new(num, den))
}

/// Reads a point request, checking its size and the digits of each value.
/// Whether it lies in the box is checked against the model by
/// [`Box::contains`].
pub(crate) fn parse_point(json: &str) -> Result<Point, PointError> {
    if json.len() > MAX_POINT_BYTES {
        return Err(PointError::TooLarge);
    }
    let raw: RawPoint = serde_json::from_str(json).map_err(|_| PointError::Malformed)?;
    Ok(Point {
        a: fraction(&raw.a)?,
        b: fraction(&raw.b)?,
    })
}

/// The box of a parametric two-state chain: the names and intervals of `a`
/// and `b`.
pub(crate) struct ParamBox<'chain> {
    a: &'chain Parameter,
    b: &'chain Parameter,
    certificate: Certificate,
}

/// Why a parametric chain is not the shape the browser analyses.
pub(crate) enum Unsupported {
    /// The chain does not have two states.
    States(usize),
    /// The exit probabilities are not exactly the two parameters.
    Exits,
    /// The range includes `a + b = 0` or `a + b = 2`.
    Range,
}

impl Unsupported {
    /// What the browser supports and why this model is not it.
    pub(crate) fn notice(&self) -> String {
        let reason = match self {
            Self::States(states) => format!("This model has {states} states"),
            Self::Exits => String::from(
                "In this model the exit probabilities are not exactly its two parameters",
            ),
            Self::Range => String::from(
                "This model's ranges include a + b = 0 or a + b = 2, where the chain never settles",
            ),
        };
        format!(
            "{SUPPORTED} {reason}, so replace its parameters with values in the text to analyse one instance."
        )
    }
}

/// The parametric shape the browser analyses, in plain words.
pub(crate) const SUPPORTED: &str = "The browser analyses parameters for two-state chains whose exit probabilities are the parameters, such as links-param.";

/// The notice for a retry with parameters.
pub(crate) const RETRY_NOTICE: &str = "The browser analyses parameters for two-state chains whose exit probabilities are the parameters, such as links-param. This model is a retry, so replace its parameters with values in the text to analyse one instance.";

fn interval(parameter: &Parameter) -> Option<Interval> {
    let (lower, upper) = (&parameter.lower, &parameter.upper);
    Interval::from_bounds(&lower.value, lower.closed, &upper.value, upper.closed).ok()
}

impl<'chain> ParamBox<'chain> {
    /// The box of `chain`, or why the browser does not analyse it.
    pub(crate) fn of(chain: &'chain Chain) -> Result<Self, Unsupported> {
        if chain.states.len() != 2 {
            return Err(Unsupported::States(chain.states.len()));
        }
        let (a, b) = chain.two_state_exits().ok_or(Unsupported::Exits)?;
        let (Some(a_range), Some(b_range)) = (interval(a), interval(b)) else {
            return Err(Unsupported::Exits);
        };
        let certificate =
            Certificate::parametric(a_range, b_range).map_err(|_| Unsupported::Range)?;
        Ok(Self { a, b, certificate })
    }

    pub(crate) fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// The midpoint of each interval, which lies inside it whatever its ends.
    pub(crate) fn midpoint(&self) -> Point {
        let two = BigRational::from_integer(BigInt::from(2));
        let mid = |p: &Parameter| (&p.lower.value + &p.upper.value) / &two;
        Point {
            a: mid(self.a),
            b: mid(self.b),
        }
    }

    /// The chain at `point`, if the point lies inside the box.
    pub(crate) fn instance(&self, point: &Point) -> Result<TwoStateChain, PointError> {
        if !(inside(self.a, &point.a) && inside(self.b, &point.b)) {
            return Err(PointError::OutOfRange);
        }
        let probability =
            |value: &BigRational| Probability::try_new(value).map_err(|_| PointError::OutOfRange);
        Ok(TwoStateChain::new(
            probability(&point.a)?,
            probability(&point.b)?,
        ))
    }

    pub(crate) fn view(&self, chain: &Chain, point: &Point) -> ParametricView {
        let sum_lo = &self.a.lower.value + &self.b.lower.value;
        let sum_hi = &self.a.upper.value + &self.b.upper.value;
        let lo_closed = self.a.lower.closed && self.b.lower.closed;
        let hi_closed = self.a.upper.closed && self.b.upper.closed;
        let one = BigRational::one();
        ParametricView {
            states: chain.states.clone(),
            a: ParameterView::from(self.a),
            b: ParameterView::from(self.b),
            steady_state: "a/(a+b)",
            factor: "1 - a - b",
            sum: RangeView {
                lo: Fraction::from(&sum_lo),
                lo_closed,
                hi: Fraction::from(&sum_hi),
                hi_closed,
            },
            lambda: RangeView {
                lo: Fraction::from(&(&one - &sum_hi)),
                lo_closed: hi_closed,
                hi: Fraction::from(&(&one - &sum_lo)),
                hi_closed: lo_closed,
            },
            point: PointView {
                a: Fraction::from(&point.a),
                b: Fraction::from(&point.b),
                source: instance_source(chain, &point.a, &point.b),
            },
        }
    }
}

fn inside(parameter: &Parameter, value: &BigRational) -> bool {
    let (lower, upper) = (&parameter.lower, &parameter.upper);
    let above = if lower.closed {
        value >= &lower.value
    } else {
        value > &lower.value
    };
    let below = if upper.closed {
        value <= &upper.value
    } else {
        value < &upper.value
    };
    above && below
}

/// The model text of the chain at `a` and `b`, without parameters, so the
/// page can simulate the instance. Zero weights are left out.
fn instance_source(chain: &Chain, a: &BigRational, b: &BigRational) -> String {
    let [first, second] = chain.states.as_slice() else {
        return String::new();
    };
    let one = BigRational::one();
    let row = |weights: [(&String, BigRational); 2]| {
        weights
            .iter()
            .filter(|(_, weight)| !weight.is_zero())
            .map(|(state, weight)| format!("{state} {}", fraction_text(weight)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "chain {} {{\n  states {first}, {second}\n  from {first}: {}\n  from {second}: {}\n}}\n",
        chain.name,
        row([(first, &one - a), (second, a.clone())]),
        row([(first, b.clone()), (second, &one - b)]),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParametricView {
    states: Vec<String>,
    a: ParameterView,
    b: ParameterView,
    steady_state: &'static str,
    factor: &'static str,
    sum: RangeView,
    lambda: RangeView,
    point: PointView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParameterView {
    name: String,
    range: RangeView,
}

impl From<&Parameter> for ParameterView {
    fn from(parameter: &Parameter) -> Self {
        Self {
            name: parameter.name.clone(),
            range: RangeView {
                lo: Fraction::from(&parameter.lower.value),
                lo_closed: parameter.lower.closed,
                hi: Fraction::from(&parameter.upper.value),
                hi_closed: parameter.upper.closed,
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RangeView {
    lo: Fraction,
    lo_closed: bool,
    hi: Fraction,
    hi_closed: bool,
}

#[derive(Serialize)]
struct PointView {
    a: Fraction,
    b: Fraction,
    source: String,
}

/// The text of an interval end, for claim statements: `]0` or `1[`.
pub(crate) fn interval_text(interval: &Interval) -> String {
    let lo = if interval.lo_end() == End::Closed {
        "["
    } else {
        "]"
    };
    let hi = if interval.hi_end() == End::Closed {
        "]"
    } else {
        "["
    };
    format!(
        "{lo}{}, {}{hi}",
        fraction_text(interval.lo().value()),
        fraction_text(interval.hi().value())
    )
}
