//! The simulation side of the comparison: for each claim kind, the
//! sampling that reaches the claim's conclusion with tolerance and failure
//! probability fixed in advance, sized by Hoeffding's sample count from
//! `axiom-sim`, and the time it takes.
//!
//! Every plan is generous to simulation. A sampled mean within `epsilon` of
//! the truth with probability `1 - delta` is weaker than the exact claim
//! the checker proves, so the times here are lower bounds on what
//! simulation would need to state the same conclusion.

use std::hint::black_box;
use std::time::Instant;

use axiom_cert::{Certificate, Claim, FAILURE_ATTEMPTS, Model};
use axiom_core::{Probability, TransitionMatrix, TwoStateChain};
use axiom_sim::required_samples;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::error::BenchError;

/// The tolerance simulation is held to for every claim but a convergence
/// bound, which brings its own.
pub const EPSILON: (u64, u64) = (1, 1000);

/// The failure probability simulation is held to.
pub const DELTA: (u64, u64) = (1, 1000);

/// The points along each axis of the grid of parameter values sampled for
/// a parametric claim.
pub const GRID: u64 = 10;

/// The most samples one job runs before its time is extrapolated, which
/// bounds the memory the sampler's per-sample results take.
pub const MAX_RUN_SAMPLES: u64 = 10_000_000;

/// The longest trajectory a fixed-point plan may use.
const MAX_FIXED_POINT_STEPS: u64 = 1_000_000;

/// A kind of claim the comparison reports on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Each row sums to exactly one.
    Mass,
    /// The steady state of a two-state chain.
    FixedPoint,
    /// The contraction factor of a two-state chain.
    Contraction,
    /// A number of steps after which the chain is within a tolerance.
    ConvergenceBound,
    /// A retry loop terminates, with its expected attempts.
    Termination,
    /// Every chain in a parameter range contracts.
    ParametricContraction,
}

impl Kind {
    /// Every kind, in report order.
    pub const ALL: [Self; 6] = [
        Self::Mass,
        Self::FixedPoint,
        Self::Contraction,
        Self::ConvergenceBound,
        Self::Termination,
        Self::ParametricContraction,
    ];

    /// The kind of `claim`, or `None` for an estimate, whose value is
    /// sampled rather than checked and so has nothing to compare.
    #[must_use]
    pub const fn of(claim: &Claim) -> Option<Self> {
        match claim {
            Claim::Mass { .. } => Some(Self::Mass),
            Claim::FixedPoint { .. } => Some(Self::FixedPoint),
            Claim::Contraction { .. } => Some(Self::Contraction),
            Claim::ConvergenceBound { .. } => Some(Self::ConvergenceBound),
            Claim::Termination { .. } => Some(Self::Termination),
            Claim::ParametricContraction => Some(Self::ParametricContraction),
            _ => None,
        }
    }

    /// The name the certificate format gives the kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mass => "Mass",
            Self::FixedPoint => "FixedPoint",
            Self::Contraction => "Contraction",
            Self::ConvergenceBound => "ConvergenceBound",
            Self::Termination => "Termination",
            Self::ParametricContraction => "ParametricContraction",
        }
    }
}

/// One batch of independent samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Trajectories of `steps` steps of a chain from state `start`.
    Chain {
        /// The chain.
        matrix: TransitionMatrix,
        /// The start state.
        start: usize,
        /// The steps of each trajectory.
        steps: u64,
        /// The number of trajectories.
        samples: u64,
    },
    /// Retries of at most `attempts` attempts.
    Retry {
        /// The success probability of one attempt.
        success: Probability,
        /// The most attempts in one retry.
        attempts: u64,
        /// The number of retries.
        samples: u64,
    },
}

impl Job {
    const fn samples(&self) -> u64 {
        match self {
            Self::Chain { samples, .. } | Self::Retry { samples, .. } => *samples,
        }
    }

    /// The random draws one sample takes at most.
    const fn draws_per_sample(&self) -> u64 {
        match self {
            Self::Chain { steps, .. } => *steps,
            Self::Retry { attempts, .. } => *attempts,
        }
    }
}

/// The sampling that reaches one claim kind's conclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The batches of samples.
    pub jobs: Vec<Job>,
    /// Whether the plan covers only part of the conclusion, so that no
    /// finite simulation reaches all of it and the plan's cost is a lower
    /// bound: a grid of points standing for a parameter range.
    pub lower_bound: bool,
    /// The conclusion, in words, for the report.
    pub conclusion: String,
}

impl Plan {
    /// The total number of samples.
    #[must_use]
    pub fn samples(&self) -> u64 {
        self.jobs
            .iter()
            .map(Job::samples)
            .fold(0, u64::saturating_add)
    }

