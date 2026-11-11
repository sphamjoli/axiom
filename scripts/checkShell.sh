#!/usr/bin/env bash
#
# Syntax-checks each shell script passed as an argument with `bash -n` (parse,
# do not execute), so a syntax error in a hook or helper script surfaces before
# the next time it runs.
#
# Arguments are the files to check (lefthook passes the staged shell files).
# Exit status is non-zero if any file fails to parse.
set -euo pipefail

detail="$(mktemp)"
trap 'rm -f "$detail"' EXIT

status=0
for file in "$@"; do
  [ -f "$file" ] || continue
  if ! bash -n "$file" 2>"$detail"; then
    echo "Shell validation failed: $file" >&2
    sed 's/^/  /' "$detail" >&2
    status=1
  fi
done

exit "$status"
