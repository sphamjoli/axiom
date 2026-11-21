//! Writing and reading axiom certificates.
//!
//! A certificate is the file the engine hands to the checker: it embeds the
//! model's transition matrix and lists claims about it. The version 1 format
//! is canonical JSON with
//! every fraction written as decimal strings in lowest terms. Reading a
//! certificate validates every rule of the format and fails closed: any
//! deviation is an error, never a silent default.

mod certificate;
mod document;
mod error;
mod fraction_text;

pub use certificate::{Certificate, Claim, MAX_CERTIFICATE_BYTES, MAX_STATES};
pub use error::CertificateError;
