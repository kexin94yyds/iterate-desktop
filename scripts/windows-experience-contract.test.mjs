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

test('bridge health probe uses reqwest and does not spawn curl', () => {
  const setup = source('src/rust/app/setup.rs')
  assert.doesNotMatch(setup, /command_stdout\("curl"/)
  assert.match(setup, /reqwest::blocking::Client::builder/)
})

test('main window is shown before background setup starts', () => {
  const builder = source('src/rust/app/builder.rs')
  assert.doesNotMatch(builder, /async_runtime::block_on/)
  const showIndex = builder.indexOf('window.show()')
  const setupIndex = builder.lastIndexOf('start_application_setup(app_handle)')
  assert.ok(showIndex >= 0 && setupIndex > showIndex)
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
  assert.doesNotMatch(exit, /window\.close\(\)/)
  assert.match(exit, /window\.hide\(\)/)
})

test('Windows bundle uses a current-user NSIS installer', () => {
  const config = JSON.parse(source('tauri.conf.json'))
  assert.equal(config.bundle.windows.nsis.installMode, 'currentUser')
})
