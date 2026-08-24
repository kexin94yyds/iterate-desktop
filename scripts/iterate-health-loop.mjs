#!/usr/bin/env node
import { execFile } from 'node:child_process'
import { appendFile, readdir, readFile, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)

const DEFAULT_LOCAL_BASE_URL = 'http://127.0.0.1:8080'
const DEFAULT_HTTP_TIMEOUT_MS = 1500
const DEFAULT_MAX_IDLE_SERVE_PER_WORKSPACE = 1
const DEFAULT_STALE_BUSY_TRANSIENT_SECS = 30
const DEFAULT_IOS_STALE_SECS = 180

const TRANSIENT_BUSY_PHASES = new Set(['starting_gui', 'starting_app', 'dispatching'])

export function summarizeConnectionStatus(connectionStatus, now = Date.now()) {
  if (!connectionStatus || typeof connectionStatus !== 'object') {
    return {
      available: false,
      localOriginHealthy: false,
      publicTunnelHealthy: false,
      publicWebSocketAuthRequired: false,
      publicWebSocketProtectedHealthy: false,
      rootTunnelHealthClass: 'unknown',
      rootBackoffActive: false,
      rootBackoffRemainingSecs: 0,
      rootHaCount: null,
      iosClientCount: 0,
      iosPrimaryMode: null,
      iosLastSeenAgeSecs: null,
      diagnosisCode: null,
    }
  }

  const clients = Array.isArray(connectionStatus.websocket?.clients)
    ? connectionStatus.websocket.clients
    : Array.isArray(connectionStatus.clients)
      ? connectionStatus.clients
      : Array.isArray(connectionStatus.active_clients)
        ? connectionStatus.active_clients
        : Array.isArray(connectionStatus.active_sessions)
          ? connectionStatus.active_sessions
          : []

  const iosClients = clients.filter((client) => {
    const kind = client.client_kind ?? client.kind ?? ''
    const mode = client.selected_transport_mode ?? client.transport_mode ?? client.mode ?? null
    return kind === 'ios' || mode != null
  })

  const modeCounts = new Map()
  let latestLastSeenAgeSecs = null
  for (const client of iosClients) {
    const mode = client.selected_transport_mode ?? client.transport_mode ?? client.mode ?? null
    if (mode) {
      modeCounts.set(mode, (modeCounts.get(mode) ?? 0) + 1)
    }
    const ageSecs = ageSeconds(client.last_seen_at, now)
    if (ageSecs != null) {
      latestLastSeenAgeSecs = latestLastSeenAgeSecs == null
        ? ageSecs
        : Math.min(latestLastSeenAgeSecs, ageSecs)
    }
  }

  const iosPrimaryMode = [...modeCounts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .at(0)?.[0] ?? null

  const publicTunnelHealthy = connectionStatus.public_tunnel?.healthy === true
  const publicWebSocketAuthRequired =
    connectionStatus.public_tunnel?.websocket_auth_required === true
    || connectionStatus.public_tunnel?.websocket?.auth_required === true
  const rootDerived = connectionStatus.root_tunnel?.derived ?? {}
  const rootMetrics = connectionStatus.root_tunnel?.metrics ?? {}
  const rootStatus = connectionStatus.root_tunnel?.status ?? {}

  return {
    available: true,
    diagnosisCode: connectionStatus.diagnosis?.code ?? null,
    localOriginHealthy: connectionStatus.local_origin?.healthy === true,
    publicTunnelHealthy,
    publicWebSocketAuthRequired,
    publicWebSocketProtectedHealthy: publicTunnelHealthy && publicWebSocketAuthRequired,
    rootTunnelHealthClass: rootDerived.tunnel_health_class ?? 'unknown',
    rootBackoffActive: rootDerived.backoff_active === true,
    rootBackoffRemainingSecs: Number(rootDerived.backoff_remaining_secs ?? 0),
    rootHaCount: rootMetrics.effective_ha_connection_count
      ?? rootStatus.ha_connection_count
      ?? null,
    iosClientCount: iosClients.length,
    iosPrimaryMode,
    iosLastSeenAgeSecs: latestLastSeenAgeSecs,
  }
}

export function classifySnapshot(snapshot, options = {}) {
  const maxIdleServePerWorkspace = Number(
    options.maxIdleServePerWorkspace ?? DEFAULT_MAX_IDLE_SERVE_PER_WORKSPACE,
  )
  const staleBusyTransientSecs = Number(
    options.staleBusyTransientSecs ?? DEFAULT_STALE_BUSY_TRANSIENT_SECS,
  )
  const iosStaleSecs = Number(options.iosStaleSecs ?? DEFAULT_IOS_STALE_SECS)
  const now = options.now ?? Date.now()
  const findings = []
  const idlePortsByWorkspace = new Map()

  for (const portInfo of snapshot.ports ?? []) {
    const workspace = portInfo.registeredWorkspace || null
    const status = portInfo.status ?? {}

    if (workspace && portInfo.healthOk === false) {
      findings.push({
        code: 'dead_port_registration',
        severity: 'warn',
        port: portInfo.port,
        workspace,
        message: `registered port ${portInfo.port} does not answer /health`,
      })
      continue
    }

    if (workspace && portInfo.healthOk === true && portInfo.statusOk === false) {
      findings.push({
        code: 'status_probe_failed',
        severity: 'warn',
        port: portInfo.port,
        workspace,
        message: `registered port ${portInfo.port} answers /health but not /status`,
      })
    }

    if (workspace && status.is_busy === false) {
      const list = idlePortsByWorkspace.get(workspace) ?? []
      list.push(portInfo.port)
      idlePortsByWorkspace.set(workspace, list)
    }

    const activeWorkspace = normalizeOptionalPath(status.active_workspace)
    const registeredWorkspace = normalizeOptionalPath(workspace)
    if (
      status.is_busy === true
      && activeWorkspace
      && registeredWorkspace
      && !isSameOrChildPath(registeredWorkspace, activeWorkspace)
    ) {
      findings.push({
        code: 'active_workspace_mismatch',
        severity: 'warn',
        port: portInfo.port,
        workspace,
        activeWorkspace: status.active_workspace,
        message: `port ${portInfo.port} is registered to ${workspace} but active for ${status.active_workspace}`,
      })
    }

    if (isStaleBusyStatus(status, staleBusyTransientSecs, now)) {
      findings.push({
        code: 'stale_busy_port',
        severity: 'warn',
        port: portInfo.port,
        workspace,
        phase: status.interaction_phase ?? null,
        activeRequestId: status.active_request_id ?? null,
        message: `port ${portInfo.port} appears stale busy`,
      })
    }
  }

  for (const [workspace, idlePorts] of idlePortsByWorkspace.entries()) {
    if (idlePorts.length > maxIdleServePerWorkspace) {
      findings.push({
        code: 'workspace_idle_serve_over_budget',
        severity: 'warn',
        workspace,
        idlePorts,
        maxIdleServePerWorkspace,
        message: `${workspace} has ${idlePorts.length} idle serve ports`,
      })
    }
  }

  const connection = snapshot.connectionStatusSummary
    ?? summarizeConnectionStatus(snapshot.connectionStatus, now)
  if (connection.available) {
    if (!connection.localOriginHealthy) {
      findings.push({
        code: 'local_origin_unhealthy',
        severity: 'error',
        message: 'local bridge origin is not healthy',
      })
    }
    if (!connection.publicTunnelHealthy) {
      findings.push({
        code: 'public_tunnel_unhealthy',
        severity: 'warn',
        message: 'public tunnel is not healthy',
      })
    }
    if (connection.rootBackoffActive) {
      findings.push({
        code: 'root_tunnel_backoff_active',
        severity: 'info',
        backoffRemainingSecs: connection.rootBackoffRemainingSecs,
        message: 'root tunnel supervisor is in backoff',
      })
    }
    if (connection.iosLastSeenAgeSecs != null && connection.iosLastSeenAgeSecs > iosStaleSecs) {
      findings.push({
        code: 'ios_last_seen_stale',
        severity: 'info',
        ageSecs: connection.iosLastSeenAgeSecs,
        thresholdSecs: iosStaleSecs,
        message: 'iOS client last_seen is stale',
      })
    }
  }

  return findings
}

export function createRecoveryPlan(findings, options = {}) {
  const destructiveActionsEnabled = options.destructiveActionsEnabled === true
  const safeRecoveryEnabled = options.safeRecoveryEnabled === true
  const actions = findings.map((finding) => {
    switch (finding.code) {
      case 'dead_port_registration':
        return {
          code: 'prune_dead_port_registration',
          mode: safeRecoveryEnabled ? 'auto' : 'dry_run',
          exec: safeRecoveryEnabled,
          port: finding.port,
          reason: finding.code,
        }
      case 'stale_busy_port':
        return {
          code: 'avoid_or_prune_stale_busy_port',
          mode: destructiveActionsEnabled ? 'auto' : 'dry_run',
          exec: false,
          port: finding.port,
          reason: finding.code,
        }
      case 'workspace_idle_serve_over_budget':
        return {
          code: 'trim_extra_idle_workspace_serve',
          mode: 'manual_confirm',
          exec: false,
          workspace: finding.workspace,
          reason: finding.code,
        }
      case 'public_tunnel_unhealthy':
        return {
          code: 'request_public_tunnel_recovery',
          mode: safeRecoveryEnabled ? 'auto' : 'dry_run',
          exec: safeRecoveryEnabled,
          reason: finding.code,
        }
      case 'local_origin_unhealthy':
        return {
          code: 'recover_bridge_origin',
          mode: 'manual_confirm',
          exec: false,
          reason: finding.code,
        }
      default:
        return {
          code: 'record_and_escalate',
          mode: 'dry_run',
          exec: false,
          reason: finding.code,
        }
    }
  })

  return {
    destructiveActionsEnabled,
    safeRecoveryEnabled,
    actions,
  }
}

export async function applyRecoveryPlan(plan, options = {}) {
  const portDir = options.portDir ?? path.join(os.homedir(), '.cunzhi_ports')
  const recoverRequestFile = options.recoverRequestFile ?? '/tmp/iterate-root-tunnel-recover.request'
  const results = []

  for (const action of plan.actions ?? []) {
    if (action.exec !== true) {
      results.push({
        code: action.code,
        status: 'skipped',
        port: action.port ?? null,
        workspace: action.workspace ?? null,
        reason: 'exec_false',
      })
      continue
    }

    try {
      if (action.code === 'prune_dead_port_registration' && action.port) {
        await rm(path.join(portDir, String(action.port)), { force: true })
        results.push({
          code: action.code,
          status: 'applied',
          port: action.port,
        })
      } else if (action.code === 'request_public_tunnel_recovery') {
        const reason = action.reason ?? 'public_tunnel_unhealthy'
        await writeFile(
          recoverRequestFile,
          `iterate_health_loop_recover=${reason}\nrequested_at=${new Date().toISOString()}\n`,
          'utf8',
        )
        results.push({
          code: action.code,
          status: 'applied',
          recoverRequestFile,
        })
      } else {
        results.push({
          code: action.code,
          status: 'skipped',
          reason: 'unsupported_action',
        })
      }
    } catch (error) {
      results.push({
        code: action.code,
        status: 'failed',
        error: String(error?.message || error),
      })
    }
  }

  return results
}

export async function collectSnapshot(options = {}) {
  const now = new Date().toISOString()
  const ports = await collectRegisteredPorts(options)
  const processes = await collectProcessSnapshot()
  const connectionStatus = await probeConnectionStatus(options)
  const connectionStatusSummary = summarizeConnectionStatus(connectionStatus)

  return {
    schema: 'iterate.health_loop.snapshot.v1',
    collectedAt: now,
    mode: 'read_only',
    ports,
    processes,
    connectionStatus,
    connectionStatusSummary,
  }
}

async function collectRegisteredPorts(options = {}) {
  const portDir = options.portDir ?? path.join(os.homedir(), '.cunzhi_ports')
  const entries = await readdir(portDir).catch(() => [])
  const ports = entries
    .map(name => Number.parseInt(name, 10))
    .filter(port => Number.isInteger(port) && port > 0)
    .sort((a, b) => a - b)

  const result = []
  for (const port of ports) {
    const filePath = path.join(portDir, String(port))
    const registeredWorkspace = (await readFile(filePath, 'utf8').catch(() => '')).trim()
    const health = await fetchJson(`http://127.0.0.1:${port}/health`, options.httpTimeoutMs)
    const status = health.ok
      ? await fetchJson(`http://127.0.0.1:${port}/status`, options.httpTimeoutMs)
      : { ok: false, error: health.error }

    result.push({
      port,
      registeredWorkspace,
      healthOk: health.ok,
      statusOk: status.ok,
      status: status.body ?? null,
      error: health.ok ? status.error ?? null : health.error ?? null,
    })
  }
  return result
}

async function collectProcessSnapshot() {
  try {
    const { stdout } = await execFileAsync('ps', ['-axo', 'pid,ppid,etime,command'], {
      maxBuffer: 1024 * 1024,
    })
    const lines = stdout.split('\n').slice(1).filter(Boolean)
    const serve = []
    const mcpServers = []
    const codexAppServers = []

    for (const line of lines) {
      const match = line.trim().match(/^(\d+)\s+(\d+)\s+(\S+)\s+(.+)$/)
      if (!match) continue
      const [, pidRaw, ppidRaw, etime, command] = match
      const item = {
        pid: Number(pidRaw),
        ppid: Number(ppidRaw),
        etime,
        command,
      }
      if (command.includes('iterate --serve --port')) {
        serve.push({
          ...item,
          port: Number(command.match(/--port\s+(\d+)/)?.[1] ?? 0) || null,
          workspace: command.match(/--workspace\s+(.+)$/)?.[1] ?? null,
        })
      } else if (command.includes('mcp-server 5311')) {
        mcpServers.push(item)
      } else if (command.includes('codex app-server')) {
        codexAppServers.push(item)
      }
    }

    return {
      serve,
      mcpServers,
      codexAppServers,
      counts: {
        serve: serve.length,
        mcpServers: mcpServers.length,
        codexAppServers: codexAppServers.length,
      },
    }
  } catch (error) {
    return {
      error: String(error?.message || error),
      serve: [],
      mcpServers: [],
      codexAppServers: [],
      counts: { serve: 0, mcpServers: 0, codexAppServers: 0 },
    }
  }
}

async function probeConnectionStatus(options = {}) {
  const localBaseUrl = options.localBaseUrl ?? DEFAULT_LOCAL_BASE_URL
  const response = await fetchJson(`${localBaseUrl.replace(/\/$/, '')}/api/connection-status`, options.httpTimeoutMs)
  return response.ok ? response.body : null
}

async function fetchJson(url, timeoutMs = DEFAULT_HTTP_TIMEOUT_MS) {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  try {
    const response = await fetch(url, {
      signal: controller.signal,
      cache: 'no-store',
    })
    const text = await response.text()
    let body = null
    try {
      body = text ? JSON.parse(text) : null
    } catch {
      return { ok: false, status: response.status, error: `invalid json from ${url}` }
    }
    return { ok: response.ok, status: response.status, body }
  } catch (error) {
    return { ok: false, error: String(error?.message || error) }
  } finally {
    clearTimeout(timer)
  }
}

function isStaleBusyStatus(status, staleBusyTransientSecs, now) {
  if (!status || status.is_busy !== true) return false

  const activeRequestId = String(status.active_request_id ?? '').trim()
  if (!activeRequestId) return true

  const phase = status.interaction_phase ?? ''
  if (phase === 'failed') return true
  if (!TRANSIENT_BUSY_PHASES.has(phase)) return false

  const age = ageSeconds(status.phase_since ?? status.busy_since, now)
  return age != null && age > staleBusyTransientSecs
}

function ageSeconds(timestamp, now = Date.now()) {
  if (!timestamp) return null
  const parsed = Date.parse(timestamp)
  if (!Number.isFinite(parsed)) return null
  return Math.max(0, Math.floor((now - parsed) / 1000))
}

function normalizeOptionalPath(value) {
  if (!value || typeof value !== 'string') return null
  return path.resolve(value)
}

function isSameOrChildPath(parentPath, candidatePath) {
  const relative = path.relative(parentPath, candidatePath)
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative))
}

