//! The commands behind the `axiom` binary: argument parsing, `analyse`,
//! which writes certificates, and `check`, which runs the Lean checker and
//! interprets its output.

pub mod analyse;
pub mod check;
pub mod cli;
pub mod error;
pub mod outcome;
