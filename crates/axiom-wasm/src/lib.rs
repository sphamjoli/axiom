//! The axiom engine for the browser.
//!
//! [`analyse`] reads a `.axm` model and returns its analysis as JSON in the
//! shape the web interface's `AnalyzeResult` type expects. Exact quantities
//! are fractions of decimal integer strings, `{"num": "1", "den": "4"}`;
//! floats appear only as `approx`, for plotting. A rejected model yields
//! diagnostics with a line and a column, and their messages never quote the
//! source.
//!
//! [`simulate`] runs seeded simulations of a model with `axiom-sim` and
//! returns sample paths, running fractions, histograms and a Hoeffding
//! estimate, bounded by the caps in this crate's constants.
//!
//! The certificate is the one `axiom analyse` writes, but the browser does
//! not run the Lean checker, so every claim is reported as unchecked.
//!
//! ```
//! let json = axiom_wasm::analyse("retry request {\n  success 3/10\n}\n");
//! assert!(json.contains("\"expectedAttempts\":{\"num\":\"10\",\"den\":\"3\"}"));
//! ```

use axiom_cert::{Certificate, Claim};
use axiom_core::{Probability, TransitionMatrix, TwoStateChain};
use axiom_model::{CheckedModel, ModelError};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

mod simulation;

pub use simulation::{
    MAX_ATTEMPTS, MAX_HISTOGRAM_BUCKETS, MAX_PATH_STEPS, MAX_REQUEST_BYTES, MAX_RUNNING_POINTS,
    MAX_SAMPLES, MAX_SOURCE_BYTES, MAX_STEPS, PATHS, RequestError,
};

/// The distance from the steady state the convergence horizon is measured
/// against: 1/1000.
const TOLERANCE_DENOMINATOR: u32 = 1000;

/// The most steps of the convergence series, so a slowly contracting chain
/// cannot make the analysis run for long.
const MAX_SERIES_STEPS: usize = 200;

/// How many steps past the horizon the series continues, to show it settle.
const STEPS_AFTER_HORIZON: usize = 7;

/// Analyses the model in `source` and returns the result as JSON.
///
/// The result is `{"ok": true, "analysis": ...}` for an accepted model and
/// `{"ok": false, "diagnostics": [...]}` for a rejected one.
#[wasm_bindgen]
#[must_use]
pub fn analyse(source: &str) -> String {
    let result = match axiom_model::parse(source) {
        Ok(model) => AnalyzeResult::Accepted {
            ok: true,
            analysis: Box::new(analyse_model(model)),
        },
        Err(error) => AnalyzeResult::Rejected {
            ok: false,
            diagnostics: vec![Diagnostic::from_error(&error)],
        },
    };
    serde_json::to_string(&result).unwrap_or_else(|_| {
        String::from(
            r#"{"ok":false,"diagnostics":[{"line":1,"column":1,"code":"internal","message":"the engine could not encode its result"}]}"#,
        )
    })
}

/// Simulates the model in `source` as `request_json` asks and returns the
/// result as JSON.
///
/// The request is `{"seed": "<u64 in decimal>", "samples": n, "steps": s,
/// "maxAttempts": m}` with optional `"epsilon"` and `"delta"` fractions such
/// as `{"num": "1", "den": "100"}`, which default to 1/100 and 1/1000. The
/// seed is a string because a JavaScript number cannot hold every `u64`.
/// The result is `{"ok": true, "simulation": ...}`, or `{"ok": false,
/// "error": {"code", "message", "line", "column"}}` with a stable code and a
/// fixed message that never repeats the request or the model.
///
/// ```
/// let json = axiom_wasm::simulate(
///     "retry request {\n  success 3/10\n}\n",
///     r#"{"seed": "7", "samples": 100, "steps": 10, "maxAttempts": 50}"#,
/// );
/// assert!(json.starts_with(r#"{"ok":true"#));
/// ```
#[wasm_bindgen]
#[must_use]
pub fn simulate(source: &str, request_json: &str) -> String {
    simulation::simulate(source, request_json)
}

