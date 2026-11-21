#!/usr/bin/env bash
#
# Installs the Charon and Aeneas binaries of the release pinned in proof3.toml
# into AENEAS_DIR (default .aeneas/ at the repository root), and the Rust
# nightly that Charon drives. The archive is fetched from the release of the
# AeneasVerif/aeneas repository and verified against the SHA-256 recorded
# below before it is unpacked; a mismatch aborts. Moving the pin means
# updating proof3.toml and both checksums together.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
aeneas_dir="${AENEAS_DIR:-${repo_root}/.aeneas}"
config="${repo_root}/proof3.toml"

read_key() {
  sed -n "s/^$1 = \"\\(.*\\)\"\$/\\1/p" "${config}"
}

tag="$(read_key aeneas)"
charon_commit="$(read_key charon)"
rust_nightly="$(read_key rust_nightly)"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    asset="aeneas-linux-x86_64.tar.gz"
    sha256="051add48507c216449f42175e5dfc4b3c1cd378ff560307936a6f737c09471ea"
    ;;
  Darwin-arm64)
    asset="aeneas-macos-aarch64.tar.gz"
    sha256="e54af16c5c6c8c2b33433258389f5e28fc5c0bdef4684cc1705d441a678246e3"
    ;;
  *)
    echo "no pinned Aeneas release for $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

if [[ -x "${aeneas_dir}/aeneas" ]] && "${aeneas_dir}/aeneas" -version | grep -qx "aeneas ${tag}"; then
  echo "Aeneas ${tag} already in ${aeneas_dir}"
else
  download="$(mktemp -d)"
  trap 'rm -rf "${download}"' EXIT
  curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
    --output "${download}/${asset}" \
    "https://github.com/AeneasVerif/aeneas/releases/download/${tag}/${asset}"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${download}/${asset}" | cut -d' ' -f1)"
  else
    actual="$(shasum -a 256 "${download}/${asset}" | cut -d' ' -f1)"
  fi
  if [[ "${actual}" != "${sha256}" ]]; then
    echo "checksum mismatch for ${asset}: expected ${sha256}, got ${actual}" >&2
    exit 1
  fi
  rm -rf "${aeneas_dir}"
  mkdir -p "${aeneas_dir}"
  tar -xzf "${download}/${asset}" -C "${aeneas_dir}"
fi

if ! "${aeneas_dir}/charon" version | grep -q "(${charon_commit})"; then
  echo "the Charon in ${aeneas_dir} is not at ${charon_commit}" >&2
  exit 1
fi

rustup toolchain install "${rust_nightly}" --no-self-update --profile minimal \
  --component rustc-dev,llvm-tools,rust-src
