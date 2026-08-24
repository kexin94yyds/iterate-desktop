#!/usr/bin/env bash
set -euo pipefail

MANIFEST_FILE="${MANIFEST_FILE:-}"
MANIFEST_URL="${MANIFEST_URL:-}"
CHECK_URLS="${CHECK_URLS:-0}"
CHECK_DOWNLOADS="${CHECK_DOWNLOADS:-0}"
REQUIRE_CHECKSUM_ASSETS="${REQUIRE_CHECKSUM_ASSETS:-0}"
HTTP_TIMEOUT_SECS="${HTTP_TIMEOUT_SECS:-8}"
MAX_DOWNLOAD_BYTES="${MAX_DOWNLOAD_BYTES:-150000000}"

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf '[pass] %s\n' "$1"
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  printf '[warn] %s\n' "$1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf '[fail] %s\n' "$1"
}

require_cmd() {
  if command -v "$1" >/dev/null 2>&1; then
    pass "command available: $1"
  else
    fail "command missing: $1"
  fi
}

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

manifest_json() {
  if [[ -n "${MANIFEST_FILE}" ]]; then
    cat "${MANIFEST_FILE}"
  elif [[ -n "${MANIFEST_URL}" ]]; then
    curl -fsS \
      --noproxy '*' \
      -H "User-Agent: iterate-release-smoke" \
      -H "Accept: application/vnd.github+json" \
      -m "${HTTP_TIMEOUT_SECS}" \
      "${MANIFEST_URL}"
  else
    return 1
  fi
}

json_has_string() {
  local file="$1"
  local query="$2"
  jq -e "${query} | type == \"string\" and length > 0" "${file}" >/dev/null
}

check_platform() {
  local manifest="$1"
  local key="$2"
  local url signature

  if jq -e --arg key "${key}" '.platforms[$key]' "${manifest}" >/dev/null; then
    pass "platform entry exists: ${key}"
  else
    warn "platform entry absent: ${key}"
    return
  fi

  if json_has_string "${manifest}" ".platforms[\"${key}\"].url"; then
    pass "platform url exists: ${key}"
  else
    fail "platform url missing: ${key}"
  fi

  if json_has_string "${manifest}" ".platforms[\"${key}\"].signature"; then
    pass "platform signature exists: ${key}"
  else
    fail "platform signature missing: ${key}"
  fi

  if [[ "${CHECK_URLS}" == "1" ]]; then
    url="$(jq -r --arg key "${key}" '.platforms[$key].url // empty' "${manifest}")"
    signature="$(jq -r --arg key "${key}" '.platforms[$key].signature // empty' "${manifest}")"
    if [[ -n "${url}" ]] && curl -fsSI -H "User-Agent: iterate-release-smoke" -m "${HTTP_TIMEOUT_SECS}" "${url}" >/dev/null; then
      pass "platform artifact URL reachable: ${key}"
    else
      fail "platform artifact URL unreachable: ${key}"
    fi
    if [[ -n "${signature}" ]]; then
      pass "platform signature has non-empty value: ${key}"
    fi
  fi
}

