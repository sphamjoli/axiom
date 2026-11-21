# The Lean checker reads certificates with its own strict JSON reader and checks them by evaluation

Status: accepted

Date: 2026-11-21

## Context and problem

ADR-001 fixes the certificate format. The checker, written in Lean, must read certificates in that format and decide each claim. Whatever reads the file belongs to the trusted computing base: if it accepts a certificate the format forbids, or reads a value differently from the engine, a claim could be accepted on the strength of a misread number. The question is how the Lean side turns certificate text into values it can reason about, and how it then runs the checks.

Lean's standard library includes a JSON parser, `Lean.Json.parse`. Before relying on it, its behaviour was compared with the format's rules. It differs on inputs the format forbids:

| Input | `Lean.Json.parse` | Certificate format |
| --- | --- | --- |
| `{"a":1,"a":2}` | accepted, keeps only the last value | repeated keys are rejected |
| `1e0`, `1.0`, `10e-1` | accepted, normalised to `1` | integers must be plain digits |
| `-0` | accepted, normalised to `0` | no signs |

A checker built on it would therefore accept certificates that the engine's reader rejects, and the two readers would disagree on what a certificate says.

## Decision drivers

- The engine's reader and the checker's reader must accept the same certificates and read the same values from them. The one exception is arithmetic: whether a row sums to one is the content of a Mass claim, which the checker decides after reading.
- The code in the trusted computing base must be small enough to review line by line.
- The reader must terminate on every input and bound its work, because certificates come from an untrusted engine.
- Agreement between the two readers must be tested, not assumed.

## Considered options

1. **`Lean.Json.parse`, followed by validation of the parsed tree.** Least code, but repeated keys and number spellings are lost during parsing, so no later validation can recover them.
2. **`Lean.Json.parse`, with a separate scan of the raw text** for repeated keys and non-integer numbers. Two readers of the same text, which must agree with each other on string boundaries and escapes, in the trusted computing base.
3. **A strict JSON reader for the subset of JSON that certificates use.** More code than option 1, but one reader, which keeps every key and rejects every number spelling except plain digits.
4. **The engine writes Lean source files instead of JSON.** Removes the reader, but turns the engine's output into code that the checker compiles, which is harder to bound and review.

## Decision

Option 3. `AxiomCheck.Json` is a strict reader for objects, arrays, strings and non-negative integers written as plain decimal digits. Objects keep every key, in order, including repeats. `true`, `false`, `null`, signs, fractions, exponents, leading zeros, integers longer than 20 digits, a byte order mark, trailing data, unescaped control characters and lone surrogates are rejected. Nesting is limited to eight levels, objects to eight members and arrays to 64 items, and every recursive call either consumes input or descends a level, so the reader terminates on every input. The member and item limits are well above what a certificate needs (four keys, 16 items) and keep the interpreter's recursion shallow: an early version without them overflowed the stack on a 40 KB object with 5,000 members.

`AxiomCheck.Parse.certificate` turns the reader's output into a `Certificate`, enforcing the rules of ADR-001: size, then JSON syntax and fields, `format`, `version`, matrix shape, fractions, and claims. The two readers accept the same certificates but may report a different first error for an invalid one, because the engine's JSON library checks field types while parsing. Whether each row sums to one is the content of a Mass claim, so it is left to the claim check and is not a reading rule.

Both readers are tested against the same files in `fixtures/certificates/`: the golden certificate must be read as the link model, every file in `accept/` must be accepted, and every file in `reject/` must be rejected, except the one whose fault is arithmetic, which the reader accepts and the claim check must reject. The acceptance fixtures pin behaviour the two readers must share, such as escaped keys and digits, and whitespace and key order.

The checks themselves will be written as a Lean function `check : Certificate → Bool` with a soundness theorem stating that a certificate for which `check` returns `true` satisfies its claims under the formal semantics. The checker runs `check` by evaluation, so the Lean compiler and interpreter join the trusted computing base. `native_decide` is not allowed: it would make every proof that uses it depend on the compiler as well, without the evidence a separate audit can inspect. Generated Lean files that state `check c = true` for a specific certificate, proved by the kernel, remain available as a way to check a single certificate without trusting the compiler.

## Consequences

- The trusted computing base gains about 400 lines of Lean, documentation included, for the reader and the format rules. In exchange, the two readers agree by construction on repeated keys and number spellings, and by test on everything else.
- The reader is not a general JSON parser: it rejects JSON that certificates never contain, such as booleans and decimals. If a later version of the format needs them, the reader must be extended in the same change.
- Every new reading rule needs a rejection fixture, so that both readers are tested against it.
- Running the checker by evaluation is fast enough to compare checking cost with simulation cost, which is the project's central measurement. The kernel-checked route is slower and is kept for spot checks.
- Lean v4.19 executables do not load on some recent macOS versions. The checker's tests therefore run through `lean --run`, which works on macOS and Linux alike.

## Verification

- `lean/AxiomCheckTest/Main.lean`, run by `make lean-test` and in CI, tests the reader against every input in the table above and against the shared fixtures, size and shape limits, member and item limits, escapes, surrogates and nesting depth.
- Removing the repeated-key check, the square-matrix check, the lowest-terms check or the member limit each makes the tests fail.
- `make lean-audit` confirms that every declaration in the checker depends only on Lean's three standard axioms.

## Related decisions

- ADR-001 defines the certificate format this reader implements.
- ADR-003 (planned) fixes the model language, and so the semantics that the soundness theorem refers to.