/// The outcome of [`analyse`].
#[derive(Serialize)]
#[serde(untagged)]
enum AnalyzeResult {
    /// The model passed every check.
    Accepted {
        /// Always `true`.
        ok: bool,
        /// What the engine computed.
        analysis: Box<Analysis>,
    },
    /// The model was rejected.
    Rejected {
        /// Always `false`.
        ok: bool,
        /// Why, with positions.
        diagnostics: Vec<Diagnostic>,
    },
}

/// A fault in the model, without any text of the model.
#[derive(Serialize)]
struct Diagnostic {
    line: usize,
    column: usize,
    code: &'static str,
    message: String,
}

impl Diagnostic {
    fn from_error(error: &ModelError) -> Self {
        let at = error.position();
        let full = error.to_string();
        let prefix = format!("{at}: ");
        let message = full.strip_prefix(&prefix).unwrap_or(&full).to_owned();
        Self {
            line: at.line,
            column: at.column,
            code: error.code(),
            message,
        }
    }
}

/// An exact fraction in lowest terms, as decimal strings.
#[derive(Serialize)]
struct Fraction {
    num: String,
    den: String,
}

impl From<&BigRational> for Fraction {
    fn from(value: &BigRational) -> Self {
        Self {
            num: value.numer().to_string(),
            den: value.denom().to_string(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Analysis {
    name: String,
    chain: Option<Chain>,
    convergence: Option<Convergence>,
    retry: Option<Retry>,
    certificate: Option<CertificateView>,
    notice: Option<&'static str>,
}

#[derive(Serialize)]
struct Chain {
    name: String,
    states: Vec<String>,
    transitions: Vec<Transition>,
}

#[derive(Serialize)]
struct Transition {
    from: String,
    to: String,
    p: Fraction,
}

#[derive(Serialize)]
struct SeriesPoint {
    t: usize,
    value: Fraction,
    approx: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Convergence {
    state: String,
    a: Fraction,
    b: Fraction,
    initial: Fraction,
    fixed_point: Fraction,
    epsilon: Fraction,
    horizon: Option<usize>,
    series: Vec<SeriesPoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Retry {
    success: Fraction,
    success_probability: Fraction,
    expected_attempts: Option<Fraction>,
}

#[derive(Serialize)]
struct ClaimView {
    id: String,
    statement: String,
    status: &'static str,
    detail: String,
}

#[derive(Serialize)]
struct CertificateView {
    version: u32,
    claims: Vec<ClaimView>,
    json: String,
}

fn analyse_model(model: CheckedModel) -> Analysis {
    match model {
        CheckedModel::Chain(chain) => {
            let Some(matrix) = chain.matrix else {
                return Analysis {
                    name: chain.name,
                    chain: None,
                    convergence: None,
                    retry: None,
                    certificate: None,
                    notice: Some(
                        "The model is valid. Chains with parameters are not yet analysed in the browser.",
                    ),
                };
            };
            let view = chain_view(&chain.name, &chain.states, &matrix);
            let convergence = convergence(&chain.states, &matrix);
            let certificate = certificate_view(&chain.states, &Certificate::strongest(matrix));
            Analysis {
                name: chain.name,
                chain: Some(view),
                convergence,
                retry: None,
                certificate,
                notice: None,
            }
        }
        CheckedModel::Retry(retry) => {
            let Some(success) = retry.probability else {
                return Analysis {
                    name: retry.name,
                    chain: None,
                    convergence: None,
                    retry: None,
                    certificate: None,
                    notice: Some(
                        "The model is valid. Retries with parameters are not yet analysed in the browser.",
                    ),
                };
            };
            Analysis {
                name: retry.name,
                chain: None,
                convergence: None,
                retry: Some(retry_view(&success)),
                certificate: None,
                notice: None,
            }
        }
    }
}

fn chain_view(name: &str, states: &[String], matrix: &TransitionMatrix) -> Chain {
    let transitions = states
        .iter()
        .zip(matrix.rows())
        .flat_map(|(from, row)| {
            states
                .iter()
                .zip(row.probabilities())
                .map(move |(to, p)| Transition {
                    from: from.clone(),
                    to: to.clone(),
                    p: Fraction::from(p.value()),
                })
        })
        .collect();
    Chain {
        name: name.to_owned(),
        states: states.to_vec(),
        transitions,
    }
}

/// The two-state chain of `matrix`, if it has two states.
fn two_state(matrix: &TransitionMatrix) -> Option<TwoStateChain> {
    if let [first, second] = matrix.rows()
        && let (Some(leave_first), Some(leave_second)) =
            (first.probabilities().get(1), second.probabilities().first())
    {
        return Some(TwoStateChain::new(
            leave_first.clone(),
            leave_second.clone(),
        ));
    }
    None
}

/// The iterates of `d' = lambda d + offset` from `d0 = 1`, for a two-state
/// chain with a unique, attracting steady state, and the first step from
/// which they stay within 1/1000 of it.
fn convergence(states: &[String], matrix: &TransitionMatrix) -> Option<Convergence> {
    let chain = two_state(matrix)?;
    let state = states.get(1)?.clone();
    let fixed = chain.steady_state().ok()?;
    let lambda = chain.contraction_factor();
    if lambda.abs() >= BigRational::one() {
        return None;
    }
    let update = chain.second_state_update();
    let epsilon = BigRational::new(BigInt::one(), BigInt::from(TOLERANCE_DENOMINATOR));
    let initial = BigRational::one();
    let mut series = Vec::new();
    let mut horizon = None;
    for (t, value) in update
        .iterate(initial.clone())
        .take(MAX_SERIES_STEPS + 1)
        .enumerate()
    {
        // With |lambda| < 1 the distance to the fixed point never grows, so
        // the first step within tolerance is the horizon.
        if horizon.is_none() && (&value - fixed.value()).abs() <= epsilon {
            horizon = Some(t);
        }
        series.push(SeriesPoint {
            t,
            approx: value.to_f64().unwrap_or(f64::NAN),
            value: Fraction::from(&value),
        });
        if horizon.is_some_and(|h| t >= h + STEPS_AFTER_HORIZON) {
            break;
        }
    }
    Some(Convergence {
        state,
        a: Fraction::from(update.factor()),
        b: Fraction::from(update.offset()),
        initial: Fraction::from(&initial),
        fixed_point: Fraction::from(fixed.value()),
        epsilon: Fraction::from(&epsilon),
        horizon,
        series,
    })
}

fn retry_view(success: &Probability) -> Retry {
    let p = success.value();
    let (eventual, attempts) = if p.is_zero() {
        (BigRational::zero(), None)
    } else {
        (BigRational::one(), Some(Fraction::from(&p.recip())))
    };
    Retry {
        success: Fraction::from(p),
        success_probability: Fraction::from(&eventual),
        expected_attempts: attempts,
    }
}

fn fraction_text(value: &BigRational) -> String {
    if value.is_integer() {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

const UNCHECKED: &str = "unchecked";
const NOT_CHECKED: &str = "Computed by the engine; the Lean checker has not run on it.";

fn certificate_view(states: &[String], certificate: &Certificate) -> Option<CertificateView> {
    let json = certificate.to_json().ok()?;
    let second = states.get(1).map_or("the second state", String::as_str);
    let claims = certificate
        .claims()
        .iter()
        .enumerate()
        .map(|(index, claim)| match claim {
            Claim::Mass { row } => ClaimView {
                id: format!("mass-{row}"),
                statement: format!(
                    "Row {} sums to 1",
                    states.get(*row).map_or("?", String::as_str)
                ),
                status: UNCHECKED,
                detail: format!("Every entry is a probability. {NOT_CHECKED}"),
            },
            Claim::FixedPoint { value } => ClaimView {
                id: String::from("fixed-point"),
                statement: format!(
                    "P({second}) = {} is a fixed point of the update",
                    fraction_text(value.value())
                ),
                status: UNCHECKED,
                detail: format!("The steady state of the chain. {NOT_CHECKED}"),
            },
            Claim::Contraction { factor } => ClaimView {
                id: String::from("contraction"),
                statement: format!(
                    "The update contracts with factor {}",
                    fraction_text(factor)
                ),
                status: UNCHECKED,
                detail: format!(
                    "Its absolute value is below 1, so every start converges to the fixed point. {NOT_CHECKED}"
                ),
            },
            _ => ClaimView {
                id: format!("claim-{index}"),
                statement: String::from("A claim of a kind this page cannot show"),
                status: UNCHECKED,
                detail: String::from(NOT_CHECKED),
            },
        })
        .collect();
    Some(CertificateView {
        version: certificate.version(),
        claims,
        json,
    })
}
