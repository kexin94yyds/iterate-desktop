import type { SpeechSnapshot } from '../services/globalSpeechSession'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, shallowRef } from 'vue'
import {
  deriveSpeechRenderPhase,
  GlobalSpeechSessionGuard,
} from '../services/globalSpeechSession'

export function useGlobalSpeechInput() {
  const guard = new GlobalSpeechSessionGuard()
  const currentSnapshot = shallowRef<SpeechSnapshot | null>(null)
  const phase = computed(() => currentSnapshot.value ? deriveSpeechRenderPhase(currentSnapshot.value) : 'idle')
  const partialText = computed(() => '')
  const finalText = computed(() => '')
  const statusMessage = computed(() => currentSnapshot.value?.phase.toLowerCase() ?? 'idle')
  const errorMessage = computed(() => phase.value === 'error' ? '语音写入失败' : '')
  let unlistenSnapshot: (() => void) | null = null

  async function acknowledgeAppliedSnapshot(snapshot: SpeechSnapshot) {
    if (!snapshot.identity)
      return
    await new Promise<void>((resolve) => {
      window.requestAnimationFrame(() => resolve())
    })
    if (!guard.isCurrent(snapshot.identity))
      return
    await invoke('ack_speech_overlay_visibility', {
      identity: snapshot.identity,
      visible: snapshot.visible,
    }).catch(() => undefined)
  }

  function project(snapshot: SpeechSnapshot) {
    if (!guard.applySnapshot(snapshot))
      return
    currentSnapshot.value = snapshot
    void acknowledgeAppliedSnapshot(snapshot)
  }

  async function initialize() {
    unlistenSnapshot = await listen<SpeechSnapshot>('speech://session-snapshot', event => project(event.payload))
    const snapshot = await invoke<SpeechSnapshot>('get_speech_control_snapshot')
    project(snapshot)
  }

  function dispose() {
    unlistenSnapshot?.()
    unlistenSnapshot = null
  }

  return {
    phase,
    partialText,
    finalText,
    statusMessage,
    errorMessage,
    initialize,
    dispose,
  }
}
