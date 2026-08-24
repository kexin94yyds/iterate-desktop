<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import SpeechWavePill from '../../components/speech/SpeechWavePill.vue'

type PreviewMode = 'idle' | 'preparing' | 'active' | 'muted' | 'reconnecting'
type SurfaceMode = 'light' | 'dark'

const modes: Array<{ key: PreviewMode, label: string }> = [
  { key: 'idle', label: '收起' },
  { key: 'preparing', label: '准备中' },
  { key: 'active', label: '实时对话' },
  { key: 'muted', label: '已静音' },
  { key: 'reconnecting', label: '重连中' },
]
const transcriptSteps = [
  '我想让 GPT-Live 不依赖 iterate 的独立弹窗，',
  '我想让 GPT-Live 不依赖 iterate 的独立弹窗，它应该始终作为全局主代理运行。',
  '我想让 GPT-Live 不依赖 iterate 的独立弹窗，它应该始终作为全局主代理运行。长按 Fn 五秒启动，再次长按五秒结束。',
  '我想让 GPT-Live 不依赖 iterate 的独立弹窗，它应该始终作为全局主代理运行。长按 Fn 五秒启动，短按 Fn 取消当前对话但保持 Live。',
]

const currentMode = ref<PreviewMode>('active')
const surfaceMode = ref<SurfaceMode>('light')
const manuallyCollapsed = ref(false)
const transcriptIndex = ref(transcriptSteps.length - 1)
const autoPlaying = ref(false)
let autoTimer: number | null = null

const liveVisible = computed(() => currentMode.value !== 'idle')
const expanded = computed(() => liveVisible.value && !manuallyCollapsed.value)
const muted = computed(() => currentMode.value === 'muted')
const phaseLabel = computed(() => ({
  idle: '全局主代理待机',
  preparing: '正在准备麦克风',
  active: '正在聆听',
  muted: '已静音',
  reconnecting: '正在重新连接',
}[currentMode.value]))
const displayText = computed(() => {
  if (currentMode.value === 'preparing')
    return '正在建立 GPT-Live 实时语音连接…'
  if (currentMode.value === 'reconnecting')
    return '网络短暂中断，正在自动恢复实时会话，已有文字不会消失。'
  return transcriptSteps[transcriptIndex.value]
})

function stopAutoPlay() {
  autoPlaying.value = false
  if (autoTimer !== null)
    window.clearInterval(autoTimer)
  autoTimer = null
}

function toggleAutoPlay() {
  if (autoPlaying.value) {
    stopAutoPlay()
    return
  }
  currentMode.value = 'active'
  manuallyCollapsed.value = false
  transcriptIndex.value = 0
  autoPlaying.value = true
  autoTimer = window.setInterval(() => {
    if (transcriptIndex.value >= transcriptSteps.length - 1) {
      stopAutoPlay()
      return
    }
    transcriptIndex.value += 1
  }, 850)
}

onBeforeUnmount(stopAutoPlay)
</script>

