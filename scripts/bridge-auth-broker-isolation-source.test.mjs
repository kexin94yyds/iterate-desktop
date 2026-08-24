import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const authSource = await readFile(new URL('../src/rust/bridge/auth.rs', import.meta.url), 'utf8')
const bridgeSource = await readFile(new URL('../src/rust/bridge/ws.rs', import.meta.url), 'utf8')

test('auth broker tests use an injected temporary socket instead of the production path', () => {
  assert.match(authSource, /start_internal_auth_broker_at\(&socket_path\)/)
  assert.match(authSource, /request_token_from_broker_at\(\s*&socket_path,/)
  assert.match(authSource, /tempfile::tempdir\(\)/)
  assert.match(authSource, /production_socket_identity_before/)
  assert.match(authSource, /production_socket_identity_after/)
})

test('auth broker lifetime owns cleanup and never exposes a global unlink helper', () => {
  assert.match(authSource, /struct AuthBrokerSocketGuard/)
  assert.match(authSource, /metadata\.dev\(\) == self\.device/)
  assert.match(authSource, /metadata\.ino\(\) == self\.inode/)
  assert.match(authSource, /libc::LOCK_EX \| libc::LOCK_NB/)
  assert.doesNotMatch(authSource, /fn remove_internal_auth_broker_socket/)
  assert.doesNotMatch(bridgeSource, /remove_internal_auth_broker_socket/)
})

test('focused Rust regressions cover live-owner refusal and replacement preservation', () => {
  assert.match(authSource, /broker_refuses_to_replace_a_live_socket/)
  assert.match(authSource, /broker_guard_preserves_a_replacement_socket/)
})