function parseArgs(argv) {
  const options = {
    once: true,
    json: false,
    intervalSecs: 30,
    durationSecs: 0,
    localBaseUrl: DEFAULT_LOCAL_BASE_URL,
    httpTimeoutMs: DEFAULT_HTTP_TIMEOUT_MS,
    maxIdleServePerWorkspace: DEFAULT_MAX_IDLE_SERVE_PER_WORKSPACE,
    staleBusyTransientSecs: DEFAULT_STALE_BUSY_TRANSIENT_SECS,
    iosStaleSecs: DEFAULT_IOS_STALE_SECS,
    out: null,
    enableSafeRecovery: false,
    recoverRequestFile: '/tmp/iterate-root-tunnel-recover.request',
  }

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--json') options.json = true
    else if (arg === '--once') options.once = true
    else if (arg === '--loop') options.once = false
    else if (arg === '--local-base-url') options.localBaseUrl = argv[++i]
    else if (arg === '--port-dir') options.portDir = argv[++i]
    else if (arg === '--out') options.out = argv[++i]
    else if (arg === '--interval-secs') options.intervalSecs = Number(argv[++i])
    else if (arg === '--duration-secs') options.durationSecs = Number(argv[++i])
    else if (arg === '--http-timeout-ms') options.httpTimeoutMs = Number(argv[++i])
    else if (arg === '--enable-safe-recovery') options.enableSafeRecovery = true
    else if (arg === '--recover-request-file') options.recoverRequestFile = argv[++i]
    else if (arg === '--max-idle-serve-per-workspace') options.maxIdleServePerWorkspace = Number(argv[++i])
    else if (arg === '--stale-busy-transient-secs') options.staleBusyTransientSecs = Number(argv[++i])
    else if (arg === '--ios-stale-secs') options.iosStaleSecs = Number(argv[++i])
    else if (arg === '--help' || arg === '-h') options.help = true
    else throw new Error(`Unknown argument: ${arg}`)
  }

  return options
}

