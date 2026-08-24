#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

run_step() {
  local label="$1"
  shift
  printf '\n==> %s\n' "${label}"
  "$@"
}

main() {
  cd "${REPO_ROOT}"

  run_step "cargo fmt --check" cargo fmt --check
  run_step "browser websocket security tests" cargo test browser::websocket -- --nocapture
  run_step "updater security tests" cargo test ui::updater -- --nocapture
  run_step "script and contract tests" pnpm run test:scripts
  run_step "frontend production build" pnpm run build
  run_step "git whitespace check" git diff --check

  printf '\nsecurity regression check passed\n'
}

main "$@"
