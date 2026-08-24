import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const [commands, buildScript, readme, building] = await Promise.all([
  readFile(new URL('../src/rust/ui/commands.rs', import.meta.url), 'utf8'),
  readFile(new URL('../build.rs', import.meta.url), 'utf8'),
  readFile(new URL('../README.md', import.meta.url), 'utf8'),
  readFile(new URL('../BUILDING.md', import.meta.url), 'utf8'),
])

test('community desktop builds skip activation by default', () => {
  assert.match(commands, /option_env!\("ITERATE_REQUIRE_ACTIVATION"\)/)
  assert.match(commands, /activation_gate_required_for_build/)
  assert.match(commands, /is_desktop[\s\S]*build_flag\.is_some_and/)
  assert.doesNotMatch(commands, /pub async fn requires_activation_gate\(\) -> bool \{\s*cfg!\(/)
})

test('official commercial builds can explicitly opt into activation', () => {
  assert.match(commands, /"1" \| "true" \| "yes" \| "on"/)
  assert.match(buildScript, /cargo:rerun-if-env-changed=ITERATE_REQUIRE_ACTIVATION/)
  assert.match(building, /ITERATE_REQUIRE_ACTIVATION=1/)
})

test('public documentation says community builds are usable without a code', () => {
  assert.match(readme, /社区构建默认不需要激活码/)
  assert.match(building, /社区构建默认免激活/)
})
