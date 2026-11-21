//! Exact building blocks for finite-state probabilistic models.
//!
//! Every quantity is an exact rational number, so a value such as one tenth
//! is represented as `1/10` and never rounded. The crate provides validated
//! probabilities, transition rows whose entries sum to exactly one, two-state
//! chains, and the affine update that describes how the probability of
//! occupying a state evolves from one step to the next. It also holds the
//! exact Hoeffding sample-count decision that the simulator and the
//! certificate reader share.

mod affine_update;
mod hoeffding;
mod probability;
mod transition_matrix;
mod transition_row;
mod two_state_chain;

pub use affine_update::AffineUpdate;
pub use hoeffding::{
    EXP_ARGUMENT_CAP, EXP_PRECISION_BITS, EXP_SERIES_TERMS, exp_lower_bound, hoeffding_holds,
};
pub use probability::{Probability, ProbabilityError};
pub use transition_matrix::{TransitionMatrix, TransitionMatrixError};
pub use transition_row::{TransitionRow, TransitionRowError};
pub use two_state_chain::{TwoStateChain, TwoStateChainError};
