<script setup lang="ts">
import type { ConnectionStatusSnapshot } from './useConnectionRouteStatus'
import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { computed, ref, watch } from 'vue'
import { useLocalQrCode } from '../../composables/useLocalQrCode'
import { bridgeFetch } from '../../services/bridgeFetch'
import ConnectionRouteStatusPanel from './ConnectionRouteStatusPanel.vue'
import { buildConnectionRouteView } from './useConnectionRouteStatus'

interface PairingCandidate {
  transport_mode: string
  base_url: string
  ws_url: string
  relay_device_id?: string | null
  relay_pairing_token?: string | null
  health?: string | null
  disabled?: boolean | null
  warning?: string | null
}

interface PairingPayload {
  version: number
  device_id: string
  device_name: string
  transport_mode: string
  base_url: string
  ws_url: string
  relay_device_id?: string | null
  relay_pairing_token?: string | null
  health?: string | null
  disabled?: boolean | null
  candidates?: PairingCandidate[]
  pairing_token: string
  issued_at: string
  expires_at: string
  warning?: string | null
}

interface BridgeOriginRecoveryResponse {
  status: 'already_healthy' | 'recovery_started' | 'cooldown_active' | 'failed' | string
  origin_state: string
  healthy: boolean
  recovered: boolean
  cooldown_remaining_secs: number
  message: string
}

const show = defineModel<boolean>('show', { default: false })

const message = useMessage()
const originHealthy = ref(false)
const pairingPayload = ref<PairingPayload | null>(null)
const pairingError = ref('')
const recoveryStatusText = ref('')
const connectionStatus = ref<ConnectionStatusSnapshot | null>(null)
const isLoading = ref(false)
const copySuccess = ref(false)
let qrImageLoadStartedAt = 0

function elapsedMs(startedAt: number) {
  return Math.round(performance.now() - startedAt)
}

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function urlHost(value?: string | null) {
  if (!value)
    return ''
  try {
    return new URL(value).hostname.toLowerCase()
  }
  catch {
    return value.toLowerCase()
  }
}

function isTailscaleFunnelUrl(value?: string | null) {
  return urlHost(value).endsWith('.ts.net')
}

function isCloudflareUrl(value?: string | null) {
  const host = urlHost(value)
  return host.includes('cloudflare') || host.endsWith('trycloudflare.com')
}

function transportLabel(mode: string, url?: string | null) {
  switch (mode) {
    case 'tailscale':
      return 'Tailscale'
    case 'public_tunnel':
      if (isTailscaleFunnelUrl(url))
        return 'Tailscale Funnel'
      if (isCloudflareUrl(url))
        return 'Cloudflare 公网'
      return '公网通道'
    case 'cloudflare_tunnel':
      return 'Cloudflare 公网'
    case 'relay':
      return 'Relay'
    case 'lan_fallback':
      return 'LAN 同网备用'
    case 'loopback_fallback':
      return '本机调试'
    default:
      return mode
  }
}