    /// The total number of random draws, at most.
    #[must_use]
    pub fn draws(&self) -> u64 {
        self.jobs
            .iter()
            .map(|job| job.samples().saturating_mul(job.draws_per_sample()))
            .fold(0, u64::saturating_add)
    }
}

fn fraction(numerator: u64, denominator: u64) -> BigRational {
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

fn epsilon() -> BigRational {
    fraction(EPSILON.0, EPSILON.1)
}

fn delta() -> BigRational {
    fraction(DELTA.0, DELTA.1)
}

fn samples(epsilon: &BigRational, delta: &BigRational) -> Result<u64, BenchError> {
    required_samples(epsilon, delta).map_err(BenchError::Simulation)
}

/// The plan for the claims of kind `kind` in `certificate`:
///
/// - `Mass` on `k` states: one step from every state, enough samples per
///   row to estimate every entry within `epsilon`, with `delta` shared by a
///   union bound over the `k * k` entries.
/// - `FixedPoint`: trajectories from the first state long enough that the
///   bias is at most `epsilon / 2`, with samples for `epsilon / 2`.
/// - `Contraction`: one step from each state, estimating each exit
///   probability within `epsilon / 2` with `delta / 2`, so the factor
///   `1 - a - b` is within `epsilon`.
/// - `ConvergenceBound` with `steps` and `tolerance`: trajectories of
///   `steps` steps from the claimed start with samples for `tolerance`.
/// - `Termination`: retries of at most the certificate's attempts, with
///   samples for `epsilon`.
/// - `ParametricContraction`: the `Contraction` plan at every point of a
///   [`GRID`] by [`GRID`] grid inside the range, a lower bound.
///
/// # Errors
///
/// Returns [`BenchError::NoSimulationPlan`] when the certificate holds no
/// claim of the kind or its model does not fit it, and the simulation error
/// when no sample count is found.
pub fn plan(kind: Kind, certificate: &Certificate) -> Result<Plan, BenchError> {
    let (epsilon, delta) = (epsilon(), delta());
    let model = certificate.model();
    match (kind, model) {
        (Kind::Mass, Model::Chain(matrix)) => {
            let states = matrix.states();
            let entries = u64::try_from(states.saturating_mul(states))
                .map_err(|_| BenchError::WorkTooLarge)?;
            let count = samples(
                &epsilon,
                &(&delta / BigRational::from_integer(entries.into())),
            )?;
            let jobs = (0..states)
                .map(|start| Job::Chain {
                    matrix: matrix.clone(),
                    start,
                    steps: 1,
                    samples: count,
                })
                .collect();
            Ok(Plan {
                jobs,
                lower_bound: false,
                conclusion: format!("each of {states} rows sums to one"),
            })
        }
        (Kind::FixedPoint, Model::Chain(matrix)) => {
            let chain = two_state(matrix)?;
            let steady = chain
                .steady_state()
                .map_err(|_| BenchError::NoSimulationPlan)?;
            let half = &epsilon / BigRational::from_integer(2.into());
            let steps = chain
                .steps_within(&BigRational::zero(), &half, MAX_FIXED_POINT_STEPS)
                .map_err(|_| BenchError::NoSimulationPlan)?
                .ok_or(BenchError::NoSimulationPlan)?;
            Ok(Plan {
                jobs: vec![Job::Chain {
                    matrix: matrix.clone(),
                    start: 0,
                    steps,
                    samples: samples(&half, &delta)?,
                }],
                lower_bound: false,
                conclusion: format!("steady state {}", steady.value()),
            })
        }
        (Kind::Contraction, Model::Chain(matrix)) => {
            let chain = two_state(matrix)?;
            Ok(Plan {
                jobs: contraction_jobs(matrix, &epsilon, &delta)?,
                lower_bound: false,
                conclusion: format!("contraction factor {}", chain.contraction_factor()),
            })
        }
        (Kind::ConvergenceBound, Model::Chain(matrix)) => {
            convergence_plan(certificate, matrix, &delta)
        }
        (Kind::Termination, Model::Retry { success }) => Ok(Plan {
            jobs: vec![Job::Retry {
                success: success.clone(),
                attempts: FAILURE_ATTEMPTS,
                samples: samples(&epsilon, &delta)?,
            }],
            lower_bound: false,
            conclusion: format!(
                "terminates with expected attempts {}",
                success.value().recip()
            ),
        }),
        (Kind::ParametricContraction, Model::Parametric { a, b }) => {
            parametric_plan(a, b, &epsilon, &delta)
        }
        _ => Err(BenchError::NoSimulationPlan),
    }
}

fn convergence_plan(
    certificate: &Certificate,
    matrix: &TransitionMatrix,
    delta: &BigRational,
) -> Result<Plan, BenchError> {
    let (start, steps, tolerance) = certificate
        .claims()
        .iter()
        .find_map(|claim| match claim {
            Claim::ConvergenceBound {
                start,
                steps,
                tolerance,
            } => Some((start, *steps, tolerance)),
            _ => None,
        })
        .ok_or(BenchError::NoSimulationPlan)?;
    let start = if start.value().is_one() {
        1
    } else if start.value().is_zero() {
        0
    } else {
        return Err(BenchError::NoSimulationPlan);
    };
    Ok(Plan {
        jobs: vec![Job::Chain {
            matrix: matrix.clone(),
            start,
            steps,
            samples: samples(tolerance, delta)?,
        }],
        lower_bound: false,
        conclusion: format!("within {tolerance} after {steps} steps"),
    })
}

fn parametric_plan(
    a: &axiom_cert::Interval,
    b: &axiom_cert::Interval,
    epsilon: &BigRational,
    delta: &BigRational,
) -> Result<Plan, BenchError> {
    let points = |lo: &Probability, hi: &Probability| -> Result<Vec<Probability>, BenchError> {
        (1..=GRID)
            .map(|i| {
                let value = lo.value() + (hi.value() - lo.value()) * fraction(i, GRID + 1);
                Probability::try_new(&value).map_err(|_| BenchError::NoSimulationPlan)
            })
            .collect()
    };
    let (xs, ys) = (points(a.lo(), a.hi())?, points(b.lo(), b.hi())?);
    let mut jobs = Vec::new();
    for x in &xs {
        for y in &ys {
            let matrix = TwoStateChain::new(x.clone(), y.clone()).matrix();
            jobs.extend(contraction_jobs(&matrix, epsilon, delta)?);
        }
    }
    Ok(Plan {
        jobs,
        lower_bound: true,
        conclusion: format!("contracts for every a and b in range ({GRID}x{GRID} grid sampled)"),
    })
}

fn two_state(matrix: &TransitionMatrix) -> Result<TwoStateChain, BenchError> {
    match matrix.rows() {
        [first, second] => match (first.probabilities().get(1), second.probabilities().first()) {
            (Some(a), Some(b)) => Ok(TwoStateChain::new(a.clone(), b.clone())),
            _ => Err(BenchError::NoSimulationPlan),
        },
        _ => Err(BenchError::NoSimulationPlan),
    }
}

fn contraction_jobs(
    matrix: &TransitionMatrix,
    epsilon: &BigRational,
    delta: &BigRational,
) -> Result<Vec<Job>, BenchError> {
    let two = BigRational::from_integer(2.into());
    let count = samples(&(epsilon / &two), &(delta / &two))?;
    Ok((0..2)
        .map(|start| Job::Chain {
            matrix: matrix.clone(),
            start,
            steps: 1,
            samples: count,
        })
        .collect())
}

/// How long a plan took.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    /// The wall-clock time in milliseconds, measured or extrapolated.
    pub ms: f64,
    /// Whether some job had more draws than the budget, so only a prefix
    /// of its samples ran and its time was scaled up linearly.
    pub extrapolated: bool,
}

