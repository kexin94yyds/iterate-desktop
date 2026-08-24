<script setup lang="ts">
import type { GlobalCodexLivePhase } from '../../services/desktopCodexLiveControl'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useDesktopCodexLiveHost } from '../../composables/useDesktopCodexLiveHost'
import { useDesktopCodexLiveHudLayout } from '../../composables/useDesktopCodexLiveHudLayout'
import { useGlobalSpeechInput } from '../../composables/useGlobalSpeechInput'
import { resolveCodexLiveHudText } from '../../utils/codexLiveHudText'
import SpeechWavePill from './SpeechWavePill.vue'

const SPEECH_OVERLAY_DOCUMENT_CLASS = 'iterate-speech-overlay-mode'
const SPEECH_OVERLAY_POSITION_KEY = 'iterate.speech-overlay.live-hud-center.v1'
const DRAG_THRESHOLD_PX = 7
const speech = useGlobalSpeechInput()
const liveHost = useDesktopCodexLiveHost()
const live = liveHost.live
const overlayWindow = getCurrentWindow()
const hudLayout = useDesktopCodexLiveHudLayout(SPEECH_OVERLAY_POSITION_KEY)
const manuallyCollapsed = ref(false)

let layoutReady = false
let activePointerId: number | null = null
let pointerTarget: HTMLElement | null = null
let pointerStart = { x: 0, y: 0 }
let pointerMoved = false

const isSpeechActive = computed(() =>
  speech.phase.value === 'starting'
  || speech.phase.value === 'listening'
  || speech.phase.value === 'stopping'
  || speech.phase.value === 'processing',
)

function isLivePhaseActive(phase?: GlobalCodexLivePhase) {
  return phase === 'preparing'
    || phase === 'connecting'
    || phase === 'active'
    || phase === 'reconnecting'
}

const livePhase = computed(() => live.phase.value)
const executionPhase = computed(() => live.executionPhase.value)
const liveActive = computed(() => isLivePhaseActive(livePhase.value))
const expanded = computed(() => liveActive.value && !manuallyCollapsed.value)
const collapsed = computed(() => liveActive.value && manuallyCollapsed.value)
const layoutMode = computed(() => {
  if (expanded.value)
    return 'expanded' as const
  if (collapsed.value)
    return 'collapsed' as const
  return 'compact' as const
})
const microphoneMuted = computed(() => live.isMicrophoneMuted.value)
const pillActive = computed(() =>
  isSpeechActive.value
  || (liveActive.value && !microphoneMuted.value),
)
const displayText = computed(() => resolveCodexLiveHudText({
  executionPhase: executionPhase.value,
  statusText: live.statusText.value,
  taskProgressText: live.taskProgressText.value,
  latestTranscript: live.latestTranscript.value,
  fallbackStatusText: live.statusText.value,
}))
const phaseLabel = computed(() => {
  if (executionPhase.value === 'submitting')
    return '正在提交'
  if (executionPhase.value === 'running')
    return '正在执行'
  if (executionPhase.value === 'completed')
    return '执行完成'
  if (executionPhase.value === 'failed')
    return '执行异常'
  return ({
    idle: '全局主代理待机',
    preparing: '正在准备麦克风',
    connecting: '正在连接',
    active: microphoneMuted.value ? '已静音' : '正在聆听',
    reconnecting: '正在重新连接',
    failed: '需要处理',
  }[livePhase.value ?? 'idle'])
})
const brandLabel = 'iterate'
const toneClass = computed(() => {
  if (speech.errorMessage.value.trim() || speech.phase.value === 'error')
    return 'error' as const
  if (livePhase.value === 'failed')
    return 'error' as const
  if (liveActive.value)
    return 'active' as const
  if (speech.phase.value === 'success')
    return 'success' as const
  if (isSpeechActive.value)
    return 'active' as const
  return 'idle' as const
})

function syncHudLayout(nextMode: 'compact' | 'collapsed' | 'expanded') {
  if (layoutReady)
    void hudLayout.setMode(nextMode)
}

function releasePointerCapture() {
  if (activePointerId === null || !pointerTarget)
    return
  try {
    if (pointerTarget.hasPointerCapture?.(activePointerId))
      pointerTarget.releasePointerCapture(activePointerId)
  }
  catch {
    // Native dragging may already have released the DOM capture.
  }
  pointerTarget = null
}

function handlePointerDown(event: PointerEvent) {
  if (event.pointerType === 'mouse' && event.button !== 0)
    return
  event.preventDefault()
  activePointerId = event.pointerId
  pointerTarget = event.currentTarget as HTMLElement
  pointerStart = { x: event.clientX, y: event.clientY }
  pointerMoved = false
  pointerTarget.setPointerCapture?.(event.pointerId)
}

function handlePointerMove(event: PointerEvent) {
  if (activePointerId !== event.pointerId || pointerMoved)
    return
  const distance = Math.hypot(
    event.clientX - pointerStart.x,
    event.clientY - pointerStart.y,
  )
  if (distance <= DRAG_THRESHOLD_PX)
    return
  pointerMoved = true
  releasePointerCapture()
  activePointerId = null
  void overlayWindow.startDragging().catch(() => undefined)
}

