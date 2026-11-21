#!/usr/bin/env bash
#
# Builds the Lean checker and runs its tests in lean/AxiomCheckTest/ through
# the interpreter, with the same options the library is built with. The tests
# read the shared fixtures in fixtures/certificates/. Running them with
# `lean --run` instead of as a compiled executable avoids a binary-loading
# failure of Lean v4.19 executables on recent macOS, and behaves the same on
# Linux CI.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}/lean"

lake build
lake env lean \
  -DwarningAsError=true \
  -DautoImplicit=false \
  -DrelaxedAutoImplicit=false \
  -Dlinter.missingDocs=true \
  -Dlinter.unusedVariables=true \
  --run AxiomCheckTest/Main.lean