/// Runs `plan` with `seed`. A job of more than `budget` draws, or more
/// than [`MAX_RUN_SAMPLES`] samples, runs only as many samples as fit both,
/// at least one, and its time is scaled by
/// the samples it skipped: each sample costs the same draws, so the time
/// is linear in the samples.
///
/// # Errors
///
/// Returns [`BenchError::WorkTooLarge`] when a sample count does not fit in
/// memory indices, and the simulation error when a start state is outside
/// its chain.
pub fn execute(plan: &Plan, budget: u64, seed: u64) -> Result<Timing, BenchError> {
    let mut ms = 0.0;
    let mut extrapolated = false;
    for job in &plan.jobs {
        let per_sample = job.draws_per_sample().max(1);
        let run = job
            .samples()
            .min((budget / per_sample).clamp(1, MAX_RUN_SAMPLES));
        let count = usize::try_from(run).map_err(|_| BenchError::WorkTooLarge)?;
        let draws =
            usize::try_from(job.draws_per_sample()).map_err(|_| BenchError::WorkTooLarge)?;
        let started = Instant::now();
        match job {
            Job::Chain { matrix, start, .. } => {
                let runs = axiom_sim::sample_chain(matrix, *start, draws, count, seed)
                    .map_err(BenchError::Simulation)?;
                black_box(runs.count(0));
            }
            Job::Retry { success, .. } => {
                let runs = axiom_sim::sample_retry(success, job.draws_per_sample(), count, seed);
                black_box(runs.successes());
            }
        }
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        if run < job.samples() {
            extrapolated = true;
        }
        ms += scaled(elapsed, job.samples(), run);
    }
    Ok(Timing { ms, extrapolated })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "sample counts below 2^62 lose at most a relative 2^-52 as f64"
)]
fn scaled(elapsed: f64, total: u64, run: u64) -> f64 {
    elapsed * (total as f64) / (run.max(1) as f64)
}
