# Certificates are versioned JSON documents that embed the model and state claims with exact fractions

Status: accepted

Date: 2026-11-17

## Context and problem

The engine (Rust, untrusted) and the checker (Lean, trusted) communicate through a single file, the *certificate*. The engine writes it; the checker reads it and decides, claim by claim, whether to accept. The format is therefore the only interface across the trust boundary, and every later decision about parsing, semantics and checking refers to it. The format must let the checker verify each claim without trusting any value the engine computed, must represent probabilities exactly, and must allow claim kinds to be added as the project grows.

## Decision drivers

- The checker must be able to rebuild everything it needs from the certificate alone, so that Bob does not need Alice's other files.
- Probabilities must be exact: one tenth must be `1/10`, never the nearest binary floating-point number.
- Each value must have exactly one valid encoding, so that two engines that agree on a claim produce identical certificates and a checker cannot be confused by alternative spellings of the same number.
- Parsing in Lean must be simple, because the parser belongs to the trusted computing base.
- Unknown or malformed input must be rejected, not silently ignored.
- The format must be readable by a person reviewing a rejection.

## Considered options

1. **JSON with fractions as decimal strings.** Human-readable; parsers exist in Rust (`serde_json`) and Lean (`Lean.Json`); strings carry integers of any size without loss.
2. **JSON with fractions as JSON numbers.** Simpler to write, but many JSON implementations parse numbers as 64-bit floating point, which silently rounds large numerators and denominators.
3. **A binary format such as CBOR.** Compact and exact, but harder to inspect when diagnosing a rejection, and the Lean side would need a dedicated decoder in the trusted computing base.
4. **Generated Lean source files.** The engine writes Lean definitions that the checker compiles. This removes the parser from the trusted computing base but makes the engine's output executable code, which is harder to bound and to review.

## Decision

Option 1. A certificate is a JSON object with exactly these fields, and any other field is rejected:

```json
{
  "format": "axiom-certificate",
  "version": 1,
  "model": {
    "rows": [
      [{ "num": "9", "den": "10" }, { "num": "1", "den": "10" }],
      [{ "num": "3", "den": "10" }, { "num": "7", "den": "10" }]
    ]
  },
  "claims": [
    { "kind": "Mass", "row": 0 },
    { "kind": "Mass", "row": 1 }
  ]
}
```

- `format` is the constant string `axiom-certificate`, so that an unrelated JSON file is rejected immediately.
- `version` is an integer. Version 1 is defined here. A checker rejects any version it does not implement.
- `model.rows` is the transition matrix: one row per current state, one entry per next state, in state order. The model is embedded so that the checker rebuilds the transition matrix itself.
- Every fraction is an object with exactly two string fields. `num` is a non-negative integer in decimal and `den` is a positive integer in decimal, with no sign, no leading zeros (zero is written `0`), no whitespace, and at most 256 digits each. The fraction must be in lowest terms, so zero is `{"num": "0", "den": "1"}` and one half is never `2/4`.
- `claims` is a list. Each claim has a `kind` and the witness fields for that kind.

The certainty of a claim follows from its kind: an `Estimate` claim is estimated and every other kind is proven, so no separate certainty field is needed.

Version 1 defines one claim kind:

| Kind | Fields | Meaning |
| --- | --- | --- |
| `Mass` | `row`: index into `model.rows` | Every entry of that row is a probability and the entries sum to exactly one. |

Later versions will add the remaining claim kinds with these witness fields, each as a version increment:

| Kind | Planned fields |
| --- | --- |
| `FixedPoint` | `value`: the steady state, a fraction |
| `Contraction` | `factor`: the contraction factor, a fraction |
| `ConvergenceBound` | `start`, `steps`, `tolerance` |
| `Termination` | `probability`: the least solution, a fraction |
| `Estimate` | `value`, `samples`, `epsilon`, `delta` |

A certificate whose JSON text exceeds 1 MiB is rejected before parsing, which bounds the memory any reader spends on a single certificate.

## Consequences

- The Lean checker needs a JSON parser and a decimal-integer parser in its trusted computing base. Both are small and can be tested against the same golden files as the engine.
- Rejecting unknown fields means an old checker cannot read a newer certificate. This is intended: a checker that ignored an unknown claim kind could report a certificate as verified while skipping part of it.
- Canonical encoding means a certificate can be compared byte for byte, and equal claims produce equal files.
- The 256-digit and 1 MiB limits exclude models with extremely precise probabilities. They can be raised in a later version if a real model needs it.
- Fractions as strings are slightly more verbose than JSON numbers. Certificate size is one of the evaluation metrics, so the cost will be measured.

## Verification

- `crates/axiom-cert` implements version 1. Its tests include a golden file, `crates/axiom-cert/tests/fixtures/links-mass.json`, which the engine must reproduce byte for byte and which the Lean checker will later parse.
- Rejection tests cover each rule above: wrong format, unsupported version, unknown fields, non-canonical and over-long numbers, a zero denominator, rows that do not sum to one, claims that refer to a missing row, and oversized input.

## Related decisions

- ADR-002 (planned) decides how the Lean checker ingests this format.
- ADR-003 (planned) fixes the model language whose transition matrices `model.rows` encodes.
