use axiom_core::{Probability, TransitionMatrix, TransitionRow};
use axiom_model::{Chain, Parameter};
use axiom_sim::{SplitMix64, Threshold, choose, row_thresholds};
use num_rational::BigRational;

use crate::error::InferError;

/// The most samples, and the most steps per trajectory times samples, any
/// one request may draw: the certificate format's limit on an `Estimate`.
pub const MAX_SAMPLES: u64 = 10_000_000;

/// The most steps one trajectory of [`hits_after`] may take.
pub const MAX_TRAJECTORY_STEPS: usize = 1_000;

/// A process that can only be run: it reports how many states it has and
/// moves from a state to the next with randomness drawn from `rng`. Its
/// transition probabilities are never exposed.
pub trait BlackBox {
    /// The number of states.
    fn states(&self) -> usize;

    /// The state after `state`. An implementation may return any value;
    /// callers refuse one that is not a state.
    fn step(&self, state: usize, rng: &mut SplitMix64) -> usize;
}

/// The black box of a checked chain: it keeps only the exact cumulative
/// cut points of each row, and steps with one draw per step exactly as
/// [`axiom_sim::sample_chain`] does, so the same seed gives the same
/// trajectories. It lets tests compare black-box results with ground truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixBox {
    cuts: Vec<Vec<Threshold>>,
}

impl MatrixBox {
    /// The black box of `matrix`.
    #[must_use]
    pub fn new(matrix: &TransitionMatrix) -> Self {
        Self {
            cuts: matrix
                .rows()
                .iter()
                .map(|row| row_thresholds(row.probabilities()))
                .collect(),
        }
    }

    /// The black box of a parametric chain at the parameter point `values`,
    /// one value per parameter in declaration order, each in its range.
    ///
    /// # Errors
    ///
    /// [`InferError::ParameterCount`] for the wrong number of values and
    /// [`InferError::InvalidInstance`] for a value outside its range or
    /// weights that are not a transition matrix.
    pub fn instance(chain: &Chain, values: &[BigRational]) -> Result<Self, InferError> {
        if values.len() != chain.parameters.len() {
            return Err(InferError::ParameterCount {
                expected: chain.parameters.len(),
                actual: values.len(),
            });
        }
        if !chain
            .parameters
            .iter()
            .zip(values)
            .all(|(parameter, value)| in_range(parameter, value))
        {
            return Err(InferError::InvalidInstance);
        }
        let rows = chain
            .rows
            .iter()
            .map(|row| {
                let entries = row
                    .iter()
                    .map(|weight| Probability::try_new(&weight.evaluate(values)))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| InferError::InvalidInstance)?;
                TransitionRow::try_new(entries).map_err(|_| InferError::InvalidInstance)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let matrix = TransitionMatrix::try_new(rows).map_err(|_| InferError::InvalidInstance)?;
        Ok(Self::new(&matrix))
    }
}

impl BlackBox for MatrixBox {
    fn states(&self) -> usize {
        self.cuts.len()
    }

    fn step(&self, state: usize, rng: &mut SplitMix64) -> usize {
        let draw = rng.next_u64();
        self.cuts.get(state).map_or(state, |row| choose(row, draw))
    }
}

/// Whether `value` lies in the range of `parameter`.
pub(crate) fn in_range(parameter: &Parameter, value: &BigRational) -> bool {
    let above = if parameter.lower.closed {
        *value >= parameter.lower.value
    } else {
        *value > parameter.lower.value
    };
    let below = if parameter.upper.closed {
        *value <= parameter.upper.value
    } else {
        *value < parameter.upper.value
    };
    above && below
}

/// A state drawn uniformly from `0..states` by rejection, so every state
/// has exactly the same probability. `states` must be positive.
pub fn sample_state(states: usize, rng: &mut SplitMix64) -> usize {
    let count = u64::try_from(states).unwrap_or(u64::MAX).max(1);
    // The largest multiple of `count` that fits, so `draw % count` is exact.
    let zone = u64::MAX - u64::MAX % count;
    loop {
        let draw = rng.next_u64();
        if draw < zone {
            return usize::try_from(draw % count).unwrap_or(0);
        }
    }
}

/// The number of `samples` trajectories of `steps` steps from `start`, run
/// on `black_box` with a [`SplitMix64`] seeded by `seed`, that end in
/// `target`.
///
/// # Errors
///
/// [`InferError::Sim`] with [`axiom_sim::SimError::StateOutOfRange`] when
/// `start` or `target` is not a state, [`InferError::TooManySamples`] when
/// `samples` is above [`MAX_SAMPLES`] or `steps` above
/// [`MAX_TRAJECTORY_STEPS`], and [`InferError::StateOutOfRange`] when the
/// black box steps outside its states.
pub fn hits_after(
    black_box: &impl BlackBox,
    start: usize,
    steps: usize,
    samples: u64,
    target: usize,
    seed: u64,
) -> Result<u64, InferError> {
    let states = black_box.states();
    for state in [start, target] {
        if state >= states {
            return Err(axiom_sim::SimError::StateOutOfRange(state).into());
        }
    }
    if samples > MAX_SAMPLES || steps > MAX_TRAJECTORY_STEPS {
        return Err(InferError::TooManySamples { limit: MAX_SAMPLES });
    }
    let mut rng = SplitMix64::new(seed);
    let mut hits = 0;
    for _ in 0..samples {
        let mut state = start;
        for _ in 0..steps {
            state = black_box.step(state, &mut rng);
            if state >= states {
                return Err(InferError::StateOutOfRange);
            }
        }
        if state == target {
            hits += 1;
        }
    }
    Ok(hits)
}