function usage() {
  return `Usage: node scripts/iterate-health-loop.mjs [options]

Read-only health snapshot for iterate MCP ports and mobile/public connectivity.

Options:
  --once                                  collect one snapshot (default)
  --loop                                  collect repeatedly
  --interval-secs <n>                     loop interval, default 30
  --duration-secs <n>                     loop duration, 0 means forever
  --json                                  print JSON instead of a short summary
  --out <path>                            append JSON snapshots to a file
  --local-base-url <url>                  default ${DEFAULT_LOCAL_BASE_URL}
  --port-dir <path>                       default ~/.cunzhi_ports
  --http-timeout-ms <n>                   default ${DEFAULT_HTTP_TIMEOUT_MS}
  --enable-safe-recovery                  execute safe P2 actions only
  --recover-request-file <path>           default /tmp/iterate-root-tunnel-recover.request
  --max-idle-serve-per-workspace <n>      default ${DEFAULT_MAX_IDLE_SERVE_PER_WORKSPACE}
  --stale-busy-transient-secs <n>         default ${DEFAULT_STALE_BUSY_TRANSIENT_SECS}
  --ios-stale-secs <n>                    default ${DEFAULT_IOS_STALE_SECS}
`
}

function formatSummary(result) {
  const severityRank = { error: 3, warn: 2, info: 1 }
  const highest = result.findings.reduce(
    (current, finding) => (severityRank[finding.severity] > severityRank[current] ? finding.severity : current),
    'info',
  )
  const lines = [
    `iterate health snapshot ${result.snapshot.collectedAt}`,
    `mode: ${result.snapshot.mode}`,
    `ports: ${result.snapshot.ports.length}, serve_processes: ${result.snapshot.processes.counts.serve}, mcp_servers: ${result.snapshot.processes.counts.mcpServers}`,
    `connection: local_origin=${result.snapshot.connectionStatusSummary.localOriginHealthy} public=${result.snapshot.connectionStatusSummary.publicTunnelHealthy} ios_clients=${result.snapshot.connectionStatusSummary.iosClientCount}`,
    `findings: ${result.findings.length} highest=${result.findings.length ? highest : 'none'}`,
  ]
  for (const finding of result.findings) {
    lines.push(`- [${finding.severity}] ${finding.code}: ${finding.message ?? ''}`)
  }
  return `${lines.join('\n')}\n`
}

export async function collectAndClassify(options) {
  const snapshot = await collectSnapshot(options)
  const findings = classifySnapshot(snapshot, options)
  const recoveryPlan = createRecoveryPlan(findings, {
    safeRecoveryEnabled: options.enableSafeRecovery === true,
  })
  const recoveryResults = await applyRecoveryPlan(recoveryPlan, options)
  return { snapshot, findings, recoveryPlan, recoveryResults }
}

async function main(argv) {
  const options = parseArgs(argv)
  if (options.help) {
    process.stdout.write(usage())
    return
  }

  const startedAt = Date.now()
  do {
    const result = await collectAndClassify(options)
    const payload = `${JSON.stringify(result)}\n`
    if (options.out) {
      await appendFile(options.out, payload)
    }
    process.stdout.write(options.json ? payload : formatSummary(result))

    if (options.once) break
    if (options.durationSecs > 0 && Date.now() - startedAt >= options.durationSecs * 1000) break
    await sleep(Math.max(1, options.intervalSecs) * 1000)
  } while (true)
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])
if (isMain) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.stack || error.message || String(error))
    process.exitCode = 1
  })
}
