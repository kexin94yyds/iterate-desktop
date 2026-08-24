import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

test('macOS native speech bridge build is gated by Cargo TARGET', () => {
  const source = readFileSync(new URL('../build.rs', import.meta.url), 'utf8')

  assert.match(source, /std::env::var\("TARGET"\)/)
  assert.match(source, /apple-darwin/)
  assert.doesNotMatch(source, /#\[cfg\(target_os\s*=\s*"macos"\)\]/)
})
