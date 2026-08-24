<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import iterateIconUrl from '../../../../icons/icon-128.png'
import { SETUP_PROMPT_CONTENT } from '../../constants/setupPrompt'

interface TrialStatus {
  is_active: boolean
  is_expired: boolean
  days_remaining: number
  trial_days: number
  days_used: number
  first_launch_at: string
  expires_at: string
  contact_url: string
  expired_message: string
  expired_subtitle: string
  time_anomaly: boolean
}

const props = withDefaults(defineProps<{
  trialStatus: TrialStatus
  previewActivatedStatus?: TrialStatus | null
}>(), {
  previewActivatedStatus: null,
})

const emit = defineEmits<{
  (e: 'activated'): void
}>()

const licenseKey = ref('')
const activating = ref(false)
const errorMsg = ref('')
const activated = ref(false)
const activatedTitle = ref('')
const activatedDescription = ref('')
const setupCopyText = ref('复制安装提示词')
const inputRef = ref<HTMLInputElement | null>(null)
const isStandardActivationState = computed(() => {
  return props.trialStatus.is_expired && !props.trialStatus.time_anomaly
})
const overlayTitle = computed(() => {
  if (isStandardActivationState.value)
    return '欢迎使用 iterate'

  return props.trialStatus.expired_message || '输入激活码开始使用'
})
const overlaySubtitle = computed(() => {
  if (isStandardActivationState.value)
    return '购买永久版后输入激活码；优惠码「无限迭代」可五折。'

  return props.trialStatus.expired_subtitle || '输入激活码继续使用。没有激活码就去官网获取。'
})
const contactUrl = computed(() => props.trialStatus.contact_url?.trim() || 'https://iterate.xin/iterate/')

function resolveActivatedMessage(status: TrialStatus) {
  if (!status.is_active) {
    return {
      title: '激活成功',
      description: '授权状态已更新，正在进入应用。',
    }
  }

  if (status.trial_days === 1) {
    return {
      title: '已激活：1 天版',
      description: `当前授权有效 1 天${status.days_remaining > 0 ? `，剩余 ${status.days_remaining} 天` : ''}。`,
    }
  }

  if (status.trial_days === 7) {
    return {
      title: '已激活：7 天版',
      description: `当前授权有效 7 天${status.days_remaining > 0 ? `，剩余 ${status.days_remaining} 天` : ''}。`,
    }
  }

  return {
    title: '已激活：永久版',
    description: '当前授权为永久版，可以继续长期使用。',
  }
}

async function handleActivate() {
  const key = licenseKey.value.replace(/\s+/g, '').trim()
  if (!key) {
    errorMsg.value = '请输入激活码'
    return
  }
  licenseKey.value = key
  activating.value = true
  errorMsg.value = ''
  try {
    await invoke('activate_license', { key })
    try {
      const status = await invoke<TrialStatus>('get_trial_status')
      const message = resolveActivatedMessage(status)
      activatedTitle.value = message.title
      activatedDescription.value = message.description
    }
    catch {
      activatedTitle.value = '激活成功'
      activatedDescription.value = '授权状态已更新，正在进入应用。'
    }
    activated.value = true
  }
  catch (e: any) {
    errorMsg.value = typeof e === 'string' ? e : '激活失败，请检查激活码'
  }
  finally {
    activating.value = false
  }
}

function openContactUrl(url: string) {
  invoke('open_external_url', { url: url || contactUrl.value })
}

function handleEnterApp() {
  emit('activated')
}

async function copySetupPrompt() {
  try {
    await navigator.clipboard.writeText(SETUP_PROMPT_CONTENT)
    setupCopyText.value = '已复制'
    setTimeout(() => {
      setupCopyText.value = '复制安装提示词'
    }, 2000)
  }
  catch (error) {
    setupCopyText.value = '复制失败'
    setTimeout(() => {
      setupCopyText.value = '复制安装提示词'
    }, 2000)
    console.error('复制安装提示词失败:', error)
  }
}

function handlePaste(event: ClipboardEvent) {
  const pasted = event.clipboardData?.getData('text') ?? ''
  if (!pasted)
    return

  event.preventDefault()
  licenseKey.value = pasted.replace(/\s+/g, '').trim()
}

onMounted(async () => {
  await nextTick()
  setTimeout(() => {
    inputRef.value?.focus()
  }, 120)
})

watch(licenseKey, () => {
  if (errorMsg.value)
    errorMsg.value = ''
})

watch(() => props.previewActivatedStatus, (status) => {
  if (!status) {
    activated.value = false
    activatedTitle.value = ''
    activatedDescription.value = ''
    return
  }

  const message = resolveActivatedMessage(status)
  activated.value = true
  activatedTitle.value = message.title
  activatedDescription.value = message.description
}, { immediate: true })
</script>

