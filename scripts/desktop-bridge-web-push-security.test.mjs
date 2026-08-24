import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const bridge = await readFile(
  new URL('../src/rust/bridge/ws.rs', import.meta.url),
  'utf8',
)

test('Web Push egress is provider allowlisted and never follows redirects', () => {
  assert.match(bridge, /host == "fcm\.googleapis\.com"/)
  assert.match(bridge, /host == "web\.push\.apple\.com"/)
  assert.match(bridge, /host\.ends_with\("\.push\.services\.mozilla\.com"\)/)
  assert.match(bridge, /host\.ends_with\("\.notify\.windows\.com"\)/)
  assert.match(bridge, /\.redirect\(reqwest::redirect::Policy::none\(\)\)/)
  assert.match(bridge, /host\.parse::<std::net::IpAddr>\(\)\.is_ok\(\)/)
})

test('Web Push registration validates fields and bounds subscription growth', () => {
  assert.match(bridge, /MAX_WEB_PUSH_SUBSCRIPTIONS: usize = 32/)
  assert.match(bridge, /MAX_WEB_PUSH_ENDPOINT_LENGTH: usize = 2048/)
  assert.match(bridge, /validate_web_push_subscription\(&subscription\)/)
  assert.match(bridge, /StatusCode::TOO_MANY_REQUESTS/)
  assert.match(bridge, /web_push_subscription_capacity_available/)
})

test('Web Push Rust regressions cover private, unknown and credentialed targets', () => {
  for (const rejectedTarget of [
    'https://127.0.0.1/push',
    'https://[::1]/push',
    'https://localhost/push',
    'https://10.0.0.1/push',
    'https://push.example.test/send',
    'https://user:secret@fcm.googleapis.com/send',
    'https://fcm.googleapis.com:8443/send',
  ]) {
    assert.ok(bridge.includes(JSON.stringify(rejectedTarget)))
  }
})
