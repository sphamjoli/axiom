use axiom_core::{Probability, TransitionMatrix};
use axiom_model::{Chain, Retry};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::ToPrimitive;

use crate::error::SimError;
use crate::rng::SplitMix64;

/// `2^64`, the number of distinct draws.
const DRAWS: u128 = 1 << 64;

/// An exact cut point for a uniform 64-bit draw.
///
/// For a cumulative probability `c`, the threshold is `t = ⌈c · 2^64⌉`, and
/// a draw `u` falls below the cut when `u < t`, which for an integer `u` is
/// the same as `u < c · 2^64`, or `u · den < num · 2^64` for `c = num/den`.
/// The comparison is exact integer arithmetic, so the event has probability
/// exactly `t / 2^64`, which lies in `[c, c + 2^-64)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threshold(u128);

impl Threshold {
    /// The threshold of the cumulative probability `cumulative`.
    #[must_use]
    pub fn of(cumulative: &Probability) -> Self {
        let value = cumulative.value();
        let scaled = value.numer() * BigInt::from(DRAWS);
        let denominator = value.denom();
        let ceiling = (scaled + denominator - BigInt::from(1)) / denominator;
        // A probability is at most one, so the ceiling is at most 2^64.
        Self(ceiling.to_u128().map_or(DRAWS, |t| t.min(DRAWS)))
    }

    /// Whether the draw `draw` falls below the cut.
    #[must_use]
    pub fn admits(self, draw: u64) -> bool {
        u128::from(draw) < self.0
    }
}

/// The cumulative thresholds of one row: entry `j` cuts at
/// `⌈(p_0 + … + p_j) · 2^64⌉`, and the last entry is `2^64`.
fn row_thresholds(row: &[Probability]) -> Vec<Threshold> {
    let mut sum = BigRational::from_integer(BigInt::from(0));
    let mut cuts = Vec::with_capacity(row.len());
    for probability in row {
        sum += probability.value();
        // A row sums to one, so every partial sum is a probability.
        let cut = Probability::try_new(&sum).map_or(Threshold(DRAWS), |p| Threshold::of(&p));
        cuts.push(cut);
    }
    cuts
}

/// The first state whose cumulative threshold admits `draw`.
fn choose(cuts: &[Threshold], draw: u64) -> usize {
    cuts.iter()
        .position(|cut| cut.admits(draw))
        .unwrap_or(cuts.len().saturating_sub(1))
}

/// The final states of independent chain trajectories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSamples {
    /// The state each trajectory ends in, one entry per sample.
    pub final_states: Vec<usize>,
}

impl ChainSamples {
    /// The number of trajectories that end in `state`.
    #[must_use]
    pub fn count(&self, state: usize) -> u64 {
        self.final_states.iter().filter(|&&s| s == state).count() as u64
    }

    /// The number of trajectories.
    #[must_use]
    pub fn samples(&self) -> u64 {
        self.final_states.len() as u64
    }
}

/// The outcomes of independent retry runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrySamples {
    /// For each run, the attempt on which it succeeded, or `None` when it
    /// had not succeeded within the attempt limit.
    pub attempts: Vec<Option<u64>>,
}

impl RetrySamples {
    /// The number of runs that succeeded within the limit.
    #[must_use]
    pub fn successes(&self) -> u64 {
        self.attempts.iter().flatten().count() as u64
    }

    /// The number of runs that succeeded within `k` attempts.
    #[must_use]
    pub fn successes_within(&self, k: u64) -> u64 {
        self.attempts.iter().flatten().filter(|&&a| a <= k).count() as u64
    }

    /// The total number of attempts over the runs that succeeded.
    #[must_use]
    pub fn total_attempts(&self) -> u64 {
        self.attempts.iter().flatten().sum()
    }

    /// The number of runs.
    #[must_use]
    pub fn samples(&self) -> u64 {
        self.attempts.len() as u64
    }
}

/// The exact transition matrix of a checked chain.
///
/// # Errors
///
/// [`SimError::Parametric`] when the chain has parameters.
pub fn chain_matrix(chain: &Chain) -> Result<&TransitionMatrix, SimError> {
    chain.matrix.as_ref().ok_or(SimError::Parametric)
}

/// The exact success probability of a checked retry.
///
/// # Errors
///
/// [`SimError::Parametric`] when the retry has parameters.
pub fn retry_probability(retry: &Retry) -> Result<&Probability, SimError> {
    retry.probability.as_ref().ok_or(SimError::Parametric)
}

/// Runs `samples` independent trajectories of `steps` steps from state
/// `start`, with one draw of a [`SplitMix64`] seeded by `seed` per step.
///
/// # Errors
///
/// [`SimError::StateOutOfRange`] when `start` is not a state of the chain.
pub fn sample_chain(
    matrix: &TransitionMatrix,
    start: usize,
    steps: usize,
    samples: usize,
    seed: u64,
) -> Result<ChainSamples, SimError> {
    if start >= matrix.states() {
        return Err(SimError::StateOutOfRange(start));
    }
    let cuts: Vec<Vec<Threshold>> = matrix
        .rows()
        .iter()
        .map(|row| row_thresholds(row.probabilities()))
        .collect();
    let mut rng = SplitMix64::new(seed);
    let mut final_states = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut state = start;
        for _ in 0..steps {
            let draw = rng.next_u64();
            state = cuts.get(state).map_or(state, |row| choose(row, draw));
        }
        final_states.push(state);
    }
    Ok(ChainSamples { final_states })
}

/// Runs `samples` independent retries, each repeating an attempt that
/// succeeds with probability `success` for at most `max_attempts` attempts,
/// with one draw of a [`SplitMix64`] seeded by `seed` per attempt.
#[must_use]
pub fn sample_retry(
    success: &Probability,
    max_attempts: u64,
    samples: usize,
    seed: u64,
) -> RetrySamples {
    let cut = Threshold::of(success);
    let mut rng = SplitMix64::new(seed);
    let mut attempts = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut outcome = None;
        for attempt in 1..=max_attempts {
            if cut.admits(rng.next_u64()) {
                outcome = Some(attempt);
                break;
            }
        }
        attempts.push(outcome);
    }
    RetrySamples { attempts }
}
