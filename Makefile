.PHONY: engine-subset engine-install engine-translate engine-check rust-test-without-lean cli-lean-test cli-demo ui-check lean-test rust-deny help hooks-install check fmt rust-fmt rust-lint rust-test rust-doc lean-build lean-check lean-cache lean-audit

help: ## List targets
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | sed 's/:.*## /\t/'

hooks-install: ## Install the lefthook git hooks and the gate script dependencies
	bun install
	bunx lefthook install

check: rust-fmt rust-lint rust-test rust-doc lean-check lean-build lean-test lean-audit engine-subset engine-translate engine-check ui-check ## Everything CI runs

fmt: ## Format Rust sources in place
	cargo fmt --all

rust-fmt: ## Verify Rust formatting
	cargo fmt --all -- --check

rust-lint: ## Clippy with warnings as errors
	cargo clippy --workspace --all-targets --locked -- -D warnings

rust-test: ## Run Rust tests
	cargo test --workspace --locked

rust-test-without-lean: ## Run Rust tests except those that run the Lean checker
	cargo test --workspace --locked -- --skip lean_

cli-lean-test: ## Run the axiom check tests that run the Lean checker (requires lake)
	cargo test -p axiom-cli --locked --test check -- lean_

rust-deny: ## Check advisories, licences, bans and sources with cargo-deny (requires cargo-deny)
	cargo deny check

rust-doc: ## Build docs with warnings as errors
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

lean-cache: ## Fetch prebuilt mathlib oleans
	cd lean && lake exe cache get

lean-check: ## Fast textual pre-check for forbidden Lean constructs
	./scripts/checkLean.sh

lean-build: lean-cache ## Build the Lean checker with warnings as errors
	cd lean && lake build

lean-test: ## Run the Lean checker tests against the shared fixtures
	./scripts/testLean.sh

lean-audit: lean-cache ## Fail unless every checker declaration uses only the standard axioms
	./scripts/checkLeanAxioms.sh

engine-subset: ## Fast check that the proof 3 functions stay in the translatable subset
	./scripts/checkProof3.sh

engine-install: ## Fetch the Charon and Aeneas release pinned in proof3.toml into .aeneas/
	./scripts/installAeneas.sh

engine-translate: ## Fail unless Charon and Aeneas regenerate the committed Lean translation
	./scripts/checkProof3.sh --translate

engine-check: ## Build the proof 3 Lean project and audit its axioms
	cd lean-engine && lake exe cache get
	./scripts/checkEngine.sh

cli-demo: ## Write the golden certificate from links.axm and check it with the Lean checker
	cargo run -q -p axiom-cli -- analyse fixtures/models/links.axm --cert target/links-mass.json
	cargo run -q -p axiom-cli -- check target/links-mass.json

ui-check: ## Install the web interface dependencies, typecheck it and build it
	cd ui && bun install --frozen-lockfile && bun run typecheck && bun run build
