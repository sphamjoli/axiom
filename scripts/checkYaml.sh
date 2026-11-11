#!/usr/bin/env bash
#
# Validates that each YAML file passed as an argument parses, using ruby's
# standard-library YAML (present on macOS and the CI ubuntu runners). Fails open
# when ruby is missing (see lefthook.yml).
#
# Arguments are the files to check (lefthook passes the staged YAML files).
# Exit status is non-zero if any file fails to parse.
set -euo pipefail

# shellcheck source=scripts/lib/prepend-common-bin-dirs.sh
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/prepend-common-bin-dirs.sh"
prepend_common_bin_dirs

if ! command -v ruby >/dev/null 2>&1; then
  echo "checkYaml: ruby not found; skipping YAML validation" >&2
  exit 0
fi

detail="$(mktemp)"
trap 'rm -f "$detail"' EXIT

status=0
for file in "$@"; do
  [ -f "$file" ] || continue
  # Psych.parse_stream validates syntax only: it parses every document in the
  # file (multi-doc aware) and tolerates anchors, aliases, and custom tags,
  # unlike YAML.load_file which under Psych 4 (Ruby 3.1+ on the CI runners)
  # safe-loads and would reject valid anchored YAML. All we assert here is that
  # the file is well-formed YAML.
  if ! ruby -ryaml -e 'Psych.parse_stream(File.read(ARGV[0]))' "$file" 2>"$detail"; then
    echo "YAML validation failed: $file" >&2
    sed 's/^/  /' "$detail" >&2
    status=1
  fi
done

exit "$status"
