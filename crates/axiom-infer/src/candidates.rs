use axiom_sim::{Certainty, SplitMix64, required_samples};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::black_box::{BlackBox, MAX_SAMPLES, sample_state};
use crate::error::InferError;

/// The steps of each trajectory before it restarts from a fresh uniformly
/// drawn state, so a state the process rarely enters is still visited.
pub const RESTART_STEPS: u64 = 16;

/// The largest step budget [`propose`] accepts.
pub const MAX_STEPS: u64 = 1 << 32;

/// What [`propose`] samples: the joint tolerance and failure probability of
/// the transition estimates, the seed, and the step budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The tolerance of each estimated transition probability.
    pub epsilon: BigRational,
    /// The probability that either estimate misses by `epsilon` or more.
    pub delta: BigRational,
    /// The seed the samples are drawn with.
    pub seed: u64,
    /// The most black-box steps to take, capped at [`MAX_STEPS`].
    pub max_steps: u64,
}

impl Config {
    /// The configuration with the largest step budget, [`MAX_STEPS`].
    #[must_use]
    pub fn new(epsilon: BigRational, delta: BigRational, seed: u64) -> Self {
        Self {
            epsilon,
            delta,
            seed,
            max_steps: MAX_STEPS,
        }
    }
}

/// A closed interval of rationals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds {
    /// The lower end.
    pub lower: BigRational,
    /// The upper end.
    pub upper: BigRational,
}

impl Bounds {
    /// Whether `value` lies in the interval.
    #[must_use]
    pub fn contains(&self, value: &BigRational) -> bool {
        self.lower <= *value && *value <= self.upper
    }
}

/// The steps observed from one state and how many of them left it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// The steps that left the state.
    pub exits: u64,
    /// The steps taken from the state.
    pub visits: u64,
}

impl Counts {
    /// The exit frequency `exits / visits`, zero for no visits.
    #[must_use]
    pub fn frequency(&self) -> BigRational {
        if self.visits == 0 {
            return BigRational::zero();
        }
        BigRational::new(BigInt::from(self.exits), BigInt::from(self.visits))
    }
}

/// Candidate claims about a two-state black box, from counts alone.
///
/// With `â` and `b̂` the exit frequencies of the first and second state, the
/// candidate fixed point is `â / (â + b̂)` and the candidate contraction
/// factor `1 - â - b̂`, both exact rationals of counts. With probability at
/// least `1 - delta` over the sampling, the true `a` and `b` both lie within
/// `epsilon` of their frequencies, and then the true fixed point and factor
/// lie in the bounds. Candidates are [`Certainty::Estimated`]; only a proof
/// checked by Lean can make a claim proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidates {
    /// The steps from the first state.
    pub leave_first: Counts,
    /// The steps from the second state.
    pub leave_second: Counts,
    /// The tolerance of each transition estimate.
    pub epsilon: BigRational,
    /// The joint failure probability.
    pub delta: BigRational,
    /// The candidate fixed point, absent when no step left either state.
    pub fixed_point: Option<BigRational>,
    /// The fixed points of every `a` and `b` in their intervals, absent when
    /// the intervals admit `a + b = 0`.
    pub fixed_point_bounds: Option<Bounds>,
    /// The candidate contraction factor.
    pub contraction: BigRational,
    /// The factors of every `a` and `b` in their intervals.
    pub contraction_bounds: Bounds,
}

impl Candidates {
    /// Whether the intervals rule out `|λ| = 1`. When they do not, the
    /// process may never settle, as the chain with `a = b = 1` that swaps its
    /// states every step never does, and the candidates are flagged
    /// non-convergent.
    #[must_use]
    pub fn convergent(&self) -> bool {
        let one = BigRational::one();
        self.contraction_bounds.lower > -one.clone() && self.contraction_bounds.upper < one
    }

    /// The certainty of a candidate, always [`Certainty::Estimated`].
    #[must_use]
    pub fn certainty(&self) -> Certainty {
        Certainty::Estimated
    }
}

