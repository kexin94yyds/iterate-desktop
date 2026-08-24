<script setup lang="ts">
import { computed, ref } from 'vue'
import TrialExpiredOverlay from '../../components/common/TrialExpiredOverlay.vue'

type ScenarioKey = 'activation-input' | 'expired' | 'time-anomaly' | 'status-failed' | 'activated-day1' | 'activated-day7' | 'activated-permanent'

const currentScenario = ref<ScenarioKey>('activation-input')
const activatedCount = ref(0)

const scenarios = {
  'activation-input': {
    is_active: true,
    is_expired: false,
    days_remaining: 7,
    trial_days: 7,
    days_used: 0,
    first_launch_at: '2026-03-29T00:00:00Z',
    expires_at: '2026-04-05T00:00:00Z',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '',
    expired_subtitle: '',
    time_anomaly: false,
  },
  'expired': {
    is_active: false,
    is_expired: true,
    days_remaining: 0,
    trial_days: 7,
    days_used: 7,
    first_launch_at: '2026-03-20T00:00:00Z',
    expires_at: '2026-03-27T00:00:00Z',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '试用期已结束',
    expired_subtitle: '请前往官网购买，或输入新的激活码继续使用。',
    time_anomaly: false,
  },
  'time-anomaly': {
    is_active: false,
    is_expired: true,
    days_remaining: 0,
    trial_days: 7,
    days_used: 7,
    first_launch_at: '2026-03-20T00:00:00Z',
    expires_at: '2026-03-27T00:00:00Z',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '检测到系统时间异常',
    expired_subtitle: '请先校准系统时间，再重新输入激活码。',
    time_anomaly: true,
  },
  'status-failed': {
    is_active: false,
    is_expired: true,
    days_remaining: 0,
    trial_days: 0,
    days_used: 0,
    first_launch_at: '',
    expires_at: '',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '暂时无法读取授权状态',
    expired_subtitle: '请重启应用，或直接去官网获取激活码。',
    time_anomaly: false,
  },
  'activated-day1': {
    is_active: true,
    is_expired: false,
    days_remaining: 1,
    trial_days: 1,
    days_used: 0,
    first_launch_at: '2026-03-29T00:00:00Z',
    expires_at: '2026-03-30T00:00:00Z',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '',
    expired_subtitle: '',
    time_anomaly: false,
  },
  'activated-day7': {
    is_active: true,
    is_expired: false,
    days_remaining: 7,
    trial_days: 7,
    days_used: 0,
    first_launch_at: '2026-03-29T00:00:00Z',
    expires_at: '2026-04-05T00:00:00Z',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '',
    expired_subtitle: '',
    time_anomaly: false,
  },
  'activated-permanent': {
    is_active: true,
    is_expired: false,
    days_remaining: 0,
    trial_days: 0,
    days_used: 0,
    first_launch_at: '2026-03-29T00:00:00Z',
    expires_at: '',
    contact_url: 'https://iterate.xin/iterate/',
    expired_message: '',
    expired_subtitle: '',
    time_anomaly: false,
  },
} as const

const scenarioTitleMap: Record<ScenarioKey, string> = {
  'activation-input': '输入前',
  'expired': '标准到期',
  'time-anomaly': '时间异常',
  'status-failed': '暂时无法读取授权状态',
  'activated-day1': '激活成功 · 1 天',
  'activated-day7': '激活成功 · 7 天',
  'activated-permanent': '激活成功 · 永久',
}

const expiredBaseScenario = computed(() => scenarios.expired)
const trialStatus = computed(() => {
  if (currentScenario.value.startsWith('activated-'))
    return expiredBaseScenario.value

  return scenarios[currentScenario.value]
})
const previewActivatedStatus = computed(() => {
  if (currentScenario.value.startsWith('activated-'))
    return scenarios[currentScenario.value]

  return null
})

function switchScenario(next: ScenarioKey) {
  currentScenario.value = next
}

function handleActivated() {
  activatedCount.value += 1
}
</script>

<template>
  <div class="trial-preview-page">
    <div class="preview-toolbar">
      <p class="preview-eyebrow">
        激活页独立预览
      </p>
      <h2 class="preview-title">
        {{ scenarioTitleMap[currentScenario] }}
      </h2>
      <p class="preview-description">
        这里直接挂真实的 `TrialExpiredOverlay` 组件，不依赖本机真实授权状态。
      </p>

      <div class="preview-actions">
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'activation-input' }"
          @click="switchScenario('activation-input')"
        >
          输入前
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'expired' }"
          @click="switchScenario('expired')"
        >
          标准到期
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'time-anomaly' }"
          @click="switchScenario('time-anomaly')"
        >
          时间异常
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'status-failed' }"
          @click="switchScenario('status-failed')"
        >
          状态失败
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'activated-day1' }"
          @click="switchScenario('activated-day1')"
        >
          成功 · 1 天
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'activated-day7' }"
          @click="switchScenario('activated-day7')"
        >
          成功 · 7 天
        </button>
        <button
          class="preview-chip"
          :class="{ active: currentScenario === 'activated-permanent' }"
          @click="switchScenario('activated-permanent')"
        >
          成功 · 永久
        </button>
      </div>

      <p class="preview-note">
        “前往官网购买激活码” 会打开真实地址：`https://iterate.xin/iterate/`
      </p>
      <p class="preview-note">
        当前已模拟激活：{{ activatedCount }} 次
      </p>
    </div>

    <TrialExpiredOverlay
      :trial-status="trialStatus"
      :preview-activated-status="previewActivatedStatus"
      @activated="handleActivated"
    />
  </div>
</template>

<style scoped>
.trial-preview-page {
  min-height: 100vh;
  position: relative;
}

.preview-toolbar {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 100001;
  width: min(320px, calc(100vw - 32px));
  padding: 18px 18px 16px;
  border-radius: 18px;
  background: rgba(23, 23, 23, 0.92);
  color: #fafafa;
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.28);
  backdrop-filter: blur(12px);
}

.preview-eyebrow {
  margin: 0 0 8px;
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: rgba(250, 250, 250, 0.58);
}

.preview-title {
  margin: 0;
  font-size: 20px;
  line-height: 1.2;
}

.preview-description,
.preview-note {
  margin: 10px 0 0;
  font-size: 12px;
  line-height: 1.55;
  color: rgba(250, 250, 250, 0.78);
}

.preview-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.preview-chip {
  border: 1px solid rgba(250, 250, 250, 0.18);
  background: rgba(255, 255, 255, 0.06);
  color: #fafafa;
  padding: 8px 10px;
  border-radius: 999px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.preview-chip.active {
  background: #fafafa;
  color: #171717;
  border-color: #fafafa;
}

.preview-chip:hover {
  border-color: rgba(250, 250, 250, 0.42);
}
</style>
