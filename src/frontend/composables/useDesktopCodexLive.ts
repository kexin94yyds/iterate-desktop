import type { DesktopCodexLiveExecutionPhase } from '../services/desktopCodexLiveControl'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { advanceCodexLiveTranscript } from '../utils/codexLiveTranscript'
import {
  advanceCodexLiveExplicitExecutionRequestPending,
  isCodexLiveAffirmativeDecision,
  isCodexLiveExecutionQuestion,
  isCodexLiveNegativeDecision,
  isExplicitCodexLiveExecutionRequest,
  normalizeCodexLiveVoiceDecision,
} from '../utils/codexLiveVoiceDecision'

export type DesktopCodexLivePhase
  = | 'idle'
    | 'preparing'
    | 'connecting'
    | 'active'
    | 'reconnecting'
    | 'failed'

interface BrokerMessage {
  type?: string
  session_id?: string
  status?: string
  sdp?: string
  role?: string
  text?: string
  code?: string
  message?: string
  thread_id?: string
  kind?: string
}

const LIVE_SOCKET_URL = 'ws://127.0.0.1:8080/ws/codex-live'
const LIVE_PROTOCOL = 'iterate.codex-live.v1'
const MAX_RECONNECT_ATTEMPTS = 3
const RECONNECT_DELAYS_MS = [800, 1600, 3200]
const MICROPHONE_START_TIMEOUT_MS = 12_000
const LIVE_DIAGNOSTICS_INTERVAL_MS = 5_000

const phase = ref<DesktopCodexLivePhase>('idle')
const executionPhase = ref<DesktopCodexLiveExecutionPhase>('waiting')
const statusText = ref('启动全局 GPT-Live 主代理')
const taskProgressText = ref('')
const latestTranscript = ref('')
const activeProjectPath = ref<string | null>(null)
const activeThreadId = ref<string | null>(null)
const isMicrophoneMuted = ref(false)
const isActive = computed(() => ['preparing', 'connecting', 'active', 'reconnecting'].includes(phase.value))

let generation = 0
let requestedStop = true
let reconnectAttempt = 0
let reconnectTimer: number | null = null
let socket: WebSocket | null = null
let peer: RTCPeerConnection | null = null
let mediaStream: MediaStream | null = null
let outputAudio: HTMLAudioElement | null = null
let dataChannel: RTCDataChannel | null = null
let sessionId: string | null = null
let explicitExecutionRequestPending = false
let agentProgressBuffer = ''
let transcriptRole: string | undefined
let transcriptFinalized = true
let reservationHeld = false
let reservationDesired = false
let reservationTransition: Promise<void> = Promise.resolve()
let diagnosticsTimer: number | null = null
let diagnosticsLastTickAt = 0
let diagnosticsLastUserFinalAt: number | null = null
let diagnosticsAssistantResponseStarted = false

export {
  isCodexLiveAffirmativeDecision,
  isCodexLiveExecutionQuestion,
  isCodexLiveNegativeDecision,
  isExplicitCodexLiveExecutionRequest,
  normalizeCodexLiveVoiceDecision,
}

function createSessionId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID().toLowerCase()
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (character) => {
    const random = Math.floor(Math.random() * 16)
    const value = character === 'x' ? random : (random & 0x3) | 0x8
    return value.toString(16)
  })
}

function summarizeStatus(value: string, fallback: string): string {
  const normalized = value.trim().replace(/\s+/g, ' ')
  if (!normalized)
    return fallback
  return normalized.length > 160 ? `${normalized.slice(0, 157)}…` : normalized
}

function microphoneErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message === 'microphone_start_timeout')
    return '麦克风启动超时，请点击常驻 Live HUD 后重试'
  if (error instanceof DOMException && error.name === 'NotAllowedError')
    return '请在系统设置中允许 iterate 使用麦克风'
  if (error instanceof DOMException && error.name === 'NotFoundError')
    return '没有找到可用的麦克风'
  return '无法启动 GPT-Live 麦克风'
}

function traceAudioStage(stage: string) {
  void invoke('debug_log', {
    message: `[GPT-Live Audio] ${stage}`,
  }).catch(() => {})
}

function traceLiveDiagnostics(event: string, details: Record<string, unknown> = {}) {
  void invoke('debug_log', {
    message: `[GPT-Live Diagnostics] ${event} ${JSON.stringify(details)}`,
  }).catch(() => {})
}

