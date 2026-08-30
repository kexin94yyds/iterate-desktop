import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
}

test('Windows registry liveness check does not spawn tasklist', () => {
  const registry = source('src/rust/ui/window_registry.rs')
  assert.doesNotMatch(registry, /Command::new\("tasklist"\)/)
  assert.match(registry, /OpenProcess\(PROCESS_QUERY_LIMITED_INFORMATION/)
})

test('bridge health probe uses reqwest only on Windows and preserves the Unix curl path', () => {
  const setup = source('src/rust/app/setup.rs')
  assert.match(setup, /#\[cfg\(target_os = "windows"\)\]\s*fn bridge_http_healthy[\s\S]*?reqwest::blocking::Client::builder/)
  assert.match(setup, /#\[cfg\(not\(target_os = "windows"\)\)\]\s*fn bridge_http_healthy[\s\S]*?command_stdout\("curl"/)
})

test('Windows shows the main window before background setup while non-Windows keeps blocking setup', () => {
  const builder = source('src/rust/app/builder.rs')
  const showIndex = builder.indexOf('window.show()')
  const setupIndex = builder.lastIndexOf('start_application_setup(app_handle)')
  assert.ok(showIndex >= 0 && setupIndex > showIndex)
  assert.match(builder, /#\[cfg\(target_os = "windows"\)\][\s\S]*?start_application_setup\(app_handle\)/)
  assert.match(builder, /#\[cfg\(not\(target_os = "windows"\)\)\][\s\S]*?async_runtime::block_on/)
})

test('Windows speech startup failure degrades without blocking the main application', () => {
  const app = source('src/frontend/App.vue')
  const windowsFallback = /if \(windowsPlatform\) \{\s*try \{\s*await speechRuntimeHost\.initialize\(\)\s*\}\s*catch \(error\) \{([\s\S]*?)\}\s*\}\s*else \{\s*await speechRuntimeHost\.initialize\(\)\s*\}/
  const fallbackMatch = windowsFallback.exec(app)

  assert.ok(fallbackMatch, 'speech initialization should degrade only on Windows')
  assert.match(fallbackMatch[1], /console\.warn\('语音运行时初始化失败，主界面将继续启动:', error\)/)
  assert.match(fallbackMatch[1], /onMounted:speechRuntimeDegraded/)
  assert.doesNotMatch(fallbackMatch[1], /\breturn\b/)

  const mcpShellIndex = app.indexOf('if (mcpLaunchContext.value.isMcp)')
  const mcpInitializeIndex = app.indexOf('await initializeApplication({ mcpShell: true })', mcpShellIndex)
  const mcpReturnIndex = app.indexOf('return', mcpInitializeIndex)
  const speechFallbackIndex = fallbackMatch.index
  const speechFallbackEnd = speechFallbackIndex + fallbackMatch[0].length
  const activationGateIndex = app.indexOf('const activationGateRequired = await requiresActivationGate()', speechFallbackIndex)
  const initializeApplicationIndex = app.indexOf('await initializeApplication()', activationGateIndex)
  const speechInitializeIndexes = [...app.matchAll(/await speechRuntimeHost\.initialize\(\)/g)].map(match => match.index)
  const activationGateCalls = [...app.matchAll(/await requiresActivationGate\(\)/g)]

  assert.ok(mcpShellIndex >= 0 && mcpInitializeIndex > mcpShellIndex, 'MCP shell should initialize the popup application')
  assert.ok(mcpReturnIndex > mcpInitializeIndex && mcpReturnIndex < speechFallbackIndex, 'MCP shell should return before speech startup')
  assert.doesNotMatch(app.slice(mcpShellIndex, mcpReturnIndex), /speechRuntimeHost\.initialize|requiresActivationGate/)
  assert.equal(speechInitializeIndexes.length, 2, 'startup should contain only the Windows and non-Windows speech calls')
  assert.ok(
    speechInitializeIndexes.every(index => index >= speechFallbackIndex && index < speechFallbackEnd),
    'all speech initialization calls should stay inside the platform fallback block',
  )
  assert.equal(activationGateCalls.length, 1, 'startup should perform exactly one activation gate call')
  assert.ok(activationGateIndex > speechFallbackIndex, 'activation checks should continue after degraded speech startup')
  assert.ok(initializeApplicationIndex > activationGateIndex, 'main application initialization should remain reachable')
})

test('window registry cleanup runs off the UI thread at low frequency', () => {
  const events = source('src/rust/ui/window_events.rs')
  const builder = source('src/rust/app/builder.rs')
  assert.match(events, /WINDOW_REGISTRY_CLEANUP_INTERVAL[^;]*Duration::from_secs\(5 \* 60\)/s)
  assert.match(events, /start_window_registry_cleanup_task[\s\S]*spawn_blocking[\s\S]*get_all_instances/)
  assert.match(builder, /start_window_registry_cleanup_task\(\)/)
})

test('close flow exits without recursively closing the window', () => {
  const exit = source('src/rust/ui/exit.rs')
  assert.match(exit, /exit_in_progress\.swap/)
  assert.match(exit, /#\[cfg\(target_os = "windows"\)\][\s\S]*?window\.hide\(\)/)
  assert.match(exit, /#\[cfg\(not\(target_os = "windows"\)\)\][\s\S]*?window\.close\(\)/)
})

test('frontend and package defaults preserve macOS behavior', () => {
  const app = source('src/frontend/App.vue')
  const content = source('src/frontend/components/AppContent.vue')
  const settings = source('src/frontend/composables/useSettings.ts')
  const pkg = JSON.parse(source('package.json'))
  assert.match(app, /windowsPlatform && startupStatus\.phase !== 'ready'/)
  assert.match(content, /\? 'iterate'\s*: `iterate - \$\{resolvedProjectPath\}`/)
  assert.match(settings, /if \(!windowsPlatform\)[\s\S]*?reloadAllSettings\(\)/)
  assert.equal(pkg.scripts.tauri, 'cargo tauri')
  assert.equal(pkg.scripts['tauri:build'], 'cargo tauri build')
})

test('Windows bundle uses a current-user NSIS installer', () => {
  const config = JSON.parse(source('tauri.conf.json'))
  assert.equal(config.bundle.windows.nsis.installMode, 'currentUser')
})

test('Windows package smoke waits for the GUI-subsystem activation probe and captures its output', () => {
  for (const path of [
    'scripts/windows-install-smoke.ps1',
    'scripts/windows-installer-smoke.ps1',
  ]) {
    const smoke = source(path)
    assert.match(smoke, /System\.Diagnostics\.ProcessStartInfo/, path)
    assert.match(smoke, /UseShellExecute\s*=\s*\$false/, path)
    assert.match(smoke, /RedirectStandardOutput\s*=\s*\$true/, path)
    assert.match(smoke, /RedirectStandardError\s*=\s*\$true/, path)
    assert.match(smoke, /WaitForExit\(\)/, path)
    assert.doesNotMatch(smoke, /\$ActivationProbe\s*=\s*&/, path)
  }
})
