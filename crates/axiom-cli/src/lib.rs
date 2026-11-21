//! The commands behind the `axiom` binary: argument parsing, `analyse`,
//! which writes certificates, and `check`, which runs the Lean checker and
//! interprets its output, and `explore`, which proposes candidate claims
//! from a black box and promotes them only through the checker.

pub mod analyse;
pub mod check;
pub mod checker_rule;
pub mod cli;
pub mod error;
pub mod explore;
pub mod outcome;
