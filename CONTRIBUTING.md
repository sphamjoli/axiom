# Contributing to axiom

This guide covers everything needed to build, check and change the repository: the tools to install, the commands to run, the layout of the code, and the rules for commits and pull requests. The project itself is described in [README.md](README.md). It is a proof of concept.

## Prerequisites

Three tools must be installed before any command below will work.

| Tool | Purpose |
| --- | --- |
| rustup | Installs and selects Rust toolchains. The version this project uses is pinned in `rust-toolchain.toml`, and rustup installs it automatically on first use. |
| elan | The Lean version manager, the Lean counterpart of rustup. The Lean version is pinned in `lean/lean-toolchain`. |
| bun | A JavaScript runtime and package manager. It runs the scripts that the Git hooks use to check formatting and commit messages, and it installs, checks and builds the web interface in `ui/`. |

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
| `make lean-test` | Runs the checker tests in `lean/AxiomCheckTest/` against the shared certificate fixtures. |
| `make cli-demo` | Writes the certificate for `fixtures/models/links.axm` with `axiom analyse` and checks it with `axiom check`. |
| `make ui-check` | Installs the web interface dependencies from `ui/bun.lock`, typechecks the interface with `vue-tsc` and builds it with Vite. |
| `make lean-audit` | Asks the Lean kernel which axioms each checker declaration depends on, and fails unless the only ones are the three standard axioms of Lean's logic (`propext`, `Classical.choice` and `Quot.sound`). This catches bypasses that the text search misses. |
| `make engine-subset` | A fast check, with no extra tools, that the engine functions covered by proof 3 (listed in `proof3.toml`, defined in `crates/axiom-core/src/kernel.rs`) match the list and use only constructs the translator supports. |
| `make engine-install` | Downloads the Charon and Aeneas release pinned in `proof3.toml` into `.aeneas/`, refusing it unless its SHA-256 matches the one in `scripts/installAeneas.sh`, and installs the Rust nightly that Charon needs. Only needed to work on proof 3. |
| `make engine-translate` | Translates the covered functions to Lean with the pinned Charon and Aeneas and fails if the result differs from the committed `lean-engine/AxiomEngine/Generated/`. Run `scripts/generateEngine.sh` to regenerate it after changing a covered function. |
| `make engine-check` | Builds the proof 3 Lean project in `lean-engine/`, treating warnings as errors, and fails unless every declaration in it uses only the three standard axioms. |

On some recent macOS versions the mathlib cache tool fails to start. `make lean-cache`, and therefore `make lean-build` and `make lean-audit`, then fail locally even though `cd lean && lake build` succeeds. CI runs on Linux and is unaffected.

## Repository layout

| Path | Contents |
| --- | --- |
| `Cargo.toml`, `crates/` | The Rust engine, a Cargo workspace. `crates/axiom-core` holds exact probabilities, transition rows, two-state chains, affine updates and the exact Hoeffding sample-count decision. `crates/axiom-cert` writes and reads certificates in format versions 1 to 4, always writing the lowest version that can express a certificate. `crates/axiom-model` reads `.axm` model files: it parses them and runs the static checks of the modelling language. `crates/axiom-sim` samples chains and retries from an exact seed and turns the samples into estimates with a Hoeffding confidence bound; estimates are Estimated, never Proven. `crates/axiom-infer` runs a model as a black box that it can only step, draws parameter points uniformly from a parametric model's ranges with a coverage statistic, and proposes a candidate fixed point and contraction factor from the observed steps with Hoeffding bounds; it may depend only on `axiom-core`, `axiom-model` and `axiom-sim`. `crates/axiom-cli` builds the `axiom` command line program described below. `crates/axiom-wasm` compiles the engine to WebAssembly for the web interface. Each crate keeps its tests in its own `tests/` folder. |
| `lean/` | The Lean 4 project for the `AxiomCheck` checker, pinned to Lean v4.19.0 and mathlib v4.19.0. `AxiomCheck/Json.lean` is the strict JSON reader and `AxiomCheck/Parse.lean` the certificate reader; their tests are in `lean/AxiomCheckTest/`. `lean/scripts/AxiomAudit.lean` implements the axiom audit. `lean/AxiomCheckCli/Main.lean` is the entry point that `axiom check` runs. |
| `lean-engine/`, `proof3.toml` | Proof 3: a second Lean project, pinned to Lean v4.35.0 and the mathlib that the pinned Aeneas release requires, separate from the checker. `AxiomEngine/Generated/Funs.lean` and `Types.lean` are written by Aeneas from `axiom_core::kernel` and never edited by hand. `FunsExternal.lean` and `TypesExternal.lean` model the engine's rational interface, `axiom_core::exact_rational`, as Lean's `Rat`; this model is what proof 3 trusts. `AxiomEngine/Correctness.lean` proves that each translated function never panics and computes what `AxiomCheck.Semantics` says, using the checker's own semantics source from `lean/`. |
| `fixtures/certificates/` | Certificates shared by the engine's and the checker's tests: a golden file both must read, and rejection fixtures both must reject. |
| `fixtures/models/` | Model files with hand-computed expected results in sibling `.expected.json` files, and in `reject/` faulty models, each with the fault and its line and column. |
| `ui/` | The web interface, written in Vue 3 and TypeScript and built with Vite and Tailwind CSS. It talks to the engine through the `Engine` interface in `ui/src/engine/types.ts`, implemented by `ui/src/engine/wasm.ts` on top of `crates/axiom-wasm`. The browser does not run the Lean checker, so the interface labels every claim Unchecked. For a two-state chain whose exit probabilities are its parameters, such as `fixtures/models/links-param.axm`, it shows the `ParametricContraction` claim for the whole range and analyses one point of the range through `analyse_at`, without rewriting the model text; a point is at most 256 bytes with at most 9 digits per number, and must lie inside both intervals, open ends excluded. The Simulate panel runs `axiom-sim` through `crates/axiom-wasm` in a Web Worker (`ui/src/engine/simWorker.ts`); a request is capped at 200 000 samples, 1000 steps, 1000 attempts and 1024 bytes, and the model at 64 KiB, so one run cannot hang the tab. Its estimates are labelled Estimated, never Proven. |
| `scripts/` | The checks run by the Git hooks and by CI. |
| `.github/` | CI workflows, the pull request template, issue templates and label definitions. |

