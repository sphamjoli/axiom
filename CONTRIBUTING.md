# Contributing to axiom

This guide covers everything needed to build, check and change the repository: the tools to install, the commands to run, the layout of the code, and the rules for commits and pull requests. The project itself is described in [README.md](README.md). It is a proof of concept, and at present the repository contains workspace configuration only.

## Prerequisites

Three tools must be installed before any command below will work.

| Tool | Purpose |
| --- | --- |
| rustup | Installs and selects Rust toolchains. The version this project uses is pinned in `rust-toolchain.toml`, and rustup installs it automatically on first use. |
| elan | The Lean version manager, the Lean counterpart of rustup. The Lean version is pinned in `lean/lean-toolchain`. |
| bun | A JavaScript runtime. It runs the scripts that the Git hooks use to check formatting and commit messages; the project itself contains no JavaScript. |

`actionlint`, a linter for GitHub Actions workflows, is optional. When it is installed, the hooks also check workflow files.

## First-time setup

Install the Git hooks and the dependencies of the hook scripts:

```bash
make hooks-install
```

Download prebuilt mathlib files (mathlib is the Lean library of formalised mathematics that the checker uses). Without this step the first Lean build compiles mathlib from source, which takes a long time:

```bash
make lean-cache
```

## Running the checks

Run everything that continuous integration (CI), the automated checks GitHub runs on every pull request, runs:

```bash
make check
```

The individual targets are listed below; `make help` prints the same list.

| Target | What it does |
| --- | --- |
| `make fmt` | Formats the Rust sources in place. |
| `make rust-fmt` | Fails if any Rust source is not formatted. |
| `make rust-lint` | Runs clippy, the Rust linter, and treats every warning as an error. |
| `make rust-test` | Runs the Rust tests. |
| `make rust-deny` | Checks dependencies for security advisories, disallowed licences, banned crates and unknown sources (needs `cargo-deny`, installed with `cargo install cargo-deny`). |
| `make rust-doc` | Builds the Rust documentation and treats every warning as an error. |
| `make lean-check` | A fast text search for Lean constructs that skip or bypass proof checking: `sorry`, `admit`, `native_decide`, new `axiom` declarations and similar. |
| `make lean-build` | Builds the Lean checker, treating warnings as errors. |
| `make lean-audit` | Asks the Lean kernel which axioms each checker declaration depends on, and fails unless the only ones are the three standard axioms of Lean's logic (`propext`, `Classical.choice` and `Quot.sound`). This catches bypasses that the text search misses. |

On some recent macOS versions the mathlib cache tool fails to start. `make lean-cache`, and therefore `make lean-build` and `make lean-audit`, then fail locally even though `cd lean && lake build` succeeds. CI runs on Linux and is unaffected.

## Repository layout

| Path | Contents |
| --- | --- |
| `Cargo.toml`, `crates/` | The Rust engine, a Cargo workspace. `crates/axiom-core` holds exact probabilities, transition rows, two-state chains and affine updates; each crate keeps its tests in its own `tests/` folder. |
| `lean/` | The Lean 4 project for the `AxiomCheck` checker, pinned to Lean v4.19.0 and mathlib v4.19.0. `lean/scripts/AxiomAudit.lean` implements the axiom audit. |
| `docs/` | Diagrams drawn in Excalidraw: each has an editable `.excalidraw` source and a rendered `.svg`. Diagrams are always drawn in Excalidraw, never Mermaid. |
| `scripts/` | The checks run by the Git hooks and by CI. |
| `.github/` | CI workflows, the pull request template, issue templates and label definitions. |

## Git hooks

The hooks are managed by lefthook and defined in `lefthook.yml`.

| Hook | Checks |
| --- | --- |
| pre-commit | Rust formatting; forbidden Lean constructs; decorative separator comments and comments that trail code on the same line; Markdown paragraphs split across several lines; validity of YAML, TOML and shell files; workflow lint when actionlint is installed; a guard that refuses to commit local planning files. |
| commit-msg | The commit message format described below. |
| pre-push | Clippy, the Rust tests, the forbidden-construct search and the axiom audit. |

A hook whose script cannot find a JavaScript runtime skips instead of failing, so that commits from graphical Git clients with a restricted environment are not blocked. CI runs the same checks and does not skip them.

## Commits

Each commit message is a single line of the form `type(scope): subject`, with no body. The type is one of `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`, `perf` and `revert`. The scope is optional and is one of `core`, `infer`, `circuit`, `cert`, `cli`, `lean`, `examples`, `docs`, `ci` and `deps`. The subject starts with a letter and states what changed, for example `fix(cert): reject certificates whose rows do not sum to one`. Commits carry no co-author trailers.

Local planning notes belong in `.local/`, which Git ignores. They are never committed.

## Pull requests

A pull request title follows the same `type(scope): subject` format as a commit message, and CI checks it with the same script. Fill in the pull request template. In particular:

- `make check` passes locally, or the description states which target could not be run and why.
- Every new claim kind that the engine emits has a matching check in the Lean checker and an adversarial certificate, a deliberately wrong certificate that the checker must reject.
- No Lean source contains `sorry`, `admit`, a new `axiom` or `native_decide`.
- Any change to the trusted computing base, the set of components whose errors could cause a false claim to be accepted, is stated in the description and the pull request is labelled `tcb`.
- No secrets, local planning notes or build output are committed.

CI also scans each pull request for secrets, applies labels according to the files changed, and posts a summary comment with the results.

## Writing

Documentation follows the conventions of the README: one line per paragraph, every technical term explained where it is first used, mathematics written in LaTeX, no citations, and claims about unimplemented parts stated as plans.
