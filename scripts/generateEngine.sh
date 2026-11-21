#!/usr/bin/env bash
#
# Regenerates the Lean translation of the engine functions covered by proof 3.
# Charon extracts the functions listed in proof3.toml from axiom-core, keeping
# the rational interface opaque, and Aeneas writes lean-engine/AxiomEngine/
# Generated/. The tools come from scripts/installAeneas.sh, at the pinned
# release in proof3.toml; AENEAS_DIR names the directory that holds them.
# With --check, it regenerates into a temporary directory and fails if the
# result differs from the committed files.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
aeneas_dir="${AENEAS_DIR:-${repo_root}/.aeneas}"
config="${repo_root}/proof3.toml"
committed="${repo_root}/lean-engine/AxiomEngine/Generated"

check=false
if [[ "${1:-}" == "--check" ]]; then
  check=true
fi

read_key() {
  sed -n "s/^$1 = \"\\(.*\\)\"\$/\\1/p" "${config}"
}

start_from="$(read_key start_from)"
opaque="$(read_key opaque)"
rust_nightly="$(read_key rust_nightly)"
if [[ -z "${start_from}" || -z "${opaque}" || -z "${rust_nightly}" ]]; then
  echo "proof3.toml lacks start_from, opaque or rust_nightly" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

(
  cd "${repo_root}/crates/axiom-core"
  RUSTUP_TOOLCHAIN="${rust_nightly}" "${aeneas_dir}/charon" cargo \
    --preset=aeneas \
    --error-on-warnings \
    --start-from "${start_from}" \
    --opaque "${opaque}" \
    --dest-file "${work}/axiom_core.llbc"
)

"${aeneas_dir}/aeneas" -backend lean -split-files \
  -namespace AxiomEngine -subdir AxiomEngine/Generated \
  -dest "${work}/out" "${work}/axiom_core.llbc"

# Aeneas also writes *_Template.lean files that declare the opaque interface
# as axioms. They are not kept: FunsExternal.lean and TypesExternal.lean model
# that interface by definitions instead, and the build fails if a signature
# no longer matches them.
generated="${work}/out/AxiomEngine/Generated"
stale=false
for file in Funs.lean Types.lean; do
  if [[ "${check}" == true ]]; then
    if ! diff "${committed}/${file}" "${generated}/${file}"; then
      stale=true
    fi
  else
    mkdir -p "${committed}"
    cp "${generated}/${file}" "${committed}/${file}"
  fi
done

if [[ "${stale}" == true ]]; then
  echo "lean-engine/AxiomEngine/Generated is stale; run scripts/generateEngine.sh" >&2
  exit 1
fi
if [[ "${check}" == true ]]; then
  echo "lean-engine/AxiomEngine/Generated matches the pinned tools"
else
  echo "regenerated lean-engine/AxiomEngine/Generated"
fi
