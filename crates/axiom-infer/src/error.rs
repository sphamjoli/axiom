use axiom_sim::SimError;
use thiserror::Error;

/// The reason a black-box analysis could not be made. Messages state counts
/// and limits only, never a parameter value or a state the black box
/// returned.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum InferError {
    /// Candidates are proposed for two-state black boxes only.
    #[error("the black box has {states} states, not two")]
    NotTwoState {
        /// The number of states the black box reports.
        states: usize,
    },
    /// A parameter point has a different number of values than the model
    /// has parameters.
    #[error("expected {expected} parameter values, got {actual}")]
    ParameterCount {
        /// The model's parameter count.
        expected: usize,
        /// The values given.
        actual: usize,
    },
    /// A parameter value lies outside its range, or the values do not give
    /// a transition matrix.
    #[error("the parameter values do not give a valid chain of the model")]
    InvalidInstance,
    /// More parameter points were asked for than the limit.
    #[error("more parameter points than the limit of {limit}")]
    TooManyPoints {
        /// The largest accepted count.
        limit: usize,
    },
    /// The coverage grid has no cells or more cells than the limit.
    #[error("the coverage grid must have between 1 and {limit} cells")]
    InvalidGrid {
        /// The largest accepted number of cells.
        limit: u64,
    },
    /// The confidence needs more samples, or a trajectory more steps, than
    /// the limit.
    #[error("the request needs more than the limit of {limit} samples or steps")]
    TooManySamples {
        /// The largest accepted count.
        limit: u64,
    },
    /// The step budget ran out before both states were visited often
    /// enough.
    #[error("both states were not visited often enough within {limit} steps")]
    StepBudgetExceeded {
        /// The step budget.
        limit: u64,
    },
    /// The black box stepped to a state it does not have.
    #[error("the black box stepped to a state outside the chain")]
    StateOutOfRange,
    /// The sample count or estimate could not be made.
    #[error(transparent)]
    Sim(#[from] SimError),
}

impl InferError {
    /// A stable kebab-case name for the error, the same on every machine. A
    /// simulation error keeps its own crate's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotTwoState { .. } => "black-box-not-two-state",
            Self::ParameterCount { .. } => "parameter-count",
            Self::InvalidInstance => "invalid-instance",
            Self::TooManyPoints { .. } => "too-many-points",
            Self::InvalidGrid { .. } => "invalid-grid",
            Self::TooManySamples { .. } => "too-many-samples",
            Self::StepBudgetExceeded { .. } => "step-budget-exceeded",
            Self::StateOutOfRange => "black-box-state-out-of-range",
            Self::Sim(error) => error.code(),
        }
    }
}
