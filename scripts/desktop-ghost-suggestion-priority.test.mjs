/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const [commands, bridge] = await Promise.all([
  readFile(new URL('../src/rust/ui/commands.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src/rust/bridge/ws.rs', import.meta.url), 'utf8'),
])

test('desktop ghost priority writes broadcast the authoritative store', () => {
  assert.match(
    commands,
    /save_ghost_suggestions_file[\s\S]*save_store_from_content\(content\)\?[\s\S]*broadcast_ghost_suggestions_changed\(&app, store\)/,
  )
  assert.match(bridge, /message_type: "ghost_suggestions_changed"\.to_string\(\)/)
  assert.match(bridge, /"ghostSuggestions": ghost_suggestions/)
})
