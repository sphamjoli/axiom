use serde::{Deserialize, Deserializer, Serialize};

use crate::fraction_text::FractionText;

/// The constant value of the `format` field.
pub(crate) const FORMAT: &str = "axiom-certificate";

/// The format version of a certificate whose claims are all `Mass`.
pub(crate) const VERSION_1: u32 = 1;

/// The format version that adds the `FixedPoint` and `Contraction` claims.
pub(crate) const VERSION_2: u32 = 2;

/// The format version that adds the `ConvergenceBound` and `Termination`
/// claims and the retry model.
pub(crate) const VERSION_3: u32 = 3;

/// The format version that adds the parametric model, the
/// `ParametricContraction` claim and the `Estimate` claim.
pub(crate) const VERSION_4: u32 = 4;

/// The JSON shape of a version 1, 2, 3 or 4 certificate, before validation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CertificateDocument {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) model: ModelDocument,
    pub(crate) claims: Vec<ClaimDocument>,
}

/// The embedded model: the transition matrix, one row per current state, a
/// retry loop, or a parametric two-state chain. Exactly one field must be
/// present; that rule is checked after parsing.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelDocument {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) rows: Option<Vec<Vec<FractionText>>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) retry: Option<RetryDocument>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) parametric: Option<ParametricDocument>,
}

/// A two-state chain `[[1 - a, a], [b, 1 - b]]` for every `a` and `b` in
/// their intervals.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParametricDocument {
    pub(crate) a: IntervalDocument,
    pub(crate) b: IntervalDocument,
}

/// A rational interval: its two ends, each `open` or `closed`. The ends are
/// strings because the checker's JSON reader has no booleans.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntervalDocument {
    pub(crate) lo: FractionText,
    pub(crate) hi: FractionText,
    pub(crate) lo_end: EndDocument,
    pub(crate) hi_end: EndDocument,
}

/// Whether an interval's end belongs to it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EndDocument {
    Open,
    Closed,
}

/// A retry loop whose attempts each succeed with probability `success`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetryDocument {
    pub(crate) success: FractionText,
}

/// The probability that each of the first `attempts` attempts fails.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureAfterDocument {
    pub(crate) attempts: u64,
    pub(crate) probability: FractionText,
}

/// One claim, tagged by its `kind` field.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(crate) enum ClaimDocument {
    Mass {
        row: u64,
    },
    FixedPoint {
        value: FractionText,
    },
    Contraction {
        factor: FractionText,
    },
    ConvergenceBound {
        start: FractionText,
        steps: u64,
        tolerance: FractionText,
    },
    Termination {
        probability: FractionText,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        expected_attempts: Option<FractionText>,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        failure_after: Option<FailureAfterDocument>,
    },
    ParametricContraction,
}

/// Reads an optional field that, when present, must hold a value: `null` is
/// rejected, as the format has no `null`. A missing field is `None` through
/// `#[serde(default)]`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
