/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import { advanceCodexLiveTranscript } from './codexLiveTranscript.ts'

test('accumulates same-role deltas and replaces them with the final transcript', () => {
  let state = { text: '', role: undefined as string | undefined, finalized: true }

  state = advanceCodexLiveTranscript(state, '我需要', 'user', false)!
  state = advanceCodexLiveTranscript(state, '转的文字', 'user', false)!
  state = advanceCodexLiveTranscript(state, '很全面', 'user', false)!
  assert.equal(state.text, '你：我需要转的文字很全面')

  state = advanceCodexLiveTranscript(state, '我需要转的文字很全面。', 'user', true)!
  assert.equal(state.text, '你：我需要转的文字很全面。')
})

test('does not join a new utterance or role onto the previous final sentence', () => {
  let state = { text: '你：上一句', role: 'user' as string | undefined, finalized: true }

  state = advanceCodexLiveTranscript(state, '下一句', 'user', false)!
  assert.equal(state.text, '你：下一句')

  state = advanceCodexLiveTranscript(state, ' 好的', 'assistant', false)!
  assert.equal(state.text, 'Codex：好的')
  state = advanceCodexLiveTranscript(state, '，我明白了', 'assistant', false)!
  assert.equal(state.text, 'Codex：好的，我明白了')
})

test('preserves realtime spaces and seals an utterance even when done has no text', () => {
  let state = { text: '', role: undefined as string | undefined, finalized: true }

  state = advanceCodexLiveTranscript(state, 'Hello ', 'user', false)!
  state = advanceCodexLiveTranscript(state, 'world', 'user', false)!
  assert.equal(state.text, '你：Hello world')

  state = advanceCodexLiveTranscript(state, '', 'user', true)!
  assert.equal(state.finalized, true)
  state = advanceCodexLiveTranscript(state, '下一句', 'user', false)!
  assert.equal(state.text, '你：下一句')
})
