//! Reading `.axm` model files.
//!
//! A model file holds one model: a *chain*, a finite-state Markov chain
//! where each state has a probability for each next state, or a *retry*, an
//! attempt that succeeds with some probability and otherwise repeats.
//! Weights are exact rationals, optionally affine in declared parameters:
//!
//! ```text
//! param a in ]0, 1[
//! chain links {
//!   states up, down
//!   from up:   up 1 - a, down a
//!   from down: up 3/10, down 7/10
//! }
//! ```
//!
//! [`parse`] reads such a file, runs every static check and returns a
//! [`CheckedModel`]. A rejection is a [`ModelError`] carrying the line and
//! column of the fault, and its message never quotes the file.
//!
//! ```
//! use axiom_model::{CheckedModel, parse};
//!
//! let model = parse("retry request {\n  success 3/10\n}\n");
//! assert!(matches!(model, Ok(CheckedModel::Retry(_))));
//!
//! let rejected = parse("retry request { success 3/2 }");
//! assert_eq!(rejected.err().map(|error| error.code()), Some("weight-above-one"));
//! ```

pub mod ast;
mod check;
mod error;
mod lexer;
mod model;
mod parser;

pub use check::check;
pub use error::{Expected, ModelError, Position};
pub use model::{AffineWeight, Bound, Chain, CheckedModel, Parameter, Retry};
pub use parser::parse_syntax;

/// The largest accepted file, in bytes: 64 KiB.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;

/// The most digits a number may have, the same limit as certificates use.
pub const MAX_DIGITS: usize = 64;

/// The most states a chain may have, the same limit as certificates use.
pub const MAX_STATES: usize = 16;

/// The most parameters a file may declare.
pub const MAX_PARAMETERS: usize = 8;

/// Parses `source` and runs every static check.
///
/// # Errors
///
/// Returns the first fault, lexical, grammatical or static, with its
/// position.
pub fn parse(source: &str) -> Result<CheckedModel, ModelError> {
    check(&parse_syntax(source)?)
}
