//! Writing and reading axiom certificates.
//!
//! A certificate is the file the engine hands to the checker: it embeds the
//! model, a transition matrix, a retry loop or a parametric two-state chain,
//! and lists claims about it. Version 1 holds `Mass` claims only; version 2
//! adds `FixedPoint` and `Contraction` claims about two-state models;
//! version 3 adds `ConvergenceBound` claims about two-state models, the
//! retry model and `Termination` claims about it; version 4 adds the
//! parametric model, the `ParametricContraction` claim about it and the
//! `Estimate` claim, the only claim whose certainty is estimated rather
//! than proven. A
//! certificate is written in the lowest version that can express it, so
//! certificates of earlier versions are written byte for byte as before.
//! The format is canonical JSON with every fraction
//! written as decimal strings in lowest terms; only a `Contraction` factor
//! may be negative, written with a leading `-`. Reading a
//! certificate validates every rule of the format and fails closed: any
//! deviation is an error, never a silent default.

mod certificate;
mod document;
mod error;
mod fraction_text;

pub use certificate::{
    Certificate, Claim, End, Estimate, FAILURE_ATTEMPTS, FailureAfter, Interval,
    MAX_CERTIFICATE_BYTES, MAX_COUNT, MAX_JSON_DEPTH, MAX_JSON_ITEMS, MAX_JSON_MEMBERS,
    MAX_SAMPLES, MAX_STATES, Model, Quantity,
};
pub use error::{CertificateError, ConvergenceBoundError};