<template>
  <div class="trial-overlay">
    <div class="trial-card">
      <div class="trial-logo">
        <img :src="iterateIconUrl" alt="Iterate" class="trial-logo-img">
      </div>

      <p class="trial-eyebrow">
        欢迎使用 Iterate
      </p>

      <h1 class="trial-title">
        {{ overlayTitle }}
      </h1>
      <p class="trial-subtitle">
        {{ overlaySubtitle }}
      </p>

      <div v-if="trialStatus.time_anomaly" class="trial-info">
        <div class="trial-warning">
          ⚠ 检测到系统时间异常
        </div>
      </div>

      <div v-if="activated" class="trial-success">
        <p class="trial-success-title">
          {{ activatedTitle }}
        </p>
        <p class="trial-success-description">
          {{ activatedDescription }}
        </p>
      </div>

      <div v-if="!activated" class="trial-activate">
        <input
          id="trial-license-input"
          ref="inputRef"
          v-model="licenseKey"
          type="text"
          class="trial-input"
          placeholder="请输入激活码"
          :disabled="activating || activated"
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          aria-label="激活码"
          @keyup.enter="handleActivate"
          @paste="handlePaste"
        >
        <div v-if="errorMsg" class="trial-error">
          {{ errorMsg }}
        </div>
        <button
          type="button"
          class="trial-btn-primary"
          :disabled="activating || activated"
          @click="handleActivate"
        >
          {{ activated ? '激活成功' : activating ? '验证中...' : '激活' }}
        </button>
      </div>

      <p v-if="activated" class="trial-helper">
        如果你要把 iterate 接到当前 IDE 或 CLI，可以先复制安装提示词，再直接开始使用。
      </p>

      <div v-if="activated" class="trial-post-actions">
        <button type="button" class="trial-btn-secondary" @click="copySetupPrompt">
          {{ setupCopyText }}
        </button>
        <button type="button" class="trial-btn-primary" @click="handleEnterApp">
          直接开始使用
        </button>
      </div>

      <div v-else class="trial-contact-actions">
        <button type="button" class="trial-btn-secondary" @click="openContactUrl(contactUrl)">
          去官网购买激活码
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.trial-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 99999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(250, 250, 250, 0.92);
  backdrop-filter: blur(12px);
  -webkit-app-region: drag;
}

.trial-card {
  -webkit-app-region: no-drag;
  max-width: 380px;
  width: 90%;
  padding: 36px 28px;
  border-radius: 24px;
  background: #fff;
  border: 1px solid #e5e5e5;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.08);
  text-align: center;
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}

.trial-logo {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}

.trial-logo-img {
  width: 56px;
  height: 56px;
  border-radius: 14px;
}

.trial-eyebrow {
  margin: 0 0 8px;
  font-size: 13px;
  line-height: 1.4;
  color: #525252;
}

.trial-title {
  font-size: 20px;
  font-weight: 700;
  color: #171717;
  margin: 0 0 6px;
  line-height: 1.3;
}

.trial-subtitle {
  font-size: 13px;
  color: #737373;
  margin: 0 0 20px;
  line-height: 1.5;
}

.trial-info {
  background: #fafafa;
  border: 1px solid #f0f0f0;
  border-radius: 12px;
  padding: 12px 16px;
  margin-bottom: 20px;
}

.trial-info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 5px 0;
}

.trial-info-row + .trial-info-row {
  border-top: 1px solid #f0f0f0;
}

.trial-info-label {
  font-size: 12px;
  color: #a3a3a3;
}

.trial-info-value {
  font-size: 12px;
  color: #525252;
  font-variant-numeric: tabular-nums;
}

.trial-warning {
  margin-top: 6px;
  padding: 5px 10px;
  border-radius: 6px;
  background: #fef3c7;
  color: #92400e;
  font-size: 11px;
}

.trial-success {
  margin-bottom: 20px;
  padding: 12px 14px;
  border-radius: 12px;
  background: #f4f8f3;
  border: 1px solid #d9e8d5;
  text-align: left;
}

.trial-success-title {
  margin: 0 0 4px;
  font-size: 13px;
  font-weight: 600;
  color: #1f5130;
}

.trial-success-description {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: #4e6a56;
}

.trial-helper {
  margin: 0 0 14px;
  font-size: 12px;
  line-height: 1.6;
  color: #737373;
}

.trial-activate {
  margin-bottom: 14px;
}

.trial-input {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid #e5e5e5;
  border-radius: 10px;
  font-size: 13px;
  color: #171717;
  background: #fafafa;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
  text-align: center;
  letter-spacing: 1px;
}

.trial-input:focus {
  border-color: #171717;
  background: #fff;
}

.trial-input::placeholder {
  color: #c4c4c4;
  letter-spacing: 0;
}

.trial-input:disabled {
  opacity: 0.5;
}

.trial-error {
  margin-top: 6px;
  font-size: 12px;
  color: #dc2626;
}

.trial-btn-primary {
  width: 100%;
  margin-top: 10px;
  padding: 10px 0;
  border: none;
  border-radius: 10px;
  background: #171717;
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
}

.trial-btn-primary:hover {
  opacity: 0.85;
}

.trial-btn-primary:active {
  opacity: 0.7;
}

.trial-btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.trial-contact-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.trial-post-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  align-items: stretch;
}

.trial-btn-secondary {
  width: 100%;
  padding: 10px 0;
  border: 1px solid #e5e5e5;
  border-radius: 10px;
  background: #fff;
  color: #525252;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.trial-btn-secondary:hover {
  background: #fafafa;
  border-color: #d4d4d4;
}

.trial-post-actions .trial-btn-primary,
.trial-post-actions .trial-btn-secondary {
  margin-top: 0;
}

@media (max-width: 520px) {
  .trial-post-actions {
    grid-template-columns: 1fr;
  }
}
</style>