<template>
  <div class="speech-test">
    <div class="section-heading">
      <div>
        <h3 class="section-title">
          GPT-Live 全局实时文字浮层
        </h3>
        <p class="section-desc">
          启动后从底部胶囊原位向上展开；文字实时追加，停止后收回，不再出现顶部副本。
        </p>
      </div>
      <span class="implementation-badge">实际组件</span>
    </div>

    <div class="preview-stage">
      <div class="desktop-sim" :class="`desktop-sim--${surfaceMode}`">
        <div class="desktop-sim__hint">
          桌面底部
        </div>
        <div
          class="live-preview"
          :class="[
            `live-preview--${currentMode}`,
            {
              'live-preview--expanded': expanded,
              'live-preview--collapsed': liveVisible && manuallyCollapsed,
            },
          ]"
        >
          <SpeechWavePill
            :active="liveVisible && !muted"
            :tone="currentMode === 'reconnecting' ? 'error' : (liveVisible ? 'active' : 'idle')"
          />

          <template v-if="expanded">
            <section class="live-preview__panel">
              <section class="live-preview__copy">
                <div class="live-preview__eyebrow">
                  <span class="live-preview__dot" />
                  <span>{{ phaseLabel }}</span>
                  <span class="live-preview__separator">·</span>
                  <span class="live-preview__project">iterate</span>
                </div>
                <p class="live-preview__status">
                  {{ displayText }}
                </p>
              </section>

              <button
                class="live-preview__mute"
                type="button"
                :class="{ 'live-preview__mute--active': muted }"
                :title="muted ? '恢复麦克风' : '静音麦克风'"
                @click="currentMode = muted ? 'active' : 'muted'"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 15.25a3.25 3.25 0 0 0 3.25-3.25V6a3.25 3.25 0 0 0-6.5 0v6A3.25 3.25 0 0 0 12 15.25Z" />
                  <path d="M5.75 11.5v.5a6.25 6.25 0 0 0 12.5 0v-.5M12 18.25V22M8.5 22h7" />
                  <path v-if="muted" d="m4 4 16 16" />
                </svg>
              </button>

              <button
                class="live-preview__fold"
                type="button"
                title="隐藏实时文字"
                @click="manuallyCollapsed = true"
              >
                <svg viewBox="0 0 16 24" aria-hidden="true">
                  <path d="m10 7-5 5 5 5" />
                </svg>
              </button>
            </section>
          </template>

          <button
            v-else-if="liveVisible && manuallyCollapsed"
            class="live-preview__fold live-preview__fold--collapsed"
            type="button"
            title="展开实时文字"
            @click="manuallyCollapsed = false"
          >
            <svg viewBox="0 0 16 24" aria-hidden="true">
              <path d="m6 7 5 5-5 5" />
            </svg>
          </button>
        </div>
      </div>
    </div>

    <div class="controls">
      <button
        class="phase-btn"
        :class="{ selected: surfaceMode === 'light' }"
        @click="surfaceMode = 'light'"
      >
        白色网页
      </button>
      <button
        class="phase-btn"
        :class="{ selected: surfaceMode === 'dark' }"
        @click="surfaceMode = 'dark'"
      >
        深色桌面
      </button>
      <span class="controls__divider" />
      <button
        v-for="mode in modes"
        :key="mode.key"
        class="phase-btn"
        :class="{ selected: currentMode === mode.key }"
        @click="currentMode = mode.key"
      >
        {{ mode.label }}
      </button>
      <button class="phase-btn auto-btn" :class="{ selected: autoPlaying }" @click="toggleAutoPlay">
        {{ autoPlaying ? '停止演示' : '演示实时追加' }}
      </button>
    </div>

    <div class="behavior-note">
      <span class="behavior-note__key">长按 Fn 5 秒</span>
      <span>幂等启动 GPT-Live</span>
      <span class="behavior-note__divider" />
      <span class="behavior-note__key">短按 Fn 或点胶囊</span>
      <span>静音 / 恢复</span>
    </div>
  </div>
</template>

