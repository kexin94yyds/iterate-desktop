/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { afterEach, describe, it } from 'node:test'
import {
  DEFAULT_DESKTOP_SPEECH_RECOGNITION_MODE,
  getDesktopSpeechRecognitionMode,
  normalizeDesktopSpeechRecognitionMode,
  setDesktopSpeechRecognitionMode,
} from './desktopSpeechRecognitionMode.ts'

const originalLocalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')

function installLocalStorageMock() {
  const store = new Map<string, string>()
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem(key: string) {
        return store.get(key) ?? null
      },
      setItem(key: string, value: string) {
        store.set(key, value)
      },
    },
  })
}

afterEach(() => {
  if (originalLocalStorage) {
    Object.defineProperty(globalThis, 'localStorage', originalLocalStorage)
    return
  }
  Reflect.deleteProperty(globalThis, 'localStorage')
})

describe('desktop speech recognition mode', () => {
  it('defaults to the iOS-like quality-first Apple Speech path', () => {
    assert.equal(DEFAULT_DESKTOP_SPEECH_RECOGNITION_MODE, 'quality')
    assert.equal(normalizeDesktopSpeechRecognitionMode(undefined), 'quality')
    assert.equal(getDesktopSpeechRecognitionMode(), 'quality')
  })

  it('preserves an explicit privacy mode choice', () => {
    installLocalStorageMock()

    setDesktopSpeechRecognitionMode('privacy')

    assert.equal(getDesktopSpeechRecognitionMode(), 'privacy')
  })
})