function requestMicrophoneMuteToggle() {
  if (!liveActive.value)
    return
  void liveHost.requestMicrophoneMuteToggle()
}

function handlePointerEnd(event: PointerEvent) {
  if (activePointerId !== event.pointerId)
    return
  const shouldToggleMute = event.type === 'pointerup'
    && !pointerMoved
    && liveActive.value
  releasePointerCapture()
  activePointerId = null
  if (shouldToggleMute)
    requestMicrophoneMuteToggle()
}

function handleFoldPointerEnd(event: PointerEvent, nextCollapsed: boolean) {
  if (activePointerId !== event.pointerId)
    return
  const shouldToggle = event.type === 'pointerup' && !pointerMoved
  releasePointerCapture()
  activePointerId = null
  if (shouldToggle)
    manuallyCollapsed.value = nextCollapsed
}

function setSpeechOverlayDocumentMode(enabled: boolean) {
  if (typeof document === 'undefined')
    return

  document.documentElement.classList.toggle(SPEECH_OVERLAY_DOCUMENT_CLASS, enabled)
  document.body.classList.toggle(SPEECH_OVERLAY_DOCUMENT_CLASS, enabled)
}

watch(layoutMode, value => syncHudLayout(value))
watch(liveActive, (active) => {
  if (!active)
    manuallyCollapsed.value = false
})

onMounted(async () => {
  setSpeechOverlayDocumentMode(true)
  hudLayout.initialize(layoutMode.value)
  layoutReady = true
  await Promise.all([
    speech.initialize(),
    liveHost.initialize(),
  ])
})

onBeforeUnmount(() => {
  releasePointerCapture()
  activePointerId = null
  hudLayout.dispose()
  speech.dispose()
  void liveHost.dispose()
  setSpeechOverlayDocumentMode(false)
})
</script>

<template>
  <main
    class="live-overlay"
    :class="[
      `live-overlay--${livePhase ?? speech.phase.value}`,
      {
        'live-overlay--expanded': expanded,
        'live-overlay--collapsed': collapsed,
      },
    ]"
    aria-live="polite"
    data-tauri-drag-region
  >
    <button
      class="anchor-control"
      type="button"
      :aria-label="liveActive ? (microphoneMuted ? '恢复 GPT-Live 麦克风' : '静音 GPT-Live 麦克风') : 'Fn 语音状态'"
      :aria-pressed="liveActive && microphoneMuted"
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerEnd"
      @pointercancel="handlePointerEnd"
      @contextmenu.prevent
    >
      <SpeechWavePill :active="pillActive" :tone="toneClass" />
    </button>

    <template v-if="expanded">
      <section class="live-overlay__panel" data-tauri-drag-region>
        <section class="live-overlay__copy" data-tauri-drag-region>
          <div class="live-overlay__eyebrow" data-tauri-drag-region>
            <span class="live-overlay__dot" aria-hidden="true" />
            <span data-tauri-drag-region>{{ phaseLabel }}</span>
            <span class="live-overlay__separator" aria-hidden="true">·</span>
            <span class="live-overlay__project" data-tauri-drag-region>{{ brandLabel }}</span>
          </div>
          <p class="live-overlay__status" :title="displayText" data-tauri-drag-region>
            {{ displayText }}
          </p>
        </section>

        <button
          class="live-overlay__mute"
          type="button"
          :class="{ 'live-overlay__mute--active': microphoneMuted }"
          :aria-label="microphoneMuted ? '恢复麦克风' : '静音麦克风'"
          :title="microphoneMuted ? '恢复麦克风' : '静音麦克风'"
          @click.stop="requestMicrophoneMuteToggle"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 15.25a3.25 3.25 0 0 0 3.25-3.25V6a3.25 3.25 0 0 0-6.5 0v6A3.25 3.25 0 0 0 12 15.25Z" />
            <path d="M5.75 11.5v.5a6.25 6.25 0 0 0 12.5 0v-.5M12 18.25V22M8.5 22h7" />
            <path v-if="microphoneMuted" d="m4 4 16 16" />
          </svg>
        </button>

        <button
          class="live-overlay__fold"
          type="button"
          aria-label="隐藏实时文字"
          title="隐藏实时文字"
          @pointerdown.stop="handlePointerDown"
          @pointermove.stop="handlePointerMove"
          @pointerup.stop="handleFoldPointerEnd($event, true)"
          @pointercancel.stop="handleFoldPointerEnd($event, true)"
          @contextmenu.prevent
        >
          <svg viewBox="0 0 16 24" aria-hidden="true">
            <path d="m10 7-5 5 5 5" />
          </svg>
        </button>
      </section>
    </template>

    <button
      v-else-if="collapsed"
      class="live-overlay__fold live-overlay__fold--collapsed"
      type="button"
      aria-label="展开实时文字"
      title="展开实时文字"
      @pointerdown.stop="handlePointerDown"
      @pointermove.stop="handlePointerMove"
      @pointerup.stop="handleFoldPointerEnd($event, false)"
      @pointercancel.stop="handleFoldPointerEnd($event, false)"
      @contextmenu.prevent
    >
      <svg viewBox="0 0 16 24" aria-hidden="true">
        <path d="m6 7 5 5-5 5" />
      </svg>
    </button>
  </main>
