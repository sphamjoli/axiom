#!/usr/bin/env bash
#
# Kernel authority for the Lean trusted computing base: builds the AxiomCheck
# library, then elaborates lean/scripts/AxiomAudit.lean, which fails unless every
# AxiomCheck declaration depends only on propext, Classical.choice, and
# Quot.sound.

set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/lean"
lake build
lake env lean scripts/AxiomAudit.lean
