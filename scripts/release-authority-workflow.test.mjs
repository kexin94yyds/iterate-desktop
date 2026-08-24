import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const manualRelease = readFileSync('.github/workflows/manual-release.yml', 'utf8')
const taggedRelease = readFileSync('.github/workflows/release.yml', 'utf8')
const releaseParity = readFileSync('scripts/iterate-release-parity-check.sh', 'utf8')

test('manual release checks out and pushes only the configured release authority ref', () => {
  assert.match(
    manualRelease,
    /RELEASE_AUTHORITY_REF:\s*\$\{\{\s*vars\.RELEASE_AUTHORITY_REF\s*\}\}/,
  )
  assert.match(
    manualRelease,
    /if \[\[ -z "\$RELEASE_AUTHORITY_REF" \]\]; then[\s\S]*?exit 1/,
  )
  assert.match(
    manualRelease,
    /uses:\s*actions\/checkout@v4[\s\S]*?ref:\s*\$\{\{\s*env\.RELEASE_AUTHORITY_REF\s*\}\}/,
  )
  assert.match(
    manualRelease,
    /git push origin "HEAD:refs\/heads\/\$\{RELEASE_AUTHORITY_REF\}"/,
  )
  assert.doesNotMatch(manualRelease, /git push origin main/)
})

test('tagged release dispatches follow-up workflows from the configured authority ref', () => {
  assert.match(
    taggedRelease,
    /RELEASE_AUTHORITY_REF:\s*\$\{\{\s*vars\.RELEASE_AUTHORITY_REF\s*\}\}/,
  )
  assert.match(
    taggedRelease,
    /const authorityRef = process\.env\.RELEASE_AUTHORITY_REF;/,
  )
  assert.match(
    taggedRelease,
    /if \(!authorityRef\)[\s\S]*?throw new Error\(/,
  )
  assert.match(taggedRelease, /ref:\s*authorityRef,/)
  assert.doesNotMatch(taggedRelease, /ref:\s*['"]main['"]/)
})

test('tagged release requires the tag to be reachable from the release authority', () => {
  assert.match(taggedRelease, /Verify tag is on the release authority/)
  assert.match(
    taggedRelease,
    /git merge-base --is-ancestor "\$\{TAG_COMMIT\}" "origin\/\$\{RELEASE_AUTHORITY_REF\}"/,
  )
})

test('release builds use the lockfile and grant write access only to the publishing job', () => {
  assert.match(taggedRelease, /build-cli:[\s\S]*?permissions:\s*\n\s*contents:\s*read/)
  assert.match(taggedRelease, /Install frontend dependencies[\s\S]*?pnpm install --frozen-lockfile/)
  assert.match(taggedRelease, /release:\s*\n\s*name: Create Release[\s\S]*?permissions:\s*\n\s*contents:\s*write/)
})

test('macOS public release fails closed without fresh signed and notarized assets', () => {
  assert.match(
    taggedRelease,
    /Required macOS signing \/ notarization secrets are incomplete; refusing to publish/,
  )
  assert.match(
    taggedRelease,
    /if \[\[ -z "\$\{MAC_DMG\}" \|\| -z "\$\{MAC_ZIP\}" \]\]; then[\s\S]*?exit 1/,
  )
  assert.doesNotMatch(taggedRelease, /reusing the current public latest macOS DMG/)
})

test('public release signs executable assets and records provenance', () => {
  assert.match(taggedRelease, /Sign public release assets and write provenance/)
  assert.match(taggedRelease, /ITERATE_RELEASE_PRIVATE_KEY_B64/)
  assert.match(taggedRelease, /scripts\/release-sign-assets\.mjs --out-dir stable-assets/)
  assert.match(taggedRelease, /stable-assets\/iterate_\$\{GITHUB_REF_NAME#v\}_aarch64\.dmg/)
  assert.match(taggedRelease, /stable-assets\/provenance\.json/)
  assert.match(taggedRelease, /source_commit:\s*process\.env\.TAG_COMMIT/)
  assert.match(taggedRelease, /authority_ref:\s*process\.env\.RELEASE_AUTHORITY_REF/)
})

test('public release remains a draft until every stable asset is uploaded', () => {
  assert.match(taggedRelease, /draft:\s*true/)
  assert.match(taggedRelease, /No stable-assets dir found; refusing to publish an empty release/)
  assert.match(taggedRelease, /repos\.updateRelease\(\{[\s\S]*?draft:\s*false/)
})

test('public release uses the public distribution repo as changelog baseline and fails any asset upload', () => {
  assert.match(
    taggedRelease,
    /api\.github\.com\/repos\/kexin94yyds\/iterate-releases\/releases\/latest/,
  )
  assert.match(taggedRelease, /\\\.\(dmg\|zip\|md\|sha256\|sig\|json\)\$/)
  assert.match(
    taggedRelease,
    /Failed to upload[\s\S]*?process\.exit\(1\)/,
  )
})

test('manual release keeps the README badge aligned with version manifests', () => {
  assert.match(
    manualRelease,
    /s\/version-\[0-9\]\[0-9\.\]\*-blue\/version-\$NEW_VERSION-blue\//,
  )
  assert.match(manualRelease, /Unable to locate the cunzhi root package entry in Cargo\.lock/)
})

test('release parity can inspect a candidate app and bounds every network probe', () => {
  assert.match(releaseParity, /APP_PATH="\$\{APP_PATH:-\/Applications\/iterate\.app\}"/)
  assert.match(releaseParity, /"\$\{APP_PATH\}\/Contents\/Info\.plist"/)
  assert.match(releaseParity, /--connect-timeout "\$\{CURL_CONNECT_TIMEOUT_SECONDS\}"/)
  assert.match(releaseParity, /--max-time "\$\{CURL_MAX_TIME_SECONDS\}"/)
})
