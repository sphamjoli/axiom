//! The reading rules the Lean checker names when it refuses a certificate.
//!
//! The checker reports a refusal as one `error:` line whose text is fixed by
//! its source: a reading rule such as `fraction is not in canonical form`,
//! or a JSON syntax rule after `invalid JSON at character N: `. Each rule
//! here pairs that text with a stable code, so `axiom check` can say which
//! rule failed without forwarding anything the checker printed. A text that
//! is not in the table is reported as a refusal with no named rule.

use std::fmt;

/// One of the checker's reading rules, listed in [`CheckerRule::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckerRule(usize);

/// Whether a rule is a JSON syntax rule, reported after the character
/// position, or a certificate format rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    /// Printed as `invalid JSON at character N: <text>`.
    Json,
    /// Printed as `<text>`.
    Format,
}

/// The checker's reading rules: the text it prints, the layer it prints it
/// in, and the code `axiom check` reports.
const RULES: [(&str, Layer, &str); 57] = [
    (
        "unexpected character",
        Layer::Json,
        "checker-json-unexpected-character",
    ),
    (
        "unexpected end of input",
        Layer::Json,
        "checker-json-unexpected-end",
    ),
    (
        "unexpected data after the value",
        Layer::Json,
        "checker-json-trailing-data",
    ),
    (
        "unterminated string",
        Layer::Json,
        "checker-json-unterminated-string",
    ),
    ("invalid escape", Layer::Json, "checker-json-invalid-escape"),
    (
        "invalid unicode escape",
        Layer::Json,
        "checker-json-invalid-unicode-escape",
    ),
    (
        "unpaired surrogate",
        Layer::Json,
        "checker-json-unpaired-surrogate",
    ),
    (
        "control character in string",
        Layer::Json,
        "checker-json-control-character",
    ),
    (
        "expected a value",
        Layer::Json,
        "checker-json-expected-value",
    ),
    ("expected a key", Layer::Json, "checker-json-expected-key"),
    (
        "expected : after key",
        Layer::Json,
        "checker-json-expected-colon",
    ),
    (
        "expected , or ] in array",
        Layer::Json,
        "checker-json-bad-array",
    ),
    (
        "expected , or } in object",
        Layer::Json,
        "checker-json-bad-object",
    ),
    ("leading zero", Layer::Json, "checker-json-leading-zero"),
    (
        "integer has too many digits",
        Layer::Json,
        "checker-json-integer-too-long",
    ),
    (
        "only plain integers are allowed",
        Layer::Json,
        "checker-json-not-plain-integer",
    ),
    ("nested too deeply", Layer::Json, "checker-json-too-deep"),
    (
        "object has too many members",
        Layer::Json,
        "checker-json-too-many-members",
    ),
    (
        "array has too many items",
        Layer::Json,
        "checker-json-too-many-items",
    ),
    ("input too complex", Layer::Json, "checker-json-too-complex"),
    (
        "expected an object",
        Layer::Format,
        "checker-expected-object",
    ),
    ("expected an array", Layer::Format, "checker-expected-array"),
    ("missing field", Layer::Format, "checker-missing-field"),
    ("repeated field", Layer::Format, "checker-repeated-field"),
    (
        "unexpected or missing field",
        Layer::Format,
        "checker-unexpected-field",
    ),
    (
        "format must be a string",
        Layer::Format,
        "checker-format-not-string",
    ),
    (
        "certificate format is not axiom-certificate",
        Layer::Format,
        "checker-unsupported-format",
    ),
    (
        "version must be an integer",
        Layer::Format,
        "checker-version-not-integer",
    ),
    (
        "unsupported certificate version",
        Layer::Format,
        "checker-unsupported-version",
    ),
    (
        "num must be a string",
        Layer::Format,
        "checker-num-not-string",
    ),
    (
        "den must be a string",
        Layer::Format,
        "checker-den-not-string",
    ),
    (
        "fraction is not in canonical form",
        Layer::Format,
        "checker-non-canonical-fraction",
    ),
    (
        "entry is above one",
        Layer::Format,
        "checker-entry-above-one",
    ),
    ("model has no rows", Layer::Format, "checker-no-rows"),
    (
        "model has too many states",
        Layer::Format,
        "checker-too-many-states",
    ),
    (
        "matrix is not square",
        Layer::Format,
        "checker-matrix-not-square",
    ),
    (
        "model must have exactly one of rows, retry and parametric",
        Layer::Format,
        "checker-model-form",
    ),
    (
        "retry model needs certificate version 3",
        Layer::Format,
        "checker-retry-needs-version",
    ),
    (
        "parametric model needs certificate version 4",
        Layer::Format,
        "checker-parametric-needs-version",
    ),
    (
        "interval end must be open or closed",
        Layer::Format,
        "checker-interval-end",
    ),
    ("interval is empty", Layer::Format, "checker-empty-interval"),
    (
        "certificate makes no claims",
        Layer::Format,
        "checker-no-claims",
    ),
    (
        "a claim is repeated",
        Layer::Format,
        "checker-duplicate-claim",
    ),
    (
        "kind must be a string",
        Layer::Format,
        "checker-kind-not-string",
    ),
    (
        "unknown claim kind",
        Layer::Format,
        "checker-unknown-claim-kind",
    ),
    (
        "claim kind needs certificate version 2",
        Layer::Format,
        "checker-claim-needs-version-2",
    ),
    (
        "claim kind needs certificate version 3",
        Layer::Format,
        "checker-claim-needs-version-3",
    ),
    (
        "claim kind needs certificate version 4",
        Layer::Format,
        "checker-claim-needs-version-4",
    ),
    (
        "claim kind needs a parametric model",
        Layer::Format,
        "checker-not-parametric",
    ),
    (
        "claim kind needs a chain model",
        Layer::Format,
        "checker-not-chain",
    ),
    (
        "claim kind needs a retry model",
        Layer::Format,
        "checker-not-retry",
    ),
    (
        "claim kind needs a two-state model",
        Layer::Format,
        "checker-not-two-state",
    ),
    (
        "claim refers to a missing row",
        Layer::Format,
        "checker-row-out-of-range",
    ),
    (
        "row must be an integer",
        Layer::Format,
        "checker-row-not-integer",
    ),
    (
        "count must be an integer",
        Layer::Format,
        "checker-count-not-integer",
    ),
    (
        "count is above the limit",
        Layer::Format,
        "checker-count-out-of-range",
    ),
    (
        "tolerance must be positive",
        Layer::Format,
        "checker-tolerance-not-positive",
    ),
];