<style scoped>
.speech-test {
  max-width: 760px;
  margin: 0 auto;
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.section-title {
  margin: 0 0 5px;
  color: var(--text-color, #222);
  font-size: 1.15rem;
  font-weight: 650;
}

.section-desc {
  max-width: 580px;
  margin: 0 0 18px;
  color: var(--text-color-3, #777);
  font-size: 0.85rem;
  line-height: 1.55;
}

.implementation-badge {
  flex: 0 0 auto;
  padding: 4px 9px;
  border: 1px solid rgba(69, 122, 255, 0.28);
  border-radius: 999px;
  color: #457aff;
  background: rgba(69, 122, 255, 0.08);
  font-size: 11px;
  font-weight: 650;
}

.preview-stage {
  margin-bottom: 18px;
}

.desktop-sim {
  position: relative;
  display: flex;
  height: 330px;
  align-items: flex-end;
  justify-content: center;
  overflow: hidden;
  padding: 0 24px 24px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 18px;
  background:
    radial-gradient(circle at 24% 18%, rgba(82, 112, 172, 0.22), transparent 32%),
    radial-gradient(circle at 78% 60%, rgba(74, 54, 107, 0.18), transparent 34%),
    linear-gradient(155deg, #20232c, #0d0f14 70%);
}

.desktop-sim--light {
  border-color: rgba(22, 28, 40, 0.12);
  background:
    radial-gradient(circle at 18% 24%, rgba(91, 139, 227, 0.08), transparent 30%),
    linear-gradient(90deg, transparent 0 56%, rgba(22, 28, 40, 0.08) 56% calc(56% + 1px), transparent calc(56% + 1px)),
    linear-gradient(180deg, #fff, #f4f6f9);
}

.desktop-sim--light::after {
  background: rgba(22, 28, 40, 0.1);
}

.desktop-sim--light .desktop-sim__hint {
  color: rgba(22, 28, 40, 0.34);
}

.desktop-sim::after {
  position: absolute;
  right: 9%;
  bottom: 8px;
  left: 9%;
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  content: '';
}

.desktop-sim__hint {
  position: absolute;
  top: 18px;
  left: 20px;
  color: rgba(255, 255, 255, 0.28);
  font-size: 11px;
  letter-spacing: 0.12em;
}

.live-preview {
  --accent: #9aa4b5;
  position: relative;
  z-index: 1;
  display: flex;
  width: 96px;
  height: 48px;
  align-items: center;
  justify-content: center;
  color: #f6f8fc;
  transform-origin: right center;
  transition: width 220ms ease, height 220ms ease, border-radius 220ms ease, transform 220ms ease;
}

.live-preview--expanded {
  display: grid;
  width: min(520px, calc(100% - 24px));
  height: 156px;
  padding: 10px 13px 10px 11px;
  grid-template-columns: 58px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  background: transparent;
  transform: translateX(calc((126px - 100%) / 2));
}

.live-preview--collapsed {
  width: 126px;
  gap: 7px;
  transform: translateX(0);
}

.live-preview--preparing,
.live-preview--active,
.live-preview--muted {
  --accent: #78a8ff;
}

.live-preview--reconnecting {
  --accent: #ff8a7a;
}

.live-preview__panel {
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
  -webkit-backdrop-filter: blur(14px) saturate(1.08);
}

.live-preview__copy {
  min-width: 0;
}

.live-preview__eyebrow {
  display: flex;
  min-width: 0;
  align-items: center;
  margin-bottom: 7px;
  color: rgba(225, 231, 242, 0.66);
  font: 600 10px/1.2 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  letter-spacing: 0.055em;
}

.live-preview__dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  margin-right: 6px;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 17%, transparent);
}

.live-preview__separator {
  margin: 0 6px;
  opacity: 0.55;
}

.live-preview__project {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.live-preview__status {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: #f6f8fc;
  font: 600 14px/1.45 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  overflow-wrap: anywhere;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 5;
}

.live-preview__mute,
.live-preview__fold {
  display: grid;
  height: 34px;
  place-items: center;
  padding: 0;
  border: 0;
  color: rgba(225, 231, 242, 0.72);
  cursor: pointer;
}

.live-preview__mute {
  width: 34px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.08);
}

.live-preview__mute:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.13);
}

.live-preview__mute--active {
  color: #ff7a7a;
  background: rgba(255, 92, 92, 0.14);
}

.live-preview__fold {
  width: 22px;
  border-left: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0 9px 9px 0;
  background: transparent;
}

.live-preview__fold:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
}

.live-preview__fold--collapsed {
  flex: 0 0 auto;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0 11px 11px 0;
  background: linear-gradient(135deg, rgba(29, 33, 43, 0.86), rgba(13, 16, 22, 0.74));
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.14);
}

.live-preview__mute svg,
.live-preview__fold svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.8;
}

.live-preview__fold svg {
  width: 12px;
  height: 18px;
}

.controls {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 18px;
}

.controls__divider {
  width: 1px;
  margin: 3px 3px;
  background: var(--border-color, #ddd);
}

.phase-btn {
  padding: 7px 13px;
  border: 1px solid var(--border-color, #ddd);
  border-radius: 9px;
  color: var(--text-color, #333);
  background: var(--card-color, #fff);
  cursor: pointer;
  font-size: 13px;
  transition: border-color 140ms ease, background 140ms ease, color 140ms ease;
}

.phase-btn:hover {
  border-color: var(--primary-color, #4098fc);
}

.phase-btn.selected {
  border-color: var(--primary-color, #4098fc);
  color: #fff;
  background: var(--primary-color, #4098fc);
}

.auto-btn {
  margin-left: auto;
}

.behavior-note {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 7px;
  padding: 12px 14px;
  border: 1px solid var(--border-color, #e5e5e5);
  border-radius: 12px;
  color: var(--text-color-3, #777);
  background: var(--card-color, #fff);
  font-size: 12px;
}

.behavior-note__key {
  color: var(--text-color, #333);
  font-weight: 650;
}

.behavior-note__divider {
  width: 1px;
  height: 14px;
  margin: 0 5px;
  background: var(--border-color, #ddd);
}

@media (prefers-reduced-motion: reduce) {
  .live-preview,
  .phase-btn {
    transition: none;
  }
}
</style>