function pickRtcStats(report: Record<string, unknown>, keys: string[]) {
  return Object.fromEntries(keys
    .filter(key => report[key] !== undefined)
    .map(key => [key, report[key]]))
}

async function captureRtcStats(connection: RTCPeerConnection, expectedGeneration: number, reason: string) {
  const startedAt = performance.now()
  try {
    const reports = await connection.getStats()
    if (expectedGeneration !== generation || peer !== connection)
      return
    let inboundAudio: Record<string, unknown> | null = null
    let outboundAudio: Record<string, unknown> | null = null
    let remoteInboundAudio: Record<string, unknown> | null = null
    let selectedCandidatePair: Record<string, unknown> | null = null
    reports.forEach((rawReport) => {
      const report = rawReport as unknown as Record<string, unknown>
      const kind = report.kind ?? report.mediaType
      if (report.type === 'inbound-rtp' && kind === 'audio') {
        inboundAudio = pickRtcStats(report, [
          'packetsReceived',
          'packetsLost',
          'jitter',
          'jitterBufferDelay',
          'jitterBufferEmittedCount',
          'concealedSamples',
          'silentConcealedSamples',
          'totalSamplesReceived',
          'audioLevel',
          'bytesReceived',
        ])
      }
      else if (report.type === 'outbound-rtp' && kind === 'audio') {
        outboundAudio = pickRtcStats(report, [
          'packetsSent',
          'bytesSent',
          'retransmittedPacketsSent',
          'totalPacketSendDelay',
        ])
      }
      else if (report.type === 'remote-inbound-rtp' && kind === 'audio') {
        remoteInboundAudio = pickRtcStats(report, [
          'packetsLost',
          'fractionLost',
          'jitter',
          'roundTripTime',
          'totalRoundTripTime',
          'roundTripTimeMeasurements',
        ])
      }
      else if (report.type === 'candidate-pair'
        && report.state === 'succeeded'
        && (report.selected === true || report.nominated === true)) {
        selectedCandidatePair = pickRtcStats(report, [
          'currentRoundTripTime',
          'availableOutgoingBitrate',
          'availableIncomingBitrate',
          'bytesSent',
          'bytesReceived',
          'requestsReceived',
          'requestsSent',
          'responsesReceived',
          'responsesSent',
        ])
      }
    })
    const now = performance.now()
    const eventLoopLagMs = diagnosticsLastTickAt > 0
      ? Math.max(0, Math.round(now - diagnosticsLastTickAt - LIVE_DIAGNOSTICS_INTERVAL_MS))
      : 0
    diagnosticsLastTickAt = now
    traceLiveDiagnostics('rtc_stats', {
      reason,
      collection_ms: Math.round(now - startedAt),
      event_loop_lag_ms: eventLoopLagMs,
      connection_state: connection.connectionState,
      ice_connection_state: connection.iceConnectionState,
      ice_gathering_state: connection.iceGatheringState,
      signaling_state: connection.signalingState,
      inbound_audio: inboundAudio,
      outbound_audio: outboundAudio,
      remote_inbound_audio: remoteInboundAudio,
      selected_candidate_pair: selectedCandidatePair,
    })
  }
  catch (error) {
    traceLiveDiagnostics('rtc_stats_failed', {
      reason,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

function stopLiveDiagnostics() {
  if (diagnosticsTimer !== null)
    window.clearInterval(diagnosticsTimer)
  diagnosticsTimer = null
  diagnosticsLastTickAt = 0
  diagnosticsLastUserFinalAt = null
  diagnosticsAssistantResponseStarted = false
}

function startLiveDiagnostics(connection: RTCPeerConnection, expectedGeneration: number) {
  stopLiveDiagnostics()
  diagnosticsLastTickAt = performance.now()
  void captureRtcStats(connection, expectedGeneration, 'start')
  diagnosticsTimer = window.setInterval(() => {
    void captureRtcStats(connection, expectedGeneration, 'interval')
  }, LIVE_DIAGNOSTICS_INTERVAL_MS)
}

async function acquireMicrophone(): Promise<MediaStream> {
  traceAudioStage('getUserMedia.request')
  let expired = false
  let timeout: number | null = null
  const request = navigator.mediaDevices.getUserMedia({
    audio: {
      channelCount: 1,
      echoCancellation: true,
      noiseSuppression: true,
      autoGainControl: true,
    },
    video: false,
  })
  void request.then((stream) => {
    if (expired)
      stream.getTracks().forEach(track => track.stop())
  }).catch(() => {})

  try {
    const stream = await Promise.race([
      request,
      new Promise<never>((_resolve, reject) => {
        timeout = window.setTimeout(() => {
          expired = true
          reject(new Error('microphone_start_timeout'))
        }, MICROPHONE_START_TIMEOUT_MS)
      }),
    ])
    traceAudioStage('getUserMedia.acquired')
    return stream
  }
  finally {
    if (timeout !== null)
      window.clearTimeout(timeout)
  }
}

function waitForIceGathering(connection: RTCPeerConnection): Promise<void> {
  if (connection.iceGatheringState === 'complete')
    return Promise.resolve()

  return new Promise((resolve) => {
    let timer: number | null = null
    function finish() {
      if (timer !== null)
        window.clearTimeout(timer)
      connection.removeEventListener('icegatheringstatechange', handleChange)
      resolve()
    }
    function handleChange() {
      if (connection.iceGatheringState === 'complete')
        finish()
    }
    timer = window.setTimeout(finish, 2500)
    connection.addEventListener('icegatheringstatechange', handleChange)
  })
}

function setAudioReservation(reserved: boolean): Promise<void> {
  reservationDesired = reserved
  reservationTransition = reservationTransition
    .catch(() => {})
    .then(async () => {
      const target = reservationDesired
      if (reservationHeld === target)
        return
      await invoke<boolean>('set_codex_live_audio_reserved', { reserved: target })
      reservationHeld = target
    })
  return reservationTransition
}

function clearReconnectTimer() {
  if (reconnectTimer !== null)
    window.clearTimeout(reconnectTimer)
  reconnectTimer = null
}

function cleanupTransport(sendStop: boolean) {
  clearReconnectTimer()
  stopLiveDiagnostics()

  const closingSocket = socket
  socket = null
  if (closingSocket) {
    closingSocket.onopen = null
    closingSocket.onmessage = null
    closingSocket.onerror = null
    closingSocket.onclose = null
    if (sendStop && closingSocket.readyState === WebSocket.OPEN && sessionId) {
      closingSocket.send(JSON.stringify({ type: 'stop', session_id: sessionId }))
    }
    closingSocket.close(1000, 'desktop_live_cleanup')
  }

  dataChannel?.close()
  dataChannel = null
  peer?.close()
  peer = null
  mediaStream?.getTracks().forEach(track => track.stop())
  mediaStream = null
  if (outputAudio) {
    outputAudio.pause()
    outputAudio.srcObject = null
    outputAudio.remove()
  }
  outputAudio = null
  sessionId = null
}

async function releaseAudioReservation() {
  try {
    await setAudioReservation(false)
  }
  catch (error) {
    console.error('释放 GPT-Live 麦克风占用失败:', error)
  }
}

async function failTerminal(message: string, expectedGeneration: number) {
  if (expectedGeneration !== generation)
    return
  generation += 1
  requestedStop = true
  cleanupTransport(true)
  phase.value = 'failed'
  executionPhase.value = 'failed'
  statusText.value = summarizeStatus(message, 'GPT-Live 启动失败')
  taskProgressText.value = statusText.value
  isMicrophoneMuted.value = false
  await releaseAudioReservation()
}

function scheduleReconnect(expectedGeneration: number, reason: string) {
  if (expectedGeneration !== generation || requestedStop)
    return
  if (reconnectAttempt >= MAX_RECONNECT_ATTEMPTS) {
    void failTerminal('GPT-Live 连接中断，请点击按钮重试', expectedGeneration)
    return
  }

  explicitExecutionRequestPending = false
  executionPhase.value = 'waiting'
  taskProgressText.value = ''
  agentProgressBuffer = ''
  cleanupTransport(false)
  phase.value = 'reconnecting'
  statusText.value = summarizeStatus(reason, 'GPT-Live 正在重新连接')
  const delay = RECONNECT_DELAYS_MS[reconnectAttempt] ?? RECONNECT_DELAYS_MS.at(-1)!
  reconnectAttempt += 1
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null
    if (!requestedStop && activeProjectPath.value)
      void beginConnection(activeProjectPath.value)
  }, delay)
}

function sendControl(type: 'confirm' | 'stop' | 'interrupt') {
  if (!socket || socket.readyState !== WebSocket.OPEN || !sessionId)
    return false
  socket.send(JSON.stringify({ type, session_id: sessionId }))
  return true
}

function cancelRealtimeResponse() {
  if (!dataChannel || dataChannel.readyState !== 'open')
    return false
  const eventId = createSessionId()
  dataChannel.send(JSON.stringify({
    type: 'response.cancel',
    event_id: `${eventId}-response`,
  }))
  dataChannel.send(JSON.stringify({
    type: 'output_audio_buffer.clear',
    event_id: `${eventId}-audio`,
  }))
  traceLiveDiagnostics('current_response_cancelled')
  return true
}

function updateTranscript(message: BrokerMessage, isFinal: boolean) {
  const rawText = message.text ?? ''
  const nextTranscript = advanceCodexLiveTranscript({
    text: latestTranscript.value,
    role: transcriptRole,
    finalized: transcriptFinalized,
  }, rawText, message.role, isFinal)
  if (!nextTranscript)
    return
  const text = rawText.trim()
  const role = nextTranscript.role
  const now = performance.now()
  if (role === 'assistant' && !diagnosticsAssistantResponseStarted) {
    diagnosticsAssistantResponseStarted = true
    traceLiveDiagnostics('assistant_response_started', {
      after_user_final_ms: diagnosticsLastUserFinalAt === null
        ? null
        : Math.round(now - diagnosticsLastUserFinalAt),
      first_event_final: isFinal,
      chunk_chars: rawText.length,
    })
  }
  latestTranscript.value = nextTranscript.text
  transcriptRole = nextTranscript.role
  transcriptFinalized = nextTranscript.finalized

  if (isFinal && role === 'user'
    && (executionPhase.value === 'completed' || executionPhase.value === 'failed')) {
    executionPhase.value = 'waiting'
    taskProgressText.value = ''
    agentProgressBuffer = ''
  }
  if (nextTranscript.text && executionPhase.value === 'waiting')
    statusText.value = summarizeStatus(latestTranscript.value, 'GPT-Live 已连接')

  if (!isFinal)
    return
  if (!text)
    return
  if (role === 'assistant') {
    traceLiveDiagnostics('assistant_transcript_final', {
      after_user_final_ms: diagnosticsLastUserFinalAt === null
        ? null
        : Math.round(now - diagnosticsLastUserFinalAt),
      chars: text.length,
    })
    return
  }
  if (role !== 'user')
    return

  diagnosticsLastUserFinalAt = now
  diagnosticsAssistantResponseStarted = false
  traceLiveDiagnostics('user_transcript_final', { chars: text.length })

  if (isCodexLiveNegativeDecision(text)) {
    explicitExecutionRequestPending = false
    executionPhase.value = 'waiting'
    taskProgressText.value = ''
    statusText.value = '已保持讨论模式，不会执行'
  }
  else {
    explicitExecutionRequestPending = advanceCodexLiveExplicitExecutionRequestPending(
      explicitExecutionRequestPending,
      text,
    )
    if (isExplicitCodexLiveExecutionRequest(text)) {
      executionPhase.value = 'submitting'
      statusText.value = '任务已收到，Codex 正在开始执行'
      taskProgressText.value = statusText.value
    }
  }
}

function handleBrokerMessage(raw: string, expectedGeneration: number) {
  if (expectedGeneration !== generation)
    return
  let message: BrokerMessage
  try {
    message = JSON.parse(raw) as BrokerMessage
  }
  catch {
    return
  }
  if (message.session_id && message.session_id !== sessionId)
    return

  switch (message.type) {
    case 'answer':
      if (!message.sdp || !peer)
        return
      void peer.setRemoteDescription({ type: 'answer', sdp: message.sdp }).catch(() => {
        scheduleReconnect(expectedGeneration, 'GPT-Live 无法接受远端音频连接，正在重试')
      })
      break
    case 'status':
      if (message.thread_id)
        activeThreadId.value = message.thread_id
      if (message.status === 'starting')
        statusText.value = '正在连接 Codex GPT-Live'
      break
    case 'transcript_delta':
      updateTranscript(message, false)
      break
    case 'transcript_done':
      updateTranscript(message, true)
      break
    case 'task_started':
      if (message.thread_id)
        activeThreadId.value = message.thread_id
      statusText.value = 'Codex 已开始执行'
      taskProgressText.value = statusText.value
      executionPhase.value = 'running'
      agentProgressBuffer = ''
      explicitExecutionRequestPending = false
      break
    case 'task_progress':
      if (message.text) {
        executionPhase.value = 'running'
        if (message.kind === 'agent') {
          agentProgressBuffer = `${agentProgressBuffer}${message.text}`.slice(-600)
          taskProgressText.value = agentProgressBuffer
          statusText.value = summarizeStatus(agentProgressBuffer, 'Codex 正在执行')
        }
        else {
          taskProgressText.value = message.text.slice(-600)
          statusText.value = summarizeStatus(message.text, 'Codex 正在执行')
        }
      }
      break
    case 'task_completed':
      executionPhase.value = 'completed'
      statusText.value = summarizeStatus(message.text || '', 'Codex 已完成任务，GPT-Live 继续聆听')
      taskProgressText.value = (message.text || statusText.value).slice(-600)
      agentProgressBuffer = ''
      explicitExecutionRequestPending = false
      break
    case 'task_interrupted':
      executionPhase.value = 'waiting'
      latestTranscript.value = summarizeStatus(message.text || '', 'Codex 网络临时中断，GPT-Live 仍保持连接')
      statusText.value = latestTranscript.value
      taskProgressText.value = ''
      agentProgressBuffer = ''
      explicitExecutionRequestPending = false
      break
    case 'interaction_interrupted':
      executionPhase.value = 'waiting'
      latestTranscript.value = summarizeStatus(message.text || '', '已取消当前对话，GPT-Live 继续聆听')
      statusText.value = latestTranscript.value
      taskProgressText.value = ''
      agentProgressBuffer = ''
      explicitExecutionRequestPending = false
      break
    case 'task_failed':
      executionPhase.value = 'failed'
      statusText.value = summarizeStatus(message.text || '', 'Codex 执行失败，GPT-Live 仍保持连接')
      taskProgressText.value = (message.text || statusText.value).slice(-600)
      agentProgressBuffer = ''
      explicitExecutionRequestPending = false
      break
    case 'error':
      traceLiveDiagnostics('broker_error', {
        code: message.code ?? null,
        message: summarizeStatus(message.message || '', 'Codex GPT-Live 发生错误'),
      })
      if (message.code?.startsWith('execution_') || message.code?.startsWith('interaction_')) {
        executionPhase.value = 'failed'
        statusText.value = summarizeStatus(message.message || '', 'Codex 执行失败，GPT-Live 仍保持连接')
        taskProgressText.value = (message.message || statusText.value).slice(-600)
        agentProgressBuffer = ''
      }
      else {
        void failTerminal(message.message || 'Codex GPT-Live 发生错误', expectedGeneration)
      }
      break
    case 'closed':
      scheduleReconnect(expectedGeneration, 'GPT-Live 连接已断开，正在重新连接')
      break
  }
}

async function beginConnection(projectPath: string) {
  const expectedGeneration = ++generation
  cleanupTransport(false)
  phase.value = reconnectAttempt > 0 ? 'reconnecting' : 'preparing'
  statusText.value = reconnectAttempt > 0 ? 'GPT-Live 正在重新连接' : '正在准备麦克风'
  transcriptRole = undefined
  transcriptFinalized = true
  explicitExecutionRequestPending = false
  executionPhase.value = 'waiting'
  taskProgressText.value = ''
  agentProgressBuffer = ''

  try {
    await setAudioReservation(true)
    if (expectedGeneration !== generation || requestedStop)
      return

    const stream = await acquireMicrophone()
    if (expectedGeneration !== generation || requestedStop) {
      stream.getTracks().forEach(track => track.stop())
      return
    }
    mediaStream = stream
    stream.getAudioTracks().forEach((track) => {
      track.enabled = !isMicrophoneMuted.value
    })

    const connection = new RTCPeerConnection()
    peer = connection
    const audio = document.createElement('audio')
    audio.autoplay = true
    audio.setAttribute('playsinline', '')
    audio.hidden = true
    document.body.appendChild(audio)
    outputAudio = audio
    for (const eventName of ['playing', 'waiting', 'stalled', 'pause', 'ended', 'error'] as const) {
      audio.addEventListener(eventName, () => {
        traceLiveDiagnostics(`output_audio_${eventName}`, {
          current_time_ms: Math.round(audio.currentTime * 1000),
          ready_state: audio.readyState,
          network_state: audio.networkState,
          paused: audio.paused,
        })
      })
    }
    dataChannel = connection.createDataChannel('oai-events')
    dataChannel.onopen = () => traceLiveDiagnostics('data_channel_open')
    dataChannel.onclose = () => traceLiveDiagnostics('data_channel_close')
    dataChannel.onerror = () => traceLiveDiagnostics('data_channel_error')
    connection.ontrack = (event) => {
      if (expectedGeneration !== generation)
        return
      traceLiveDiagnostics('remote_track', {
        kind: event.track.kind,
        muted: event.track.muted,
        ready_state: event.track.readyState,
      })
      event.track.onmute = () => traceLiveDiagnostics('remote_track_muted')
      event.track.onunmute = () => traceLiveDiagnostics('remote_track_unmuted')
      event.track.onended = () => traceLiveDiagnostics('remote_track_ended')
      audio.srcObject = event.streams[0] || new MediaStream([event.track])
      void audio.play().catch((error) => {
        traceLiveDiagnostics('output_audio_play_failed', {
          error: error instanceof Error ? error.message : String(error),
        })
      })
    }
    connection.onconnectionstatechange = () => {
      if (expectedGeneration !== generation)
        return
      traceLiveDiagnostics('connection_state_changed', {
        connection_state: connection.connectionState,
        ice_connection_state: connection.iceConnectionState,
      })
      if (connection.connectionState === 'connected') {
        reconnectAttempt = 0
        phase.value = 'active'
        statusText.value = isMicrophoneMuted.value
          ? '麦克风已静音，GPT-Live 保持连接'
          : 'GPT-Live 已连接，可以开始说话'
        startLiveDiagnostics(connection, expectedGeneration)
      }
      else if (connection.connectionState === 'failed' || connection.connectionState === 'closed') {
        scheduleReconnect(expectedGeneration, 'GPT-Live 音频连接中断，正在重新连接')
      }
    }
    connection.oniceconnectionstatechange = () => {
      traceLiveDiagnostics('ice_connection_state_changed', {
        ice_connection_state: connection.iceConnectionState,
      })
    }
    stream.getAudioTracks().forEach(track => connection.addTrack(track, stream))
    stream.getAudioTracks().forEach((track) => {
      const settings = track.getSettings()
      traceLiveDiagnostics('local_track_ready', {
        enabled: track.enabled,
        muted: track.muted,
        ready_state: track.readyState,
        settings: pickRtcStats(settings as unknown as Record<string, unknown>, [
          'sampleRate',
          'sampleSize',
          'channelCount',
          'echoCancellation',
          'noiseSuppression',
          'autoGainControl',
          'latency',
        ]),
      })
      track.onmute = () => traceLiveDiagnostics('local_track_muted')
      track.onunmute = () => traceLiveDiagnostics('local_track_unmuted')
      track.onended = () => traceLiveDiagnostics('local_track_ended')
    })
    await connection.setLocalDescription(await connection.createOffer())
    await waitForIceGathering(connection)
    if (expectedGeneration !== generation || requestedStop)
      return
    const sdp = connection.localDescription?.sdp
    if (!sdp)
      throw new Error('GPT-Live WebRTC offer 为空')

    const token = await invoke<string>('get_bridge_desktop_token', {
      method: 'GET',
      path: '/ws/codex-live',
    })
    if (expectedGeneration !== generation || requestedStop)
      return

    phase.value = 'connecting'
    statusText.value = '正在连接 Codex GPT-Live'
    const currentSessionId = createSessionId()
    sessionId = currentSessionId
    const liveSocket = new WebSocket(LIVE_SOCKET_URL, [
      LIVE_PROTOCOL,
      `iterate.desktop-token.${token}`,
    ])
    socket = liveSocket
    liveSocket.onopen = () => {
      if (expectedGeneration !== generation || socket !== liveSocket)
        return
      liveSocket.send(JSON.stringify({
        type: 'start',
        session_id: currentSessionId,
        sdp,
        project_path: projectPath,
      }))
    }
    liveSocket.onmessage = event => handleBrokerMessage(String(event.data), expectedGeneration)
    liveSocket.onerror = () => {
      traceLiveDiagnostics('broker_socket_error', {
        ready_state: liveSocket.readyState,
      })
      scheduleReconnect(expectedGeneration, '无法连接本机 GPT-Live 服务，正在重试')
    }
    liveSocket.onclose = (event) => {
      traceLiveDiagnostics('broker_socket_close', {
        code: event.code,
        reason: event.reason,
        was_clean: event.wasClean,
      })
      scheduleReconnect(expectedGeneration, 'GPT-Live 控制连接中断，正在重新连接')
    }
  }
  catch (error) {
    if (expectedGeneration !== generation || requestedStop)
      return
    const message = error instanceof Error && error.message === 'microphone_start_timeout'
      ? microphoneErrorMessage(error)
      : error instanceof DOMException
        ? microphoneErrorMessage(error)
        : error instanceof Error
          ? error.message
          : '无法启动 GPT-Live'
    if (reconnectAttempt > 0)
      scheduleReconnect(expectedGeneration, message)
    else
      await failTerminal(message, expectedGeneration)
  }
}

async function start(projectPath: string) {
  const normalizedPath = projectPath.trim()
  if (!normalizedPath.startsWith('/'))
    throw new Error('请先选择 Mac 上的目标项目')
  if (phase.value !== 'idle' && phase.value !== 'failed')
    return

  requestedStop = false
  reconnectAttempt = 0
  activeProjectPath.value = normalizedPath
  activeThreadId.value = null
  latestTranscript.value = ''
  transcriptRole = undefined
  transcriptFinalized = true
  explicitExecutionRequestPending = false
  executionPhase.value = 'waiting'
  taskProgressText.value = ''
  agentProgressBuffer = ''
  isMicrophoneMuted.value = false
  await beginConnection(normalizedPath)
}

async function stop() {
  requestedStop = true
  generation += 1
  sendControl('stop')
  cleanupTransport(false)
  await releaseAudioReservation()
  phase.value = 'idle'
  statusText.value = '启动全局 GPT-Live 主代理'
  latestTranscript.value = ''
  transcriptRole = undefined
  transcriptFinalized = true
  activeProjectPath.value = null
  activeThreadId.value = null
  isMicrophoneMuted.value = false
  explicitExecutionRequestPending = false
  executionPhase.value = 'waiting'
  taskProgressText.value = ''
  agentProgressBuffer = ''
  reconnectAttempt = 0
}

async function interruptCurrentConversation() {
  if (!isActive.value)
    return
  cancelRealtimeResponse()
  sendControl('interrupt')
  executionPhase.value = 'waiting'
  taskProgressText.value = ''
  agentProgressBuffer = ''
  explicitExecutionRequestPending = false
  latestTranscript.value = '已取消当前对话，GPT-Live 继续聆听'
  statusText.value = latestTranscript.value
}

function toggleMicrophoneMuted() {
  setMicrophoneMuted(!isMicrophoneMuted.value)
}

function setMicrophoneMuted(nextMuted: boolean) {
  if (!isActive.value)
    return
  mediaStream?.getAudioTracks().forEach((track) => {
    track.enabled = !nextMuted
  })
  traceLiveDiagnostics('microphone_muted_changed', {
    muted: nextMuted,
    track_count: mediaStream?.getAudioTracks().length ?? 0,
    track_enabled: mediaStream?.getAudioTracks().map(track => track.enabled) ?? [],
  })
  isMicrophoneMuted.value = nextMuted
  if (executionPhase.value === 'waiting') {
    statusText.value = nextMuted
      ? '麦克风已静音，GPT-Live 保持连接'
      : '麦克风已恢复，GPT-Live 正在聆听'
  }
}

export function useDesktopCodexLive() {
  return {
    phase,
    executionPhase,
    statusText,
    taskProgressText,
    latestTranscript,
    activeProjectPath,
    activeThreadId,
    isMicrophoneMuted,
    isActive,
    start,
    stop,
    interruptCurrentConversation,
    setMicrophoneMuted,
    toggleMicrophoneMuted,
  }
}