/// The prefix of a JSON syntax refusal, before the character position.
const JSON_PREFIX: &str = "invalid JSON at character ";

impl CheckerRule {
    /// Every rule, in table order.
    pub const ALL: [Self; RULES.len()] = {
        let mut all = [Self(0); RULES.len()];
        let mut index = 0;
        while index < RULES.len() {
            all[index] = Self(index);
            index += 1;
        }
        all
    };

    /// The rule a checker `error:` line names, if any. For example,
    /// `invalid JSON at character 3: leading zero` names
    /// `checker-json-leading-zero`, and `a claim is repeated` names
    /// `checker-duplicate-claim`.
    #[must_use]
    pub fn from_line(line: &str) -> Option<Self> {
        let (layer, text) = match line.strip_prefix(JSON_PREFIX) {
            Some(rest) => {
                let (position, text) = rest.split_once(": ")?;
                if position.is_empty() || !position.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                (Layer::Json, text)
            }
            None => (Layer::Format, line),
        };
        RULES
            .iter()
            .position(|(rule, rule_layer, _)| *rule == text && *rule_layer == layer)
            .map(Self)
    }

    /// The stable kebab-case code for the rule.
    #[must_use]
    pub const fn code(self) -> &'static str {
        RULES[self.0].2
    }

    /// The rule as the checker states it, without any character position.
    #[must_use]
    pub const fn text(self) -> &'static str {
        RULES[self.0].0
    }
}

impl fmt::Display for CheckerRule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.text())
    }
}
