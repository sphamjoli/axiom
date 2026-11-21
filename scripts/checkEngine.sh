#!/usr/bin/env bash
#
# Builds the proof 3 Lean project in lean-engine/ with warnings as errors, then
# elaborates lean-engine/scripts/AxiomAudit.lean, which fails unless every
# AxiomEngine declaration depends only on propext, Classical.choice and
# Quot.sound. The project pins its own Lean and mathlib, from the Aeneas
# release in proof3.toml, separate from the checker in lean/.

set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/lean-engine"
lake build
lake env lean scripts/AxiomAudit.lean