</template>

<style scoped>
:global(html.iterate-speech-overlay-mode),
:global(body.iterate-speech-overlay-mode),
:global(.iterate-speech-overlay-mode #app) {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
  background: transparent !important;
  font-family: "SF Pro Display", "PingFang SC", sans-serif;
}

button {
  font: inherit;
}

.live-overlay {
  --accent: #9aa4b5;
  box-sizing: border-box;
  display: flex;
  width: 100%;
  height: 100%;
  align-items: center;
  justify-content: center;
  color: #f6f8fc;
  user-select: none;
}

.live-overlay--expanded {
  display: grid;
  width: calc(100% - 8px);
  height: calc(100% - 8px);
  margin: 4px;
  padding: 9px 12px 9px 10px;
  grid-template-columns: 58px minmax(0, 1fr);
  align-items: center;
  gap: 11px;
  background: transparent;
  cursor: grab;
}

.live-overlay--collapsed {
  gap: 7px;
}

.live-overlay--expanded:active {
  cursor: grabbing;
}

.live-overlay--active,
.live-overlay--preparing,
.live-overlay--connecting,
.live-overlay--reconnecting {
  --accent: #78a8ff;
}

.live-overlay--failed,
.live-overlay--error {
  --accent: #ff7a7a;
}

.anchor-control,
.live-overlay__mute,
.live-overlay__fold {
  display: grid;
  place-items: center;
  padding: 0;
  border: 0;
  outline: 0;
}

.anchor-control {
  width: 100%;
  height: 100%;
  cursor: grab;
  touch-action: none;
  background: transparent;
}

.live-overlay--expanded .anchor-control,
.live-overlay--collapsed .anchor-control {
  width: 58px;
  height: 34px;
}

.anchor-control:active {
  cursor: grabbing;
}

.anchor-control:focus-visible,
.live-overlay__mute:focus-visible,
.live-overlay__fold:focus-visible {
  border-radius: 18px;
  box-shadow: 0 0 0 3px rgba(120, 168, 255, 0.34);
}

.live-overlay__panel {
  display: grid;
  min-width: 0;
  padding: 8px 10px 9px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 13px;
  background: linear-gradient(135deg, rgba(29, 33, 43, 0.86), rgba(13, 16, 22, 0.74));
  box-shadow: 0 8px 22px rgba(0, 0, 0, 0.16), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  grid-template-columns: minmax(0, 1fr) 34px 22px;
  align-items: center;
  gap: 7px;
  backdrop-filter: blur(14px) saturate(1.08);
  cursor: grab;
  -webkit-backdrop-filter: blur(14px) saturate(1.08);
}

.live-overlay__panel:active {
  cursor: grabbing;
}

.live-overlay__copy {
  min-width: 0;
}

.live-overlay__eyebrow {
  display: flex;
  min-width: 0;
  align-items: center;
  margin-bottom: 4px;
  color: rgba(225, 231, 242, 0.68);
  font: 600 10px/1.2 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  letter-spacing: 0.055em;
}

.live-overlay__dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  margin-right: 6px;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 17%, transparent);
}

.live-overlay__separator {
  margin: 0 6px;
  opacity: 0.55;
}

.live-overlay__project {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.live-overlay__status {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: #f6f8fc;
  font: 600 13px/1.35 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  overflow-wrap: anywhere;
  white-space: normal;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 6;
}

.live-overlay__mute {
  width: 34px;
  height: 34px;
  color: rgba(225, 231, 242, 0.72);
  cursor: pointer;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.08);
  transform-origin: center;
  transition: transform 110ms ease-out, background 110ms ease-out, color 110ms ease-out;
}

.live-overlay__fold {
  width: 22px;
  height: 34px;
  color: rgba(225, 231, 242, 0.72);
  cursor: grab;
  border-left: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0 9px 9px 0;
  background: transparent;
  touch-action: none;
  transition: transform 140ms ease, background 140ms ease, color 140ms ease;
}

.live-overlay__fold:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
}

.live-overlay__fold:active {
  cursor: grabbing;
  color: #fff;
  background: rgba(255, 255, 255, 0.14);
  transform: scale(0.9);
}

.live-overlay__fold--collapsed {
  flex: 0 0 auto;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0 11px 11px 0;
  background: linear-gradient(135deg, rgba(29, 33, 43, 0.86), rgba(13, 16, 22, 0.74));
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.14);
}

.live-overlay__mute:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.13);
}

.live-overlay__mute:active {
  transform: scale(0.96);
}

.live-overlay__mute--active {
  color: #ff7a7a;
  background: rgba(255, 92, 92, 0.14);
}

.live-overlay__mute svg,
.live-overlay__fold svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.8;
}

.live-overlay__fold svg {
  width: 12px;
  height: 18px;
}

@media (prefers-reduced-motion: reduce) {
  .live-overlay__mute,
  .live-overlay__fold {
    transition: none;
  }
}
</style>
