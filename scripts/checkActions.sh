#!/usr/bin/env bash
#
# Lints the GitHub Actions workflows with actionlint, which checks workflow
# syntax, expression types, job wiring, and shellchecks the run scripts. Fails
# open when actionlint is not installed (see lefthook.yml); install it with
# `brew install actionlint`. File arguments scope the lint to those workflows;
# with none, actionlint lints every workflow.

set -euo pipefail

# shellcheck source=scripts/lib/prepend-common-bin-dirs.sh
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/prepend-common-bin-dirs.sh"
prepend_common_bin_dirs

if ! command -v actionlint >/dev/null 2>&1; then
  echo "checkActions: actionlint not installed; skipping." >&2
  exit 0
fi

exec actionlint "$@"
