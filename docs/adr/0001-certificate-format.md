# Certificates are versioned JSON documents that embed the model and state claims with exact fractions

Status: accepted

Date: 2026-11-17

## Context and problem

The engine (Rust, untrusted) and the checker (Lean, trusted) communicate through a single file, the *certificate*. The engine writes it; the checker reads it and decides, claim by claim, whether to accept. The format is therefore the only interface across the trust boundary, and every later decision about parsing, semantics and checking refers to it. The format must let the checker verify each claim without trusting any value the engine computed, must represent probabilities exactly, and must allow claim kinds to be added as the project grows.

## Decision drivers

- The checker must be able to rebuild everything it needs from the certificate alone, so that Bob does not need Alice's other files.
- Probabilities must be exact: one tenth must be `1/10`, never the nearest binary floating-point number.
- Each number must have exactly one valid spelling, so that a checker cannot be confused by alternative spellings of the same value, and the engine's output must be canonical, so that two engines that agree on a claim produce identical files.
- Parsing in Lean must be simple, because the parser belongs to the trusted computing base.
- Unknown or malformed input must be rejected, not silently ignored.
- The format must be readable by a person reviewing a rejection.

## Considered options

1. **JSON with fractions as decimal strings.** Human-readable; parsers exist in Rust (`serde_json`) and Lean (`Lean.Json`); strings carry integers of any size without loss.
2. **JSON with fractions as JSON numbers.** Simpler to write, but many JSON implementations parse numbers as 64-bit floating point, which silently rounds large numerators and denominators.
3. **A binary format such as CBOR.** Compact and exact, but harder to inspect when diagnosing a rejection, and the Lean side would need a dedicated decoder in the trusted computing base.
4. **Generated Lean source files.** The engine writes Lean definitions that the checker compiles. This removes the parser from the trusted computing base but makes the engine's output executable code, which is harder to bound and to review.

## Decision

Option 1. A certificate is a JSON object with exactly these fields; any other field, and any field that appears twice in one object, is rejected:

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
- `version` is a JSON integer with no fraction or exponent. Version 1 is defined here. A checker rejects any version it does not implement.
- `model.rows` is the transition matrix: one row per current state, one entry per next state, in state order. There are between 1 and 16 rows, and every row has as many entries as there are rows, so the matrix is square. The model is embedded so that the checker rebuilds the transition matrix itself.
- Every fraction is an object with exactly two string fields. `num` is a non-negative integer in decimal and `den` is a positive integer in decimal, with no sign, no leading zeros (zero is written `0`), no whitespace, and at most 64 digits each. The fraction must be in lowest terms, meaning that the greatest common divisor of `num` and `den` is 1, so zero is `{"num": "0", "den": "1"}` and one half is never `2/4`.
- `claims` is a non-empty list of distinct claims, so a certificate always asserts something and never asserts the same thing twice. Each claim has a `kind` and the witness fields for that kind. A row index is a JSON integer from 0 to 2<sup>64</sup> − 1, with no sign, fraction or exponent, and must name an existing row.

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

The text is UTF-8 with no byte order mark. It must be strict JSON: `NaN`, `Infinity`, comments and trailing commas are rejected. Escape sequences inside strings are decoded before the rules above are applied, so `"\u0031"` is read as `"1"`.

A certificate larger than 1 MiB, measured in UTF-8 bytes, is rejected before parsing, which bounds the memory any reader spends on a single certificate. The number of rows and the length of every row are checked before any fraction is parsed or summed. This order matters: checking that a row sums to one costs more as the entries' denominators grow apart, and an early version without the 16-state limit spent more than 18 minutes on a single 1 MiB row of large, distinct denominators. With at most 16 entries of at most 64 digits per row, a reader checks the worst-case certificate in well under a second.

A reader's error messages report positions, counts and limits, never text copied from the certificate, so that a hostile certificate cannot fill a log or inject terminal control sequences.

Whitespace between tokens and the order of keys within an object carry no meaning: a reader accepts any arrangement, provided every value is canonical. The engine always writes the same arrangement, the one shown above, pretty-printed with two-space indentation and a trailing newline. Requiring canonical input text as well would add a second, byte-level check to the trusted parser without strengthening any claim, because the checker's verdict depends only on the values.

## Consequences

- The Lean checker needs a JSON parser and a decimal-integer parser in its trusted computing base. Both are small and can be tested against the same golden files as the engine.
- Rejecting unknown fields means an old checker cannot read a newer certificate. This is intended: a checker that ignored an unknown claim kind could report a certificate as verified while skipping part of it.
- Canonical output means certificates written by the engine can be compared byte for byte, and equal claims produce equal files. Certificates from other sources are compared by value.
- The 16-state, 64-digit and 1 MiB limits exclude large models and extremely precise probabilities. The models in this project have two states. The limits can be raised in a later version, together with a faster sum check, if a real model needs it.
- Fractions as strings are slightly more verbose than JSON numbers. Certificate size is one of the evaluation metrics, so the cost will be measured.

## Verification

- `crates/axiom-cert` implements version 1. Its tests include a golden file, `fixtures/certificates/links-mass.json`, which the engine must reproduce byte for byte and which the Lean checker will later parse.
- Rejection tests cover each rule above: wrong format, unsupported version, unknown and repeated fields, a byte order mark, numbers with a fraction or exponent, non-canonical and over-long numbers, a zero denominator, rows that do not sum to one, a matrix that is empty, not square or larger than 16 states, an empty or repeated claim, claims that refer to a missing row, and oversized input. Further tests confirm that whitespace and key order do not change the result, that a long row of large denominators is rejected before it is summed, and that no error message contains text from the certificate.

## Related decisions

- ADR-002 (planned) decides how the Lean checker ingests this format.
- ADR-003 (planned) fixes the model language whose transition matrices `model.rows` encodes.
