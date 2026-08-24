import { bridgeFetch } from './bridgeFetch'

export type GlobalCodexLivePhase
  = | 'idle'
    | 'preparing'
    | 'connecting'
    | 'active'
    | 'reconnecting'
    | 'failed'

export type DesktopCodexLiveExecutionPhase
  = | 'waiting'
    | 'submitting'
    | 'running'
    | 'completed'
    | 'failed'

export interface DesktopCodexLiveCommand {
  action: 'start' | 'stop' | 'mute' | 'interrupt'
  project_path?: string | null
  microphone_muted?: boolean | null
}

export interface DesktopCodexLiveSnapshot {
  server_epoch: string
  revision: number
  command: DesktopCodexLiveCommand | null
  phase: GlobalCodexLivePhase
  status_text: string
  active_project_path: string | null
  active_thread_id: string | null
  microphone_muted: boolean
  updated_at_ms: number
}

export interface DesktopCodexLiveUiSnapshot {
  phase: GlobalCodexLivePhase
  execution_phase: DesktopCodexLiveExecutionPhase
  status_text: string
  task_progress_text: string
  latest_transcript: string
  active_project_path: string | null
  microphone_muted: boolean
}

export const DESKTOP_CODEX_LIVE_UI_SNAPSHOT_EVENT = 'desktop-codex-live://ui-snapshot'

export interface DesktopCodexLiveLease {
  snapshot: DesktopCodexLiveSnapshot
  granted: boolean
}

export interface DesktopCodexLiveToggleResult {
  action: 'start' | 'stop'
  snapshot: DesktopCodexLiveSnapshot
}

const CONTROL_URL = 'http://127.0.0.1:8080/api/desktop-codex-live'
const STATUS_URL = 'http://127.0.0.1:8080/api/desktop-codex-live/status'
const LEASE_URL = 'http://127.0.0.1:8080/api/desktop-codex-live/lease'

async function parseSnapshot(response: Response): Promise<DesktopCodexLiveSnapshot> {
  if (!response.ok)
    throw new Error(`desktop_codex_live_bridge_${response.status}`)
  return await response.json() as DesktopCodexLiveSnapshot
}

export async function getDesktopCodexLiveSnapshot(): Promise<DesktopCodexLiveSnapshot> {
  return await parseSnapshot(await bridgeFetch(CONTROL_URL, { cache: 'no-store' }))
}

export async function sendDesktopCodexLiveCommand(
  action: 'start' | 'stop' | 'toggle' | 'mute' | 'interrupt',
  projectPath?: string | null,
  microphoneMuted?: boolean,
): Promise<DesktopCodexLiveSnapshot> {
  return await parseSnapshot(await bridgeFetch(CONTROL_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      action,
      project_path: action === 'stop' ? null : projectPath,
      microphone_muted: action === 'mute' ? microphoneMuted : undefined,
    }),
  }))
}

export async function toggleDesktopCodexLiveMicrophone(): Promise<DesktopCodexLiveSnapshot> {
  return await sendDesktopCodexLiveCommand('mute')
}

export async function setDesktopCodexLiveMicrophoneMuted(
  microphoneMuted: boolean,
): Promise<DesktopCodexLiveSnapshot> {
  return await sendDesktopCodexLiveCommand('mute', null, microphoneMuted)
}

export async function toggleDesktopCodexLive(
  preferredProjectPath?: string | null,
): Promise<DesktopCodexLiveToggleResult> {
  const projectPath = preferredProjectPath?.trim() || null
  const snapshot = await sendDesktopCodexLiveCommand('toggle', projectPath)
  const action = snapshot.command?.action
  if (action !== 'start' && action !== 'stop')
    throw new Error('desktop_codex_live_toggle_response_invalid')

  return {
    action,
    snapshot,
  }
}

export async function reportDesktopCodexLiveStatus(input: {
  serverEpoch: string
  hostId: string
  revision: number
  phase: GlobalCodexLivePhase
  statusText: string
  activeProjectPath?: string | null
  activeThreadId?: string | null
  microphoneMuted: boolean
}): Promise<DesktopCodexLiveSnapshot> {
  return await parseSnapshot(await bridgeFetch(STATUS_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      server_epoch: input.serverEpoch,
      host_id: input.hostId,
      revision: input.revision,
      phase: input.phase,
      status_text: input.statusText,
      active_project_path: input.activeProjectPath ?? null,
      active_thread_id: input.activeThreadId ?? null,
      microphone_muted: input.microphoneMuted,
    }),
  }))
}

export async function renewDesktopCodexLiveLease(
  serverEpoch: string,
  hostId: string,
): Promise<DesktopCodexLiveLease> {
  const response = await bridgeFetch(LEASE_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ server_epoch: serverEpoch, host_id: hostId }),
  })
  if (!response.ok)
    throw new Error(`desktop_codex_live_lease_${response.status}`)
  return await response.json() as DesktopCodexLiveLease
}
