#!/usr/bin/env bash
#
# Shared by the checkers that shell out to a system tool (ruby, python3), so a
# Git GUI's minimal-environment shell still finds it. Only `/usr/bin` is added:
# it exists on every Linux and macOS install, and a missing directory is
# skipped.
prepend_common_bin_dirs() {
  _prepend_path() {
    case ":${PATH:-}:" in
      *":$1:"*) ;;
      *) [ -d "$1" ] && PATH="$1:${PATH:-}" ;;
    esac
  }
  _prepend_path /usr/bin
  export PATH
}
