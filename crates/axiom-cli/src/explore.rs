//! The `explore` command: black-box analysis with a promotion path.
//!
//! The chain is wrapped in an [`axiom_infer::MatrixBox`], so the analysis
//! sees only sampled steps, and [`axiom_infer::propose`] turns them into a
//! candidate fixed point and contraction factor. A candidate is labelled
//! proven only when the model is supplied, the engine wrote its exact claim
//! into the certificate and the Lean checker accepted that claim; in every
//! other case, including `--black-box`, it is labelled estimated. The label
//! is decided here from the checker's verdicts as [`check`] parses them,
//! never from text in the model or the certificate.

use std::fmt;
use std::path::Path;

use axiom_cert::{Certificate, Claim, Quantity};
use axiom_infer::{Bounds, Candidates, Config, MatrixBox};
use axiom_model::CheckedModel;
use num_rational::BigRational;
use num_traits::One;

use crate::analyse::{STEADY_STATE_STEPS, read_model, sample_count, with_sampled_estimate};
use crate::check;
use crate::cli::{CheckerArgs, EstimateRequest};
use crate::error::{CliError, FileRole, IoFailure};

/// The label of a candidate whose exact claim the Lean checker accepted.
pub const PROVEN: &str = "proven (checked by Lean)";

/// The label of every other candidate.
pub const ESTIMATED: &str = "estimated (sampled, not a proof)";

/// What black-box results cannot show, printed after every exploration.
pub const LIMITATION: &str = "limitation: black-box results cannot show unsampled regions of the parameter space or transitions rarer than the sampling resolution epsilon, and an estimate is not a proof";

/// Whether the model is supplied to the promotion path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The engine writes its exact claims and the checker may prove the
    /// candidates.
    ModelSupplied,
    /// The analysis uses samples only and every candidate is estimated.
    BlackBox,
}

/// What `explore` found, for printing.
#[derive(Debug)]
pub struct Exploration {
    lines: Vec<String>,
    all_accepted: bool,
}

impl Exploration {
    /// Whether the checker accepted every claim of the certificate.
    #[must_use]
    pub fn all_accepted(&self) -> bool {
        self.all_accepted
    }
}

impl fmt::Display for Exploration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(formatter, "{line}")?;
        }
        Ok(())
    }
}

/// Runs the chain in `model` as a black box with the request's seed,
/// tolerance and failure probability, writes a certificate to `cert`,
/// checks it with the Lean checker and labels each candidate.
///
/// With [`Mode::ModelSupplied`] the certificate holds the engine's exact
/// claims, [`Certificate::strongest`]; with [`Mode::BlackBox`] it holds only
/// the `Mass` claims of the model the format must embed. Either way it ends
/// with an `Estimate` of the steady state drawn through the black box.
///
/// # Errors
///
/// Returns [`CliError::ExploreUnsupported`] for a retry or a chain with
/// parameters, the errors of reading the model, of
/// [`axiom_infer::propose`] and of the estimate, [`CliError::WriteFailed`]
/// when the certificate cannot be written, and the errors of [`check::run`].
pub fn run(
    model: &Path,
    cert: &Path,
    mode: Mode,
    request: &EstimateRequest,
    checker: &CheckerArgs,
) -> Result<Exploration, CliError> {
    let CheckedModel::Chain(chain) = axiom_model::parse(&read_model(model)?)? else {
        return Err(CliError::ExploreUnsupported);
    };
    // A chain without two states is refused by `propose`.
    let matrix = chain.matrix.ok_or(CliError::ExploreUnsupported)?;
    let black_box = MatrixBox::new(&matrix);
    let config = Config::new(request.epsilon.clone(), request.delta.clone(), request.seed);
    let found = axiom_infer::propose(&black_box, &config)?;
    let base = match mode {
        Mode::ModelSupplied => Certificate::strongest(matrix),
        Mode::BlackBox => Certificate::mass(matrix),
    };
    let samples = sample_count(&base, request)?;
    let hits =
        axiom_infer::hits_after(&black_box, 0, STEADY_STATE_STEPS, samples, 1, request.seed)?;
    let certificate = with_sampled_estimate(base, Quantity::SteadyState, hits, samples, request)?;
    std::fs::write(cert, certificate.to_json()?).map_err(|error| CliError::WriteFailed {
        file: FileRole::Certificate,
        failure: IoFailure::from(&error),
    })?;
    let report = check::run(cert, checker.lean_dir.as_deref(), checker.timeout())?;
    // A black-box certificate holds no `FixedPoint` or `Contraction` claim,
    // so nothing the checker accepts can promote a black-box candidate.
    let proven = |kind: &str| report.proven(kind);
    let exact_fixed_point = certificate.claims().iter().find_map(|claim| match claim {
        Claim::FixedPoint { value } => Some(value.value().clone()),
        _ => None,
    });
    let exact_factor = certificate.claims().iter().find_map(|claim| match claim {
        Claim::Contraction { factor } => Some(factor.clone()),
        _ => None,
    });
    let mut lines = vec![
        match mode {
            Mode::ModelSupplied => "explore: two-state chain, model supplied to the checker",
            Mode::BlackBox => "explore: two-state chain, black box: the matrix is hidden",
        }
        .to_owned(),
        sampling_line(&found, request),
        candidate_line(
            "fixed point",
            found.fixed_point.as_ref(),
            found.fixed_point_bounds.as_ref(),
            exact_fixed_point.filter(|_| proven("FixedPoint")),
        ),
        candidate_line(
            "contraction factor",
            Some(&found.contraction),
            Some(&found.contraction_bounds),
            exact_factor.filter(|_| proven("Contraction")),
        ),
    ];
    if !found.convergent() {
        lines.push(
            "non-convergent: the bounds admit |lambda| = 1, so the process may never settle"
                .to_owned(),
        );
    }
    let estimate = certificate.claims().iter().find_map(|claim| match claim {
        Claim::Estimate(estimate) => Some(estimate.value.value().clone()),
        _ => None,
    });
    if let Some(value) = estimate {
        lines.push(format!(
            "estimate steady-state {value} from {samples} trajectories of {STEADY_STATE_STEPS} steps: {ESTIMATED}"
        ));
    }
    lines.push(format!(
        "checker: {} claims in the certificate, all accepted: {}",
        report.claims(),
        if report.all_accepted() { "yes" } else { "no" }
    ));
    lines.push(LIMITATION.to_owned());
    Ok(Exploration {
        lines,
        all_accepted: report.all_accepted(),
    })
}

/// The line that states the sample counts and the joint confidence.
fn sampling_line(found: &Candidates, request: &EstimateRequest) -> String {
    let confidence = BigRational::one() - &found.delta;
    format!(
        "samples: {} steps from each state with seed {}; both exit probabilities within {} with probability at least {confidence}",
        found.leave_first.visits, request.seed, found.epsilon
    )
}

/// The line for one candidate: its value, its bounds and its label, with
/// the exact value when the checker proved the engine's claim.
fn candidate_line(
    name: &str,
    value: Option<&BigRational>,
    bounds: Option<&Bounds>,
    proven: Option<BigRational>,
) -> String {
    let value = value.map_or_else(|| "none".to_owned(), ToString::to_string);
    let bounds = bounds.map_or_else(
        || "unbounded".to_owned(),
        |bounds| format!("[{}, {}]", bounds.lower, bounds.upper),
    );
    let label = match proven {
        Some(exact) => format!("{PROVEN}, exact value {exact}"),
        None => ESTIMATED.to_owned(),
    };
    format!("candidate {name} {value} in {bounds}: {label}")
}
