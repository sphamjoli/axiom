#!/usr/bin/env bash
#
# Validates that each TOML file passed as an argument parses, using python3's
# standard-library tomllib (Python 3.11+). Fails open when that is unavailable
# (see lefthook.yml).
#
# Arguments are the files to check (lefthook passes the staged TOML files).
# Exit status is non-zero only if a present parser rejects a file.
set -euo pipefail

# shellcheck source=scripts/lib/prepend-common-bin-dirs.sh
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/prepend-common-bin-dirs.sh"
prepend_common_bin_dirs

if ! command -v python3 >/dev/null 2>&1; then
  echo "checkToml: python3 not found; skipping TOML validation" >&2
  exit 0
fi
if ! python3 -c 'import tomllib' >/dev/null 2>&1; then
  echo "checkToml: python3 lacks tomllib (needs 3.11+); skipping TOML validation" >&2
  exit 0
fi

detail="$(mktemp)"
trap 'rm -f "$detail"' EXIT

status=0
for file in "$@"; do
  [ -f "$file" ] || continue
  if ! python3 -c 'import pathlib, sys, tomllib; tomllib.loads(pathlib.Path(sys.argv[1]).read_text())' "$file" 2>"$detail"; then
    echo "TOML validation failed: $file" >&2
    sed 's/^/  /' "$detail" >&2
    status=1
  fi
done

exit "$status"
