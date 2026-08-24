import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const activeSessionSource = readFileSync('src/rust/bridge/active_session.rs', 'utf8')
const bridgeSource = readFileSync('src/rust/bridge/ws.rs', 'utf8')
const windowEventSource = readFileSync('src/rust/ui/window_events.rs', 'utf8')
const windowRegistrySource = readFileSync('src/rust/ui/window_registry.rs', 'utf8')

test('desktop window registry persists focus timestamps independently by pid', () => {
  assert.match(windowRegistrySource, /last_focused_at_by_pid:\s*HashMap<u32, String>/)
  assert.match(windowRegistrySource, /pub fn mark_current_window_focused\(&mut self\)/)
  assert.match(windowRegistrySource, /last_focused_at_by_pid\.insert\(pid, focused_at\)/)
  assert.match(windowRegistrySource, /last_focused_at_by_pid\s*\.retain\(/)
  assert.match(windowRegistrySource, /#\[serde\(default[^\]]*\)\][\s\S]*last_focused_at_by_pid/)
})

test('focused window events update the shared registry instead of only process memory', () => {
  const focusHandler = windowEventSource.match(
    /if let WindowEvent::Focused\(true\) = event \{([\s\S]*?)\n\s*\}/,
  )?.[1] ?? ''

  assert.match(focusHandler, /set_last_focused_window\(&label_clone\)/)
  assert.match(focusHandler, /WindowRegistry::load\(\)/)
  assert.match(focusHandler, /mark_current_window_focused\(\)/)
})

test('active sessions expose desktop focus time and ignore repeated mcp state touches', () => {
  assert.match(activeSessionSource, /build_active_session_summaries_with_focus/)
  assert.match(activeSessionSource, /last_focused_at_by_pid\s*\.get\(&instance\.pid\)/)
  assert.match(activeSessionSource, /unwrap_or\(&instance\.registered_at\)/)
  assert.doesNotMatch(
    activeSessionSource,
    /let last_active_at = entry[\s\S]*entry\.last_active_at\.clone\(\)/,
  )
  assert.match(bridgeSource, /build_active_session_summaries_with_focus\(/)
  assert.match(bridgeSource, /window_registry\.last_focused_at_by_pid\(\)/)
})
