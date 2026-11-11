#!/usr/bin/env bash
#
# Runs scripts/checkTrailingComments.mjs under whichever JS runtime is present, forwarding
# all arguments. Fails open when no runtime is found (see lefthook.yml).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# shellcheck source=scripts/lib/resolve-js-runtime.sh
. "${repo_root}/scripts/lib/resolve-js-runtime.sh"

if ! runtime="$(resolve_js_runtime)"; then
  echo "checkTrailingComments: no bun/node runtime found; skipping." >&2
  exit 0
fi

exec "${runtime}" "${repo_root}/scripts/checkTrailingComments.mjs" "$@"