function encodeBase64Url(value: string) {
  const bytes = new TextEncoder().encode(value)
  let binary = ''
  for (const byte of bytes)
    binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function compactPairingPayload(payload: PairingPayload) {
  const candidates = (payload.candidates || []).map(candidate => ({
    transport_mode: candidate.transport_mode,
    base_url: candidate.base_url,
    ws_url: candidate.ws_url,
    ...(candidate.relay_device_id ? { relay_device_id: candidate.relay_device_id } : {}),
    ...(candidate.relay_pairing_token ? { relay_pairing_token: candidate.relay_pairing_token } : {}),
    ...(candidate.health ? { health: candidate.health } : {}),
    ...(candidate.disabled ? { disabled: true } : {}),
    ...(candidate.warning ? { warning: candidate.warning } : {}),
  }))

  return {
    transport_mode: payload.transport_mode,
    base_url: payload.base_url,
    ws_url: payload.ws_url,
    ...(payload.relay_device_id ? { relay_device_id: payload.relay_device_id } : {}),
    ...(payload.relay_pairing_token ? { relay_pairing_token: payload.relay_pairing_token } : {}),
    ...(payload.health ? { health: payload.health } : {}),
    ...(payload.disabled ? { disabled: true } : {}),
    pairing_token: payload.pairing_token,
    ...(payload.warning ? { warning: payload.warning } : {}),
    ...(candidates.length ? { candidates } : {}),
  }
}

const pairingImportUrl = computed(() => {
  if (!pairingPayload.value)
    return ''
  const serialized = JSON.stringify(compactPairingPayload(pairingPayload.value))
  return `iterate://pairing?payload=${encodeURIComponent(encodeBase64Url(serialized))}`
})

const {
  dataUrl: pairingQrCodeUrl,
  error: pairingQrCodeError,
} = useLocalQrCode(pairingImportUrl, { width: 320 })

const pairingPrimaryText = computed(() => {
  if (!pairingPayload.value)
    return ''
  return `当前首选：${transportLabel(pairingPayload.value.transport_mode, pairingPayload.value.base_url)} · ${pairingPayload.value.ws_url || pairingPayload.value.base_url}`
})

const pairingCandidateLabels = computed(() => {
  const candidates = pairingPayload.value?.candidates || []
  return candidates.map((candidate, index) => {
    const label = transportLabel(candidate.transport_mode, candidate.base_url)
    return `${index + 1}. ${label}${candidate.disabled ? ' · 已禁用' : ''}`
  })
})

const pairingRouteDescription = computed(() => {
  if (!pairingPayload.value)
    return 'iPhone 扫描 Companion 配对二维码后，会按二维码中的候选通道连接当前 Mac。'

  const label = transportLabel(pairingPayload.value.transport_mode, pairingPayload.value.base_url)
  return `iPhone 扫描 Companion 配对二维码后，会按候选通道连接当前 Mac；当前首选 ${label}。`
})

const tailscaleCandidateAvailable = computed(() => {
  if (pairingPayload.value?.transport_mode === 'tailscale')
    return true
  return (pairingPayload.value?.candidates || []).some(
    candidate => candidate.transport_mode === 'tailscale' && !candidate.disabled,
  )
})

const connectionRouteView = computed(() => buildConnectionRouteView({
  connectionStatus: connectionStatus.value,
  localBridgeHealthy: originHealthy.value,
  tailscaleClientOnline: false,
  tailscaleCandidateAvailable: tailscaleCandidateAvailable.value,
}))

async function refreshConnectionRouteStatus() {
  try {
    // 8080 hung 时 fetch 会无限挂起，卡死「重试」流程走不到自愈；必须带超时
    const response = await bridgeFetch('http://127.0.0.1:8080/api/connection-status', {
      cache: 'no-store',
      signal: AbortSignal.timeout(3000),
    })
    const data = await response.json()
    if (!response.ok)
      return
    connectionStatus.value = {
      diagnosis: data.diagnosis,
      local_origin: data.local_origin,
      public_tunnel: data.public_tunnel,
      root_tunnel: data.root_tunnel,
    }
  }
  catch {
    connectionStatus.value = null
  }
}

function connectionStatusLocalBridgeHealthy() {
  return connectionStatus.value?.local_origin?.healthy === true
}

async function checkLocalBridgeHealth() {
  await refreshConnectionRouteStatus()
  if (connectionStatusLocalBridgeHealthy())
    return true

  try {
    return await invoke('check_origin_health') as boolean
  }
  catch {
    return false
  }
}

function recoveryStatusMessage(recovery: BridgeOriginRecoveryResponse) {
  switch (recovery.status) {
    case 'already_healthy':
      return '本地 bridge 已恢复，正在重新生成配对二维码'
    case 'recovery_started':
      return '已触发本地 bridge 自愈，正在重新检查'
    case 'cooldown_active':
      return `本地 bridge 刚刚执行过恢复，${recovery.cooldown_remaining_secs}s 后可再次尝试`
    case 'failed':
      return '本地 bridge 自愈失败，请稍后重试'
    default:
      return recovery.message || '本地 bridge 自愈状态未知'
  }
}

async function recoverOriginIfNeeded() {
  const recoveryStartedAt = performance.now()
  const recovery = await invoke('recover_bridge_origin') as BridgeOriginRecoveryResponse
  recoveryStatusText.value = recoveryStatusMessage(recovery)
  console.info('[MobilePairingModal] origin_recovery_done', {
    status: recovery.status,
    originState: recovery.origin_state,
    healthy: recovery.healthy,
    recovered: recovery.recovered,
    cooldownRemainingSecs: recovery.cooldown_remaining_secs,
    elapsedMs: elapsedMs(recoveryStartedAt),
  })

  if (recovery.status === 'recovery_started')
    await sleep(2000)

  return recovery
}

async function loadPairingQr(options: { recoverOnUnhealthy?: boolean } = {}) {
  const startedAt = performance.now()
  console.info('[MobilePairingModal] load_start')
  isLoading.value = true
  pairingError.value = ''
  recoveryStatusText.value = ''
  try {
    const healthStartedAt = performance.now()
    originHealthy.value = await checkLocalBridgeHealth()
    console.info('[MobilePairingModal] origin_health_done', {
      healthy: originHealthy.value,
      source: connectionStatusLocalBridgeHealthy() ? 'connection-status' : 'tauri-command',
      elapsedMs: elapsedMs(healthStartedAt),
    })
    if (!originHealthy.value && options.recoverOnUnhealthy) {
      await recoverOriginIfNeeded()
      const recheckStartedAt = performance.now()
      originHealthy.value = await checkLocalBridgeHealth()
      console.info('[MobilePairingModal] origin_health_recheck_done', {
        healthy: originHealthy.value,
        source: connectionStatusLocalBridgeHealthy() ? 'connection-status' : 'tauri-command',
        elapsedMs: elapsedMs(recheckStartedAt),
      })
    }
    if (!originHealthy.value)
      throw new Error(recoveryStatusText.value || '8080 端口不可达，请确保 iterate 服务已启动')

    const fetchStartedAt = performance.now()
    const response = await bridgeFetch('http://127.0.0.1:8080/api/mobile/pairing', {
      signal: AbortSignal.timeout(5000),
    })
    console.info('[MobilePairingModal] pairing_fetch_done', {
      ok: response.ok,
      status: response.status,
      elapsedMs: elapsedMs(fetchStartedAt),
    })
    const data = await response.json()
    if (!response.ok || !data.ok || !data.pairing)
      throw new Error(data.error || '配对信息获取失败')

    pairingPayload.value = data.pairing as PairingPayload
    console.info('[MobilePairingModal] payload_ready', {
      transportMode: pairingPayload.value.transport_mode,
      candidateCount: pairingPayload.value.candidates?.length || 0,
      totalElapsedMs: elapsedMs(startedAt),
    })
  }
  catch (error: any) {
    pairingPayload.value = null
    pairingError.value = String(error?.message || error || '配对二维码生成失败')
    console.warn('[MobilePairingModal] load_failed', {
      error: pairingError.value,
      totalElapsedMs: elapsedMs(startedAt),
    })
  }
  finally {
    isLoading.value = false
    console.info('[MobilePairingModal] load_done', {
      hasPayload: Boolean(pairingPayload.value),
      hasQrUrl: Boolean(pairingQrCodeUrl.value),
      totalElapsedMs: elapsedMs(startedAt),
    })
  }
}

async function retryPairingQr() {
  await loadPairingQr({ recoverOnUnhealthy: true })
}

async function refreshPairingQr() {
  await loadPairingQr()
}

function onPairingQrImageLoad() {
  console.info('[MobilePairingModal] qr_image_load', {
    srcLength: pairingQrCodeUrl.value.length,
    elapsedMs: elapsedMs(qrImageLoadStartedAt),
  })
}

function onPairingQrImageError() {
  console.warn('[MobilePairingModal] qr_image_error', {
    srcLength: pairingQrCodeUrl.value.length,
    elapsedMs: elapsedMs(qrImageLoadStartedAt),
  })
}

async function copyPairingLink() {
  if (!pairingImportUrl.value)
    return

  try {
    await navigator.clipboard.writeText(pairingImportUrl.value)
    copySuccess.value = true
    message.success('配对链接已复制')
    setTimeout(() => {
      copySuccess.value = false
    }, 2000)
  }
  catch {
    message.error('复制失败')
  }
}

watch(show, (visible) => {
  if (visible)
    loadPairingQr()
})

watch(pairingQrCodeUrl, (url) => {
  if (!url)
    return
  qrImageLoadStartedAt = performance.now()
  console.info('[MobilePairingModal] qr_image_start', {
    srcLength: url.length,
  })
})

watch(pairingQrCodeError, (error) => {
  if (!error)
    return
  pairingError.value = `本地二维码生成失败：${error.message}`
  console.warn('[MobilePairingModal] qr_generation_error', {
    error: error.message,
  })
})
</script>

<template>
  <n-modal
    v-model:show="show"
    preset="card"
    title="手机连接"
    class="iterate-pairing-modal"
    :bordered="false"
  >
    <n-space vertical size="large">
      <div v-if="isLoading" class="flex flex-col items-center justify-center py-12 gap-3">
        <div class="w-5 h-5 border-2 border-primary border-t-transparent rounded-full animate-spin" />
        <div class="text-xs opacity-60">
          正在生成 Companion 配对二维码
        </div>
      </div>

      <template v-else-if="pairingQrCodeUrl">
        <div class="flex justify-center pt-1">
          <img
            :src="pairingQrCodeUrl"
            alt="Companion pairing QR code"
            class="w-80 h-80 max-w-full rounded bg-white p-2"
            @load="onPairingQrImageLoad"
            @error="onPairingQrImageError"
          >
        </div>

        <div class="text-xs opacity-65 leading-relaxed">
          {{ pairingPrimaryText }}
        </div>

        <div v-if="pairingCandidateLabels.length" class="flex flex-wrap gap-1.5">
          <span
            v-for="label in pairingCandidateLabels"
            :key="label"
            class="text-[11px] leading-relaxed bg-black-200 px-2 py-0.5 rounded"
          >
            {{ label }}
          </span>
        </div>

        <div v-if="pairingPayload?.warning" class="text-[11px] opacity-55 leading-relaxed">
          {{ pairingPayload.warning }}
        </div>

        <div class="flex justify-end gap-2">
          <n-button size="small" @click="refreshPairingQr">
            刷新
          </n-button>
          <n-button size="small" :type="copySuccess ? 'success' : 'primary'" @click="copyPairingLink">
            {{ copySuccess ? '已复制' : '复制链接' }}
          </n-button>
        </div>

        <details class="rounded border border-black-200/70 px-3 py-2">
          <summary class="cursor-pointer text-xs opacity-65 select-none">
            连接状态
          </summary>
          <div class="mt-3">
            <ConnectionRouteStatusPanel :route-view="connectionRouteView" compact />
          </div>
          <div class="mt-3 text-[11px] opacity-55 leading-relaxed">
            {{ pairingRouteDescription }}
          </div>
        </details>
      </template>

      <template v-else>
        <n-alert type="warning" :bordered="false">
          {{ pairingError || '配对二维码生成失败' }}
        </n-alert>
        <div v-if="recoveryStatusText" class="text-xs text-warning leading-relaxed">
          {{ recoveryStatusText }}
        </div>
        <div class="flex justify-end">
          <n-button size="small" type="primary" :loading="isLoading" @click="retryPairingQr">
            重试
          </n-button>
        </div>
        <ConnectionRouteStatusPanel :route-view="connectionRouteView" compact />
      </template>
    </n-space>
  </n-modal>
</template>

<style scoped>
.iterate-pairing-modal {
  width: min(420px, calc(100vw - 32px));
}
</style>