## Command line

The `axiom` program is built from `crates/axiom-cli`. Run it with `cargo run -p axiom-cli --` followed by one of these commands:

| Command | What it does |
| --- | --- |
| `axiom analyse <MODEL> --cert <OUT>` | Reads a model and writes a certificate of the strongest claims the engine can make: a `Mass` claim for every row of a chain, with `FixedPoint`, `Contraction` and `ConvergenceBound` claims for a two-state chain that settles; a `ParametricContraction` claim for a two-state chain whose exit probabilities are its two parameters `a` and `b`, provided `0 < a + b < 2` across their whole ranges; or a `Termination` claim for a retry loop. Other parametric models are refused, because no claim kind covers them. |
| `axiom analyse <MODEL> --cert <OUT> --estimate --seed <U64> [--epsilon <FRACTION>] [--delta <FRACTION>]` | Also samples the model from the seed and adds an `Estimate` claim: the probability of being in the second state after 64 steps from the first for a two-state chain, or of success within 20 attempts for a retry loop. The sample count is the least that meets Hoeffding's bound for the tolerance (default `1/100`) and failure probability (default `1/1000`), at most 10,000,000. Without `--estimate` the certificate is unchanged. |
| `axiom explore <MODEL> --seed <U64> --cert <OUT> [--black-box] [--epsilon <FRACTION>] [--delta <FRACTION>]` | Runs a two-state chain without parameters as a black box from the seed, counts how often each state is left, and proposes a candidate fixed point $\hat a / (\hat a + \hat b)$ and contraction factor $1 - \hat a - \hat b$, exact ratios of the counts, with bounds that hold with probability at least $1 - \delta$ (defaults as for `analyse --estimate`). It flags the chain non-convergent when the bounds admit $\lvert\lambda\rvert = 1$. It writes a certificate, runs the Lean checker on it, and labels each sampled candidate `estimated (sampled, not a proof)`. When the model is supplied and the checker accepted the engine's exact claim, the line adds that exact value with `proven (checked by Lean)`, so the proof belongs to the exact value, never to the sampled one; with `--black-box` no exact value is proven. With `--black-box` the certificate holds only the `Mass` claims of the model the format must embed and an `Estimate` of the steady state. Retries and parametric chains are refused with `explore-unsupported`. |
| `axiom check <CERT> [--lean-dir <DIR>] [--timeout-secs <SECONDS>]` | Runs the Lean checker on a certificate and prints one line per claim, such as `claim 0 Mass row 0: accepted (proven)` or `claim 5 Estimate: accepted (estimated)`, followed by the number of accepted claims, how many are proven and how many estimated, and the time taken. An accepted `Estimate` claim means only that its sample count satisfies Hoeffding's bound for its tolerance and failure probability; the checker never confirms the estimated value. `axiom check` requires the certainty word to match the claim kind and treats any other line as unexpected checker output. |
| `axiom --help`, `axiom --version` | Prints the help, which lists the exit codes, or the version. |

