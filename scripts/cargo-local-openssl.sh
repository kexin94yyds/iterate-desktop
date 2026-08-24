#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OPENSSL_DIR="${OPENSSL_DIR:-/opt/homebrew/opt/openssl@3}"

if [[ ! -d "$OPENSSL_DIR" ]]; then
  echo "OPENSSL_DIR not found: $OPENSSL_DIR" >&2
  echo "Install OpenSSL with Homebrew or set OPENSSL_DIR to a valid OpenSSL prefix." >&2
  exit 1
fi

export OPENSSL_NO_VENDOR="${OPENSSL_NO_VENDOR:-1}"
export OPENSSL_DIR
export PKG_CONFIG_PATH="${OPENSSL_DIR}/lib/pkgconfig${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"

cd "$ROOT_DIR"

if [[ $# -eq 0 ]]; then
  set -- check --bin iterate -j1
fi

exec cargo "$@"
