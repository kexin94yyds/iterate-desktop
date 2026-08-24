import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
// eslint-disable-next-line test/no-import-node-test
import test from 'node:test'
import {
  getPromptShortcutIndex,
  PROMPT_SHORTCUT_LIMIT,
} from '../src/frontend/utils/prompt-shortcut.mjs'

const popupInputSource = readFileSync(
  new URL('../src/frontend/components/popup/PopupInput.vue', import.meta.url),
  'utf8',
)

function shortcutEvent(overrides = {}) {
  return {
    altKey: true,
    code: 'Digit1',
    ctrlKey: false,
    isComposing: false,
    metaKey: false,
    repeat: false,
    shiftKey: false,
    ...overrides,
  }
}

test('maps Option+1 through Option+9 to zero-based prompt indexes', () => {
  assert.equal(PROMPT_SHORTCUT_LIMIT, 9)

  for (let number = 1; number <= PROMPT_SHORTCUT_LIMIT; number += 1) {
    assert.equal(
      getPromptShortcutIndex(shortcutEvent({ code: `Digit${number}` })),
      number - 1,
    )
  }
})

test('uses the physical digit code when Option changes the produced key', () => {
  assert.equal(
    getPromptShortcutIndex(shortcutEvent({ code: 'Digit1', key: '¡' })),
    0,
  )
})

test('rejects unsupported digits and conflicting modifier combinations', () => {
  assert.equal(getPromptShortcutIndex(shortcutEvent({ code: 'Digit0' })), -1)
  assert.equal(getPromptShortcutIndex(shortcutEvent({ altKey: false })), -1)
  assert.equal(getPromptShortcutIndex(shortcutEvent({ metaKey: true })), -1)
  assert.equal(getPromptShortcutIndex(shortcutEvent({ ctrlKey: true })), -1)
  assert.equal(getPromptShortcutIndex(shortcutEvent({ shiftKey: true })), -1)
})

test('does not trigger while composing text or repeating a held key', () => {
  assert.equal(getPromptShortcutIndex(shortcutEvent({ isComposing: true })), -1)
  assert.equal(getPromptShortcutIndex(shortcutEvent({ repeat: true })), -1)
})

test('wires the shortcut into the ordered quick-template list', () => {
  assert.match(popupInputSource, /getPromptShortcutIndex\(event\)/)
  assert.match(popupInputSource, /const prompt = sortablePrompts\.value\[index\]/)
  assert.match(popupInputSource, /handlePromptClick\(prompt\)/)
  assert.match(popupInputSource, /window\.addEventListener\('keydown', handlePromptShortcut\)/)
  assert.match(popupInputSource, /window\.removeEventListener\('keydown', handlePromptShortcut\)/)
  assert.match(popupInputSource, /window\.addEventListener\('keydown', handlePromptShortcutModifierKeydown\)/)
  assert.match(popupInputSource, /window\.addEventListener\('keyup', handlePromptShortcutModifierKeyup\)/)
  assert.match(popupInputSource, /window\.addEventListener\('blur', clearPromptShortcutModifier\)/)
  assert.match(popupInputSource, /isPromptShortcutModifierHeld && index < PROMPT_SHORTCUT_LIMIT/)
  assert.match(popupInputSource, /⌥\{\{ index \+ 1 \}\}/)
})
