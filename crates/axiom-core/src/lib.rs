//! Exact building blocks for finite-state probabilistic models.
//!
//! Every quantity is an exact rational number, so a value such as one tenth
//! is represented as `1/10` and never rounded. The crate provides validated
//! probabilities, transition rows whose entries sum to exactly one, two-state
//! chains, and the affine update that describes how the probability of
//! occupying a state evolves from one step to the next.

mod affine_update;
mod probability;
mod transition_row;
mod two_state_chain;

pub use affine_update::AffineUpdate;
pub use probability::{Probability, ProbabilityError};
pub use transition_row::{TransitionRow, TransitionRowError};
pub use two_state_chain::TwoStateChain;
