import type { UnlistenFn } from '@tauri-apps/api/event'
import type {
  DesktopCodexLiveSnapshot,
} from '../services/desktopCodexLiveControl'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ref } from 'vue'
import {
  renewDesktopCodexLiveLease,
  reportDesktopCodexLiveStatus,
  setDesktopCodexLiveMicrophoneMuted,
  toggleDesktopCodexLive,
} from '../services/desktopCodexLiveControl'
import { createLatestTaskGuard } from '../utils/latestTaskGuard'
import { useDesktopCodexLive } from './useDesktopCodexLive'

const HOST_POLL_INTERVAL_MS = 250
const HEARTBEAT_INTERVAL_MS = 1500
const LAST_PROJECT_STORAGE_KEY = 'iterate.desktop-codex-live.last-project'

function createHostId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID()
  return `host-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

async function runtimeLog(event: string, details: Record<string, unknown> = {}) {
  try {
    await invoke('debug_log', {
      message: `[GPT-Live Host] ${event} ${JSON.stringify(details)}`,
    })
  }
  catch {
    // Runtime logging must never block the global audio host.
  }
}

/**
 * Owns the desktop GPT-Live transport in the canonical speech-overlay WebView.
 *
 * The overlay is independent from AppContent and popup lifetime. It is revealed
 * before getUserMedia so WebKit can start the microphone, while the authenticated
 * bridge lease still elects at most one live host.
 */
export function useDesktopCodexLiveHost() {
  const live = useDesktopCodexLive()
  const hostWindow = getCurrentWindow()
  const hostId = createHostId()
  const commandGuard = createLatestTaskGuard()
  const ownsLease = ref(false)
  const lastProjectPath = ref(localStorage.getItem(LAST_PROJECT_STORAGE_KEY) || '')
  let initialized = false
  let disposed = false
  let isOwner = false
  let pollTimer: number | null = null
  let pollInFlight = false
  let statusInFlight = false
  let lastHeartbeatAt = 0
  let lastProcessedRevision = 0
  let serverEpoch = ''
  let unlistenFnAction: UnlistenFn | null = null
  let lifecycleRequestInFlight = false
  let muteRequestInFlight = false

  async function applyEpoch(snapshot: DesktopCodexLiveSnapshot) {
    if (serverEpoch === snapshot.server_epoch)
      return
    commandGuard.invalidate()
    if (isOwner)
      await demoteOwner()
    serverEpoch = snapshot.server_epoch
    lastProcessedRevision = 0
    lastHeartbeatAt = 0
    void runtimeLog('server_epoch_changed', { serverEpoch })
  }

  async function demoteOwner() {
    const previousOwner = isOwner
    isOwner = false
    ownsLease.value = false
    commandGuard.invalidate()

    // stop() tears down the microphone, peer, and socket synchronously before
    // its first await. Do that before any window IPC so owner handoff cannot
    // leave two live transports running while hide() is pending.
    const stopPromise = previousOwner && live.isActive.value
      ? live.stop()
      : null

    try {
      await hostWindow.hide()
    }
    catch (error) {
      void runtimeLog('host_visibility_failed', { nextOwner: false, error: String(error) })
    }

    if (stopPromise)
      await stopPromise

    if (previousOwner) {
      void runtimeLog('owner_transition', {
        previousOwner,
        nextOwner: false,
        hostId,
      })
    }
  }

  async function keepHostHidden() {
    try {
      // The canonical speech overlay remains hidden while idle. Reserving the
      // microphone reveals this same window before WebKit calls getUserMedia.
      await hostWindow.hide()
      return true
    }
    catch (error) {
      void runtimeLog('host_visibility_failed', { nextOwner: true, error: String(error) })
      return false
    }
  }

  function promoteOwner() {
    const previousOwner = isOwner
    isOwner = true
    ownsLease.value = true
    commandGuard.invalidate()
    void runtimeLog('owner_transition', {
      previousOwner,
      nextOwner: true,
      hostId,
    })
  }

  async function publishStatus(revision = lastProcessedRevision) {
    if (!serverEpoch || statusInFlight || disposed)
      return false
    statusInFlight = true
    const requestEpoch = serverEpoch
    try {
      const snapshot = await reportDesktopCodexLiveStatus({
        serverEpoch: requestEpoch,
        hostId,
        revision,
        phase: live.phase.value,
        statusText: live.statusText.value,
        activeProjectPath: live.activeProjectPath.value,
        activeThreadId: live.activeThreadId.value,
        microphoneMuted: live.isMicrophoneMuted.value,
      })
      // A response from the previous bridge instance may arrive after a newer
      // lease poll has already advanced this host to the replacement epoch.
      if (serverEpoch === requestEpoch || snapshot.server_epoch === serverEpoch) {
        await applyEpoch(snapshot)
      }
      lastHeartbeatAt = Date.now()
      return snapshot.server_epoch === serverEpoch && isOwner
    }
    catch (error) {
      await runtimeLog('status_publish_failed', { error: String(error) })
      return false
    }
    finally {
      statusInFlight = false
    }
  }

  function runCommand(snapshot: DesktopCodexLiveSnapshot) {
    if (!isOwner || snapshot.revision <= lastProcessedRevision)
      return

    lastProcessedRevision = snapshot.revision
    const commandToken = commandGuard.issue(serverEpoch, snapshot.revision)
    const command = snapshot.command
    void runtimeLog('command_received', {
      revision: snapshot.revision,
      action: command?.action ?? null,
      projectPath: command?.project_path ?? null,
    })

    let operation: Promise<void> | null = null
    if (command?.action === 'start' && command.project_path) {
      lastProjectPath.value = command.project_path
      localStorage.setItem(LAST_PROJECT_STORAGE_KEY, command.project_path)
      operation = (async () => {
        if (live.isActive.value && live.activeProjectPath.value !== command.project_path)
          await live.stop()
        if (!isCurrentCommand(commandToken))
          return
        if (!live.isActive.value)
          await live.start(command.project_path)
      })()
    }
    else if (command?.action === 'stop') {
      operation = live.stop()
    }
    else if (command?.action === 'interrupt') {
      operation = live.interruptCurrentConversation()
    }
    else if (command?.action === 'mute') {
      live.setMicrophoneMuted(command.microphone_muted ?? !live.isMicrophoneMuted.value)
    }

    // start() enters `preparing` synchronously before waiting for microphone
    // permission. Publish that state immediately, then keep the regular
    // heartbeat alive while the long-running operation continues.
    void publishStatus()
    if (operation) {
      void operation
        .catch(error => runtimeLog('command_failed', {
          revision: snapshot.revision,
          error: String(error),
        }))
        .finally(() => {
          if (isCurrentCommand(commandToken))
            return publishStatus()
        })
    }
  }

  function isCurrentCommand(token: { generation: number, scope: string, revision: number }) {
    return !disposed
      && isOwner
      && lastProcessedRevision === token.revision
      && commandGuard.isCurrent(token, serverEpoch)
  }

  async function poll() {
    if (pollInFlight || disposed)
      return
    pollInFlight = true
    try {
      const requestedEpoch = serverEpoch
      let lease = await renewDesktopCodexLiveLease(requestedEpoch, hostId)
      await applyEpoch(lease.snapshot)
      if (!lease.granted && serverEpoch !== requestedEpoch) {
        lease = await renewDesktopCodexLiveLease(serverEpoch, hostId)
        await applyEpoch(lease.snapshot)
      }
      if (!lease.granted) {
        await demoteOwner()
        return
      }

      if (!isOwner) {
        if (!await keepHostHidden()) {
          await demoteOwner()
          return
        }

        // Native window IPC is asynchronous. Renew after it completes so a
        // slow AppKit transition cannot let an expired owner run a command.
        const confirmedLease = await renewDesktopCodexLiveLease(serverEpoch, hostId)
        await applyEpoch(confirmedLease.snapshot)
        if (!confirmedLease.granted) {
          await demoteOwner()
          return
        }
        lease = confirmedLease
        promoteOwner()
      }

      runCommand(lease.snapshot)
      if (Date.now() - lastHeartbeatAt >= HEARTBEAT_INTERVAL_MS)
        await publishStatus()
    }
    catch (error) {
      await runtimeLog('poll_failed', { error: String(error) })
      // Keep non-owners hidden until a later poll proves ownership.
      if (!isOwner)
        await demoteOwner()
    }
    finally {
      pollInFlight = false
    }
  }

  async function initialize() {
    if (initialized)
      return
    initialized = true
    disposed = false
    await runtimeLog('initialize', { hostId })
    try {
      unlistenFnAction = await listen<string>('desktop-codex-live-fn-action', (event) => {
        if (event.payload === 'toggle')
          void toggleLive()
        else if (event.payload === 'mute')
          void requestMicrophoneMuteToggle()
      })
    }
    catch (error) {
      await runtimeLog('fn_action_listener_failed', { error: String(error) })
    }
    await poll()
    if (disposed)
      return
    pollTimer = window.setInterval(() => void poll(), HOST_POLL_INTERVAL_MS)
  }

  async function dispose() {
    disposed = true
    unlistenFnAction?.()
    unlistenFnAction = null
    commandGuard.invalidate()
    if (pollTimer !== null)
      window.clearInterval(pollTimer)
    pollTimer = null
    await demoteOwner()
    initialized = false
    await runtimeLog('dispose')
  }

  async function toggleLive() {
    if (disposed || lifecycleRequestInFlight)
      return
    lifecycleRequestInFlight = true
    const projectPath = lastProjectPath.value.trim()
    try {
      const result = await toggleDesktopCodexLive(projectPath)
      await applyEpoch(result.snapshot)
      runCommand(result.snapshot)
    }
    catch (error) {
      const details = error instanceof Error ? error.message : String(error)
      live.statusText.value = details === 'desktop_codex_live_bridge_400'
        ? '请先在 iterate 顶部选择目标项目并启动一次'
        : '无法切换全局 GPT-Live，请重试'
      await runtimeLog('hud_toggle_failed', { error: details })
      await publishStatus()
    }
    finally {
      lifecycleRequestInFlight = false
    }
  }

  async function requestMicrophoneMuteToggle() {
    if (!live.isActive.value || disposed || muteRequestInFlight)
      return
    muteRequestInFlight = true
    const previousMuted = live.isMicrophoneMuted.value
    const nextMuted = !previousMuted
    live.setMicrophoneMuted(nextMuted)
    try {
      const snapshot = await setDesktopCodexLiveMicrophoneMuted(nextMuted)
      await applyEpoch(snapshot)
      runCommand(snapshot)
    }
    catch (error) {
      if (live.isMicrophoneMuted.value === nextMuted)
        live.setMicrophoneMuted(previousMuted)
      await runtimeLog('hud_mute_failed', { error: String(error) })
    }
    finally {
      muteRequestInFlight = false
    }
  }

  return {
    live,
    ownsLease,
    lastProjectPath,
    initialize,
    dispose,
    toggleLive,
    requestMicrophoneMuteToggle,
  }
}
