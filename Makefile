.PHONY: help hooks-install check fmt rust-fmt rust-lint rust-test rust-doc lean-build lean-check lean-cache lean-audit

help: ## List targets
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | sed 's/:.*## /\t/'

hooks-install: ## Install the lefthook git hooks and the gate script dependencies
	bun install
	bunx lefthook install

check: rust-fmt rust-lint rust-test rust-doc lean-check lean-build lean-audit ## Everything CI runs

fmt: ## Format Rust sources in place
	cargo fmt --all

rust-fmt: ## Verify Rust formatting
	cargo fmt --all -- --check

rust-lint: ## Clippy with warnings as errors
	cargo clippy --workspace --all-targets --locked -- -D warnings

rust-test: ## Run Rust tests
	cargo test --workspace --locked

rust-doc: ## Build docs with warnings as errors
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

lean-cache: ## Fetch prebuilt mathlib oleans
	cd lean && lake exe cache get

lean-check: ## Fast textual pre-check for forbidden Lean constructs
	./scripts/checkLean.sh

lean-build: lean-cache ## Build the Lean checker with warnings as errors
	cd lean && lake build

lean-audit: lean-cache ## Fail unless every checker declaration uses only the standard axioms
	./scripts/checkLeanAxioms.sh
