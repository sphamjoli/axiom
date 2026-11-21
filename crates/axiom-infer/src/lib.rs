//! Black-box analysis: a model treated as a process that can only be run.
//!
//! A [`BlackBox`] reports its number of states and steps from a state to the
//! next; its transition probabilities stay hidden. [`MatrixBox`] is the
//! black box of a checked model, so tests can compare black-box results
//! with the exact answer. [`sample_parameters`] draws parameter points
//! uniformly from a parametric model's box of ranges as exact rationals,
//! and [`coverage`] reports how much of the box they reach.
//!
//! [`propose`] runs a two-state black box from a seed, counts how often each
//! state is left, and proposes a candidate fixed point `â / (â + b̂)` and
//! contraction factor `1 - â - b̂` with Hoeffding bounds, flagging a process
//! whose bounds admit `|λ| = 1` as non-convergent.
//!
//! Every result here is estimated, never proven. Black-box results cannot
//! show what happens in regions of the parameter space no sample reached,
//! nor transitions rarer than the sampling resolution `epsilon`, and an
//! estimate is not a proof. A candidate becomes a proven claim only when a
//! model is supplied, the engine writes the exact claim and the Lean checker
//! accepts it.
//!
//! ```
//! use axiom_infer::{Config, MatrixBox, propose};
//! use num_rational::BigRational;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let source = "chain links {\n  states up, down\n  from up: up 9/10, down 1/10\n  from down: up 3/10, down 7/10\n}\n";
//! let axiom_model::CheckedModel::Chain(chain) = axiom_model::parse(source)? else {
//!     return Ok(());
//! };
//! let matrix = chain.matrix.ok_or("parametric")?;
//! let ratio = |n: i64, d: i64| BigRational::new(n.into(), d.into());
//! let config = Config::new(ratio(1, 20), ratio(1, 1000), 7);
//! let candidates = propose(&MatrixBox::new(&matrix), &config)?;
//! assert!(candidates.contraction_bounds.contains(&ratio(3, 5)));
//! # Ok(())
//! # }
//! ```

mod black_box;
mod candidates;
mod error;
mod parameters;

pub use black_box::{
    BlackBox, MAX_SAMPLES, MAX_TRAJECTORY_STEPS, MatrixBox, hits_after, sample_state,
};
pub use candidates::{Bounds, Candidates, Config, Counts, MAX_STEPS, RESTART_STEPS, propose};
pub use error::InferError;
pub use parameters::{Coverage, MAX_CELLS, MAX_POINTS, POINT_BITS, coverage, sample_parameters};
