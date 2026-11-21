//! The benchmark harness behind `axiom-bench`.
//!
//! For every fixture model and a generated scaling family it measures the
//! engine's time, the certificate's size, the Lean checker's time and the
//! time to simulate each claim's conclusion to the same confidence, then
//! writes the measurements as CSV, a Markdown table and SVG plots.

pub mod cases;
pub mod coverage;
pub mod engine;
pub mod error;
pub mod report;
pub mod run;
pub mod simulate;
pub mod svg;