The exit codes are stable:

| Code | Meaning |
| --- | --- |
| 0 | The command succeeded and every claim was accepted, whether proven or estimated. |
| 1 | The checker rejected at least one claim. |
| 2 | The model or the certificate was refused as invalid or unsupported. |
| 3 | An operational failure: a file could not be read or written, or the checker could not run, ran past its time limit or printed something unexpected. |
| 64 | A usage error: the command line is not a valid command. |

Errors are printed on one line as `axiom: <code>: <message>`, for example `axiom: missing-row: line 2, column 14: state has no row`. The code is a stable kebab-case name, and the message names files by role (model or certificate) and states positions, counts and limits only, so the same failure prints the same bytes on every machine and from every directory. Messages never quote the input, a path, an operating system message or the checker's error stream. A usage error names at most the option it concerns and never repeats what was typed. Model and certificate errors keep the codes of `axiom-model` and `axiom-cert`, which the web interface's diagnostics also use.

`axiom check` runs `lake env lean --run AxiomCheckCli/Main.lean <CERT>` in the Lean project directory, which is `lean/` in the repository the program was built from unless `--lean-dir` or the `AXIOM_LEAN_DIR` environment variable names another one. It therefore needs `lake` on `PATH` and a built checker (`cd lean && lake build`). The checker is killed after 60 seconds unless `--timeout-secs` or `AXIOM_CHECK_TIMEOUT_SECS` sets another limit, a whole number of seconds of at least 1. The checker's `error:` lines are mapped onto codes beginning `checker-`: each reading rule the checker can name has its own code, such as `checker-non-canonical-fraction` or `checker-json-leading-zero`, and a test fails if the Lean reader gains a reason the table does not know. The end-to-end tests of `axiom check` print a message and pass without checking anything when `lake` is not on `PATH`, unless `CI` is set.

Black-box results have limits that `axiom explore` prints on its last line. They cannot show what happens in regions of the parameter space that no sample reached, nor transitions rarer than the sampling resolution $\varepsilon$, which the counts cannot tell apart from zero, and an estimate is not a proof: its bounds fail with probability up to $\delta$, and the checker confirms only that its sample count meets Hoeffding's bound.

## Web interface

The web interface lives in `ui/` and has its own dependencies, pinned to exact versions in `ui/package.json` and locked in `ui/bun.lock`. Run these commands from `ui/`:

| Command | What it does |
| --- | --- |
| `bun install` | Installs the dependencies. |
| `bun run dev` | Starts a development server that reloads on every change. |
| `bun run typecheck` | Typechecks the TypeScript and Vue sources in strict mode. |
| `bun run wasm` | Compiles `crates/axiom-wasm` to WebAssembly and writes the bindings into `ui/src/engine/wasm/`, which Git ignores. Needs the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`) and `wasm-bindgen-cli` 0.2.130 (`cargo install wasm-bindgen-cli --version =0.2.130 --locked`). `bun run typecheck` and `bun run build` run it first. |
| `bun run build` | Builds the static site into `ui/dist/`, which Git ignores. |

## Git hooks

The hooks are managed by lefthook and defined in `lefthook.yml`.

| Hook | Checks |
| --- | --- |
| pre-commit | Rust formatting; forbidden Lean constructs; decorative separator comments and comments that trail code on the same line; Markdown paragraphs split across several lines; validity of YAML, TOML and shell files; workflow lint when actionlint is installed; a guard that refuses to commit local planning files. |
| commit-msg | The commit message format described below. |
| pre-push | Clippy, the Rust tests, the forbidden-construct search, the Lean tests and the axiom audit. |

A hook whose script cannot find a JavaScript runtime skips instead of failing, so that commits from graphical Git clients with a restricted environment are not blocked. CI runs the same checks and does not skip them.

## Commits

Each commit message is a single line of the form `type(scope): subject`, with no body. The type is one of `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`, `perf` and `revert`. The scope is optional and is one of `core`, `infer`, `circuit`, `cert`, `model`, `sim`, `cli`, `lean`, `examples`, `docs`, `ci`, `deps`, `ui` and `wasm`. The subject starts with a letter and states what changed, for example `fix(cert): reject certificates whose rows do not sum to one`. Commits carry no co-author trailers.

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
