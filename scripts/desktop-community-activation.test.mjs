import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const [commands, cli, app, buildScript, readme, building, releaseWorkflow, windowsWorkflow, macosWorkflow, macosDelivery, windowsSmoke, windowsInstallerSmoke] = await Promise.all([
  readFile(new URL('../src/rust/ui/commands.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src/rust/app/cli.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src/frontend/App.vue', import.meta.url), 'utf8'),
  readFile(new URL('../build.rs', import.meta.url), 'utf8'),
  readFile(new URL('../README.md', import.meta.url), 'utf8'),
  readFile(new URL('../BUILDING.md', import.meta.url), 'utf8'),
  readFile(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8'),
  readFile(new URL('../.github/workflows/windows-package.yml', import.meta.url), 'utf8'),
  readFile(new URL('../.github/workflows/macos-sign-notarize.yml', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/prepare-macos-delivery.sh', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/windows-install-smoke.ps1', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/windows-installer-smoke.ps1', import.meta.url), 'utf8'),
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

test('MCP popup shells bypass product activation in both UI and Rust', () => {
  const launchContextIndex = app.indexOf('mcpLaunchContext.value = await resolveMcpLaunchContext()')
  const activationQueryIndex = app.indexOf('const activationGateRequired = await requiresActivationGate()')
  assert.ok(launchContextIndex >= 0 && launchContextIndex < activationQueryIndex)
  assert.match(app, /if \(mcpLaunchContext\.value\.isMcp\) \{[\s\S]*initializeApplication\(\{ mcpShell: true \}\)[\s\S]*return/)
  assert.match(app, /if \(mcpShellMode\.value\)\s*return false/)
  assert.match(commands, /activation_gate_required_for_build\([\s\S]*is_mcp_shell/)
  assert.match(commands, /is_desktop\s*&&\s*!is_mcp_shell/)
})

test('community release artifacts expose a runtime activation gate receipt', () => {
  assert.match(cli, /--activation-gate-status/)
  for (const workflow of [releaseWorkflow, windowsWorkflow, macosWorkflow]) {
    assert.match(workflow, /ITERATE_REQUIRE_ACTIVATION:\s*['"]?0['"]?/)
    assert.doesNotMatch(workflow, /ITERATE_REQUIRE_ACTIVATION:\s*['"]?1['"]?/)
  }
  assert.match(macosDelivery, /--activation-gate-status/)
  assert.match(windowsSmoke, /--activation-gate-status/)
  assert.match(windowsInstallerSmoke, /--activation-gate-status/)
})

test('public documentation says community builds are usable without a code', () => {
  assert.match(readme, /社区构建默认不需要激活码/)
  assert.match(building, /社区构建默认免激活/)
})