check_github_release() {
  local manifest="$1"
  local asset_count checksum_asset_count download_dir mac_asset_count name sha size url

  if json_has_string "${manifest}" '.tag_name'; then
    pass "GitHub release tag_name exists"
  else
    fail "GitHub release tag_name missing"
  fi

  if jq -e '.assets | type == "array"' "${manifest}" >/dev/null; then
    pass "GitHub release assets array exists"
  else
    fail "GitHub release assets array missing"
    return
  fi

  asset_count="$(jq '.assets | length' "${manifest}")"
  if [[ "${asset_count}" -gt 0 ]]; then
    pass "GitHub release assets present: ${asset_count}"
  else
    fail "GitHub release assets empty"
  fi

  mac_asset_count="$(jq '[.assets[] | select((.name // "") | test("(?i)(mac|darwin|dmg|aarch64|x86_64)"))] | length' "${manifest}")"
  if [[ "${mac_asset_count}" -gt 0 ]]; then
    pass "GitHub release has macOS-like assets: ${mac_asset_count}"
  else
    warn "GitHub release has no obvious macOS assets"
  fi

  if jq -e '[.assets[] | select((.browser_download_url // "") | length > 0)] | length > 0' "${manifest}" >/dev/null; then
    pass "GitHub release assets include browser_download_url"
  else
    fail "GitHub release assets missing browser_download_url"
  fi

  checksum_asset_count="$(jq '[.assets[] | select((.name // "") | test("(?i)(sha256|checksum|checksums)"))] | length' "${manifest}")"
  if [[ "${checksum_asset_count}" -gt 0 ]]; then
    pass "GitHub release checksum-like assets present: ${checksum_asset_count}"
  elif [[ "${REQUIRE_CHECKSUM_ASSETS}" == "1" ]]; then
    fail "GitHub release checksum-like assets missing"
  else
    warn "GitHub release checksum-like assets absent"
  fi

  if [[ "${CHECK_URLS}" == "1" ]]; then
    while IFS= read -r url; do
      [[ -n "${url}" ]] || continue
      if curl -fsSI -H "User-Agent: iterate-release-smoke" -m "${HTTP_TIMEOUT_SECS}" "${url}" >/dev/null; then
        pass "GitHub release asset URL reachable: ${url}"
      else
        fail "GitHub release asset URL unreachable: ${url}"
      fi
    done < <(jq -r '.assets[]?.browser_download_url // empty' "${manifest}")
  fi

  if [[ "${CHECK_DOWNLOADS}" == "1" ]]; then
    require_cmd shasum
    download_dir="$(mktemp -d "${TMPDIR:-/tmp}/iterate-release-downloads.XXXXXX")"
    while IFS=$'\t' read -r name size url; do
      [[ -n "${url}" ]] || continue
      if [[ "${size}" -gt "${MAX_DOWNLOAD_BYTES}" ]]; then
        warn "GitHub release asset skipped, size exceeds MAX_DOWNLOAD_BYTES: ${name} (${size})"
        continue
      fi

      if curl -fsSL \
        -H "User-Agent: iterate-release-smoke" \
        --retry 2 \
        -m "${HTTP_TIMEOUT_SECS}" \
        -o "${download_dir}/${name}" \
        "${url}"; then
        pass "GitHub release asset downloaded: ${name} (${size} bytes)"
        sha="$(sha256_file "${download_dir}/${name}")"
        pass "GitHub release asset sha256 computed: ${name} ${sha}"
      else
        fail "GitHub release asset download failed: ${name}"
      fi
    done < <(jq -r '.assets[]? | [.name, ((.size // 0) | tostring), (.browser_download_url // "")] | @tsv' "${manifest}")
    rm -rf "${download_dir}"
  fi
}

main() {
  printf 'iterate macOS manifest smoke\n'
  printf 'manifest_file=%s\n' "${MANIFEST_FILE:-<not-set>}"
  printf 'manifest_url=%s\n' "${MANIFEST_URL:-<not-set>}"
  printf 'check_urls=%s\n' "${CHECK_URLS}"
  printf 'check_downloads=%s\n' "${CHECK_DOWNLOADS}"
  printf 'require_checksum_assets=%s\n' "${REQUIRE_CHECKSUM_ASSETS}"
  printf 'max_download_bytes=%s\n' "${MAX_DOWNLOAD_BYTES}"
  printf '\n'

  require_cmd jq
  require_cmd curl

  local tmp_manifest
  tmp_manifest="$(mktemp "${TMPDIR:-/tmp}/iterate-manifest.XXXXXX.json")"
  if ! manifest_json >"${tmp_manifest}"; then
    rm -f "${tmp_manifest}"
    fail "MANIFEST_FILE or MANIFEST_URL must be set"
    printf '\nsummary: pass=%s warn=%s fail=%s\n' "${PASS_COUNT}" "${WARN_COUNT}" "${FAIL_COUNT}"
    exit 1
  fi

  if jq -e . "${tmp_manifest}" >/dev/null; then
    pass "manifest is valid JSON"
  else
    fail "manifest is invalid JSON"
  fi

  if jq -e '.assets | type == "array"' "${tmp_manifest}" >/dev/null; then
    check_github_release "${tmp_manifest}"
    rm -f "${tmp_manifest}"
    printf '\nsummary: pass=%s warn=%s fail=%s\n' "${PASS_COUNT}" "${WARN_COUNT}" "${FAIL_COUNT}"

    if [[ "${FAIL_COUNT}" -gt 0 ]]; then
      exit 1
    fi
    return
  fi

  if json_has_string "${tmp_manifest}" '.version'; then
    pass "manifest version exists"
  else
    fail "manifest version missing"
  fi

  if jq -e '.platforms | type == "object"' "${tmp_manifest}" >/dev/null; then
    pass "manifest platforms object exists"
  else
    fail "manifest platforms object missing"
  fi

  check_platform "${tmp_manifest}" "darwin-aarch64"
  check_platform "${tmp_manifest}" "darwin-x86_64"

  rm -f "${tmp_manifest}"
  printf '\nsummary: pass=%s warn=%s fail=%s\n' "${PASS_COUNT}" "${WARN_COUNT}" "${FAIL_COUNT}"

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    exit 1
  fi
}

main "$@"
