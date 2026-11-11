#!/usr/bin/env bash
#
# Shared by the `.mjs` gate wrappers to find a `bun` or `node` binary. Beyond
# PATH it checks bun's documented install location (`$BUN_INSTALL`, default
# `~/.bun`), because Git GUI clients often run hooks with a PATH that omits it.
#
# Usage: source this file, then call `resolve_js_runtime`, which prints the
# binary path, or prints nothing and returns non-zero when none is found.
resolve_js_runtime() {
  if command -v bun >/dev/null 2>&1; then
    command -v bun
    return 0
  fi
  if command -v node >/dev/null 2>&1; then
    command -v node
    return 0
  fi

  local bun_install="${BUN_INSTALL:-${HOME}/.bun}"
  if [[ -x "${bun_install}/bin/bun" ]]; then
    echo "${bun_install}/bin/bun"
    return 0
  fi
  if [[ -x "${bun_install}/bin/bun.exe" ]]; then
    echo "${bun_install}/bin/bun.exe"
    return 0
  fi

  return 1
}
