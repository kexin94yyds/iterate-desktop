/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeBridgeUrlForFetch } from './bridgeFetch.ts'

test('desktop bridge fetch accepts only the exact loopback service origin', () => {
  assert.equal(
    normalizeBridgeUrlForFetch('http://127.0.0.1:8080/api/config?tab=mobile').pathname,
    '/api/config',
  )
  assert.equal(
    normalizeBridgeUrlForFetch('http://[::1]:8080/api/version').hostname,
    '[::1]',
  )

  for (const url of [
    'https://127.0.0.1:8080/api/config',
    'http://127.0.0.1:8081/api/config',
    'http://192.168.1.5:8080/api/config',
    'http://example.com:8080/api/config',
    'http://user@localhost:8080/api/config',
  ]) {
    assert.throws(() => normalizeBridgeUrlForFetch(url), /bridge_fetch_requires_loopback_8080/)
  }
})
