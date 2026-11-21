//! Writing and reading axiom certificates.
//!
//! A certificate is the file the engine hands to the checker: it embeds the
//! model, a transition matrix or a retry loop, and lists claims about it.
//! Version 1 holds `Mass` claims only; version 2 adds `FixedPoint` and
//! `Contraction` claims about two-state models; version 3 adds
//! `ConvergenceBound` claims about two-state models, the retry model and
//! `Termination` claims about it. The format is canonical JSON with every fraction
//! written as decimal strings in lowest terms; only a `Contraction` factor
//! may be negative, written with a leading `-`. Reading a
//! certificate validates every rule of the format and fails closed: any
//! deviation is an error, never a silent default.

mod certificate;
mod document;
mod error;
mod fraction_text;

pub use certificate::{
    Certificate, Claim, FailureAfter, MAX_CERTIFICATE_BYTES, MAX_COUNT, MAX_JSON_DEPTH,
    MAX_JSON_ITEMS, MAX_JSON_MEMBERS, MAX_STATES, Model,
};
pub use error::{CertificateError, ConvergenceBoundError};
