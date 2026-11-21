#!/usr/bin/env bash
#
# Keeps the engine functions covered by proof 3 inside the translatable subset
# of ADR-004. First a fast textual check that needs no tools: the functions in
# axiom_core::kernel and the [[covered]] entries of proof3.toml name the same
# set, and the kernel uses no construct outside the subset (unsafe, trait
# objects, interior mutability, loops, closures, iterator adaptors, `?`, or
# `BigRational` instead of `ExactRational`). Then, with --translate, the
# authority: Charon and Aeneas from the pinned release translate the kernel,
# fail on any unsupported construct or warning, and the output must equal the
# committed lean-engine/AxiomEngine/Generated.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
kernel="${repo_root}/crates/axiom-core/src/kernel.rs"
config="${repo_root}/proof3.toml"

defined="$(sed -n 's/^pub(crate) fn \([a-z_][a-z0-9_]*\).*/\1/p' "${kernel}" | sort)"
listed="$(sed -n 's/^rust = "axiom_core::kernel::\([a-z0-9_]*\)"$/\1/p' "${config}" | sort)"
if [[ -z "${defined}" || "${defined}" != "${listed}" ]]; then
  echo "checkProof3: kernel functions and proof3.toml [[covered]] entries differ" >&2
  diff <(echo "${defined}") <(echo "${listed}") >&2 || true
  exit 1
fi

# Strips line comments, then reports any line outside the subset.
forbidden='\bunsafe\b|\bdyn\b|\b(Cell|RefCell|Mutex|RwLock|Atomic[A-Za-z0-9]*)\b|\b(for|while|loop)\b|\|[^|]*\||\.(map|iter|into_iter|successors|take|fold)\(|\?|\bBigRational\b|\bimpl\b|\bunwrap\b|\bexpect\b|\bpanic!'
if sed 's://.*$::' "${kernel}" | grep -nE "${forbidden}"; then
  echo "checkProof3: kernel.rs uses a construct outside the translatable subset" >&2
  exit 1
fi

if [[ "${1:-}" == "--translate" ]]; then
  "${repo_root}/scripts/generateEngine.sh" --check
fi
echo "checkProof3: covered functions are inside the subset"