/// Runs `black_box` and proposes [`Candidates`] from the observed steps.
///
/// Trajectories start in a uniformly drawn state and restart every
/// [`RESTART_STEPS`] steps. The first `n` steps from each state are counted,
/// where `n` is the least count Hoeffding's bound accepts for `epsilon` and
/// `delta / 2`. Given the current state, each step is a fresh independent
/// draw, so the first `n` steps from a state are independent Bernoulli
/// trials with its exit probability whatever path led there, and a union
/// bound over the two states gives the joint failure probability `delta`.
///
/// # Errors
///
/// [`InferError::NotTwoState`] for a black box without two states, the
/// [`InferError::Sim`] errors of [`required_samples`],
/// [`InferError::TooManySamples`] when `n` is above [`MAX_SAMPLES`],
/// [`InferError::StepBudgetExceeded`] when the budget runs out first, and
/// [`InferError::StateOutOfRange`] when the black box steps outside its
/// states.
pub fn propose(black_box: &impl BlackBox, config: &Config) -> Result<Candidates, InferError> {
    let states = black_box.states();
    if states != 2 {
        return Err(InferError::NotTwoState { states });
    }
    let half = &config.delta / BigInt::from(2);
    let needed = required_samples(&config.epsilon, &half)?;
    if needed > MAX_SAMPLES {
        return Err(InferError::TooManySamples { limit: MAX_SAMPLES });
    }
    let budget = config.max_steps.min(MAX_STEPS);
    let mut rng = SplitMix64::new(config.seed);
    let mut counts = [Counts::default(); 2];
    let mut state = sample_state(2, &mut rng);
    let mut since_restart = 0;
    let mut steps = 0;
    while counts.iter().any(|count| count.visits < needed) {
        if steps >= budget {
            return Err(InferError::StepBudgetExceeded { limit: budget });
        }
        if since_restart == RESTART_STEPS {
            state = sample_state(2, &mut rng);
            since_restart = 0;
        }
        let next = black_box.step(state, &mut rng);
        if next >= 2 {
            return Err(InferError::StateOutOfRange);
        }
        if let Some(count) = counts.get_mut(state)
            && count.visits < needed
        {
            count.visits += 1;
            if next != state {
                count.exits += 1;
            }
        }
        state = next;
        since_restart += 1;
        steps += 1;
    }
    let [leave_first, leave_second] = counts;
    Ok(candidates(leave_first, leave_second, config))
}

/// The candidates and their bounds from the counts.
fn candidates(leave_first: Counts, leave_second: Counts, config: &Config) -> Candidates {
    let (a, b) = (leave_first.frequency(), leave_second.frequency());
    let around = |centre: &BigRational| Bounds {
        lower: (centre - &config.epsilon).max(BigRational::zero()),
        upper: (centre + &config.epsilon).min(BigRational::one()),
    };
    let (a_range, b_range) = (around(&a), around(&b));
    let sum = &a + &b;
    let fixed_point = (!sum.is_zero()).then(|| &a / &sum);
    // `a / (a + b)` rises with `a` and falls with `b`, so its extremes are at
    // the corners; it is defined on the whole box only when `a + b > 0`
    // there, that is when the lower ends do not sum to zero.
    let fixed_point_bounds = (&a_range.lower + &b_range.lower)
        .is_positive()
        .then(|| Bounds {
            lower: &a_range.lower / (&a_range.lower + &b_range.upper),
            upper: &a_range.upper / (&a_range.upper + &b_range.lower),
        });
    let one = BigRational::one();
    Candidates {
        leave_first,
        leave_second,
        epsilon: config.epsilon.clone(),
        delta: config.delta.clone(),
        fixed_point,
        fixed_point_bounds,
        contraction: &one - sum,
        contraction_bounds: Bounds {
            lower: &one - &a_range.upper - &b_range.upper,
            upper: &one - &a_range.lower - &b_range.lower,
        },
    }
}
