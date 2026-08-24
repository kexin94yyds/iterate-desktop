#!/usr/bin/env node
import { appendFile, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { collectAndClassify } from './iterate-health-loop.mjs'

const DEFAULT_RUN_DIR = '.cunzhi-memory/health-loop-runs'
const DEFAULT_INTERVAL_SECS = 60

export function redactHealthResult(result) {
  const snapshot = result.snapshot ?? {}
  const processes = snapshot.processes ?? {}
  const recoveryActions = result.recoveryPlan?.actions ?? []
  const recoveryResults = result.recoveryResults ?? []

  return {
    schema: 'iterate.health_loop.daemon_record.v1',
    collectedAt: snapshot.collectedAt ?? new Date().toISOString(),
    mode: snapshot.mode ?? 'read_only',
    snapshot: {
      schema: snapshot.schema ?? 'iterate.health_loop.snapshot.v1',
      collectedAt: snapshot.collectedAt ?? null,
      mode: snapshot.mode ?? 'read_only',
      ports: (snapshot.ports ?? []).map(redactPort),
      processCounts: processes.counts ?? { serve: 0, mcpServers: 0, codexAppServers: 0 },
      connectionStatusSummary: snapshot.connectionStatusSummary ?? null,
    },
    findings: (result.findings ?? []).map(redactFinding),
    recoveryPlan: {
      destructiveActionsEnabled: result.recoveryPlan?.destructiveActionsEnabled === true,
      safeRecoveryEnabled: result.recoveryPlan?.safeRecoveryEnabled === true,
      actions: recoveryActions.map(redactAction),
    },
    recoveryResults: recoveryResults.map(redactRecoveryResult),
  }
}

export function updateReceipt(receipt, record) {
  const next = receipt ?? {
    schema: 'iterate.health_loop.receipt.v1',
    startedAt: record.collectedAt,
    updatedAt: record.collectedAt,
    findings: [],
  }
  const findings = new Map(next.findings.map(finding => [finding.key, finding]))

  for (const finding of record.findings ?? []) {
    const key = findingKey(finding)
    const previous = findings.get(key)
    const lastAction = (record.recoveryPlan?.actions ?? [])
      .find(action => action.reason === finding.code && targetsMatch(action, finding)) ?? null
    const lastResult = lastAction
      ? (record.recoveryResults ?? []).find(result => result.code === lastAction.code && targetsMatch(result, lastAction)) ?? null
      : null

    if (previous) {
      previous.lastSeenAt = record.collectedAt
      previous.count += 1
      previous.lastAction = lastAction
      previous.lastResult = lastResult
      previous.severity = finding.severity
      previous.message = finding.message ?? previous.message
    } else {
      findings.set(key, {
        key,
        code: finding.code,
        severity: finding.severity,
        port: finding.port ?? null,
        workspace: finding.workspace ?? null,
        firstSeenAt: record.collectedAt,
        lastSeenAt: record.collectedAt,
        count: 1,
        message: finding.message ?? null,
        lastAction,
        lastResult,
      })
    }
  }

  next.updatedAt = record.collectedAt
  next.findings = [...findings.values()].sort((a, b) => a.key.localeCompare(b.key))
  return next
}

export async function acquireLock(lockPath, options = {}) {
  await mkdir(path.dirname(lockPath), { recursive: true })
  const pid = Number(options.pid ?? process.pid)
  const isProcessAlive = options.isProcessAlive ?? defaultIsProcessAlive
  const metadata = {
    pid,
    startedAt: new Date().toISOString(),
  }

  try {
    await writeFile(lockPath, `${JSON.stringify(metadata)}\n`, { flag: 'wx' })
  } catch (error) {
    if (error?.code !== 'EEXIST') throw error
    const existing = await readLock(lockPath)
    if (existing?.pid && isProcessAlive(existing.pid)) {
      throw new Error(`iterate health daemon already running: pid=${existing.pid}`)
    }
    await rm(lockPath, { force: true })
    await writeFile(lockPath, `${JSON.stringify(metadata)}\n`, { flag: 'wx' })
  }

  return {
    lockPath,
    release: async () => {
      const current = await readLock(lockPath)
      if (!current?.pid || current.pid === pid) {
        await rm(lockPath, { force: true })
      }
    },
  }
}

export async function runDaemon(options = {}) {
  const runDir = options.runDir ?? DEFAULT_RUN_DIR
  const lockFile = options.lockFile ?? path.join(runDir, 'daemon.lock')
  const recordsPath = options.recordsPath ?? path.join(runDir, 'records.jsonl')
  const receiptPath = options.receiptPath ?? path.join(runDir, 'receipt.json')
  const intervalSecs = Number(options.intervalSecs ?? DEFAULT_INTERVAL_SECS)
  const maxIterations = Number(options.maxIterations ?? 0)
  const durationSecs = Number(options.durationSecs ?? 0)
  const collect = options.collect ?? collectAndClassify
  const sleep = options.sleep ?? defaultSleep
  const startedAt = Date.now()
  const lock = await acquireLock(lockFile, options.lockOptions)
  let receipt = await readReceipt(receiptPath)
  let iteration = 0

  try {
    await mkdir(path.dirname(recordsPath), { recursive: true })
    await mkdir(path.dirname(receiptPath), { recursive: true })
    do {
      iteration += 1
      const result = await collect(options)
      const record = redactHealthResult(result)
      receipt = updateReceipt(receipt, record)
      await mkdir(runDir, { recursive: true })
      await appendFile(recordsPath, `${JSON.stringify(record)}\n`)
      await writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`)
      if (options.json) process.stdout.write(`${JSON.stringify(record)}\n`)

      if (options.once === true) break
      if (maxIterations > 0 && iteration >= maxIterations) break
      if (durationSecs > 0 && Date.now() - startedAt >= durationSecs * 1000) break
      await sleep(Math.max(1, intervalSecs) * 1000)
    } while (true)
  } finally {
    await lock.release()
  }

  return {
    recordsPath,
    receiptPath,
    iterations: iteration,
  }
}

function redactPort(portInfo) {
  const status = portInfo.status ?? {}
  return {
    port: portInfo.port,
    registeredWorkspace: portInfo.registeredWorkspace ?? null,
    healthOk: portInfo.healthOk === true,
    statusOk: portInfo.statusOk === true,
    isBusy: status.is_busy ?? null,
    interactionPhase: status.interaction_phase ?? null,
  }
}

function redactFinding(finding) {
  return {
    code: finding.code,
    severity: finding.severity,
    port: finding.port ?? null,
    workspace: finding.workspace ?? null,
    activeWorkspace: finding.activeWorkspace ?? null,
    phase: finding.phase ?? null,
    idlePorts: finding.idlePorts ?? null,
    thresholdSecs: finding.thresholdSecs ?? null,
    ageSecs: finding.ageSecs ?? null,
    message: finding.message ?? null,
  }
}

function redactAction(action) {
  return {
    code: action.code,
    mode: action.mode,
    exec: action.exec === true,
    port: action.port ?? null,
    workspace: action.workspace ?? null,
    reason: action.reason ?? null,
  }
}

function redactRecoveryResult(result) {
  return {
    code: result.code,
    status: result.status,
    port: result.port ?? null,
    workspace: result.workspace ?? null,
    reason: result.reason ?? null,
    error: result.error ?? null,
  }
}

function targetsMatch(left, right) {
  return targetFieldMatches(left, right, 'port') && targetFieldMatches(left, right, 'workspace')
}

function targetFieldMatches(left, right, field) {
  const leftValue = left[field] ?? null
  const rightValue = right[field] ?? null
  if (leftValue == null && rightValue == null) return true
  if (leftValue == null || rightValue == null) return false
  return leftValue === rightValue
}

function findingKey(finding) {
  return [
    finding.code,
    finding.workspace ?? '',
    finding.port ?? '',
  ].join('|')
}

async function readLock(lockPath) {
  try {
    return JSON.parse(await readFile(lockPath, 'utf8'))
  } catch {
    return null
  }
}

async function readReceipt(receiptPath) {
  try {
    return JSON.parse(await readFile(receiptPath, 'utf8'))
  } catch {
    return null
  }
}

function defaultIsProcessAlive(pid) {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

function defaultSleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function parseArgs(argv) {
  const options = {
    runDir: DEFAULT_RUN_DIR,
    intervalSecs: DEFAULT_INTERVAL_SECS,
    durationSecs: 0,
    maxIterations: 0,
    once: false,
    json: false,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--run-dir') options.runDir = argv[++i]
    else if (arg === '--lock-file') options.lockFile = argv[++i]
    else if (arg === '--records-path') options.recordsPath = argv[++i]
    else if (arg === '--receipt-path') options.receiptPath = argv[++i]
    else if (arg === '--interval-secs') options.intervalSecs = Number(argv[++i])
    else if (arg === '--duration-secs') options.durationSecs = Number(argv[++i])
    else if (arg === '--max-iterations') options.maxIterations = Number(argv[++i])
    else if (arg === '--once') options.once = true
    else if (arg === '--json') options.json = true
    else if (arg === '--enable-safe-recovery') options.enableSafeRecovery = true
    else if (arg === '--local-base-url') options.localBaseUrl = argv[++i]
    else if (arg === '--port-dir') options.portDir = argv[++i]
    else if (arg === '--http-timeout-ms') options.httpTimeoutMs = Number(argv[++i])
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
  return `Usage: node scripts/iterate-health-daemon.mjs [options]

P3 health-loop daemon wrapper. Writes redacted records and a compact receipt.

Options:
  --run-dir <path>                       default ${DEFAULT_RUN_DIR}
  --lock-file <path>                     default <run-dir>/daemon.lock
  --records-path <path>                  default <run-dir>/records.jsonl
  --receipt-path <path>                  default <run-dir>/receipt.json
  --interval-secs <n>                    default ${DEFAULT_INTERVAL_SECS}
  --duration-secs <n>                    default 0, forever
  --max-iterations <n>                   default 0, unlimited
  --once                                 collect one daemon record and exit
  --json                                 also print each redacted record
  --enable-safe-recovery                 pass through safe P2 recovery only
  --local-base-url <url>                 pass through to health loop
  --port-dir <path>                      pass through to health loop
  --http-timeout-ms <n>                  pass through to health loop
  --recover-request-file <path>          pass through to health loop
  --max-idle-serve-per-workspace <n>     pass through to health loop
  --stale-busy-transient-secs <n>        pass through to health loop
  --ios-stale-secs <n>                   pass through to health loop
`
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])
if (isMain) {
  const options = parseArgs(process.argv.slice(2))
  if (options.help) {
    process.stdout.write(usage())
  } else {
    runDaemon(options).catch((error) => {
      console.error(error.stack || error.message || String(error))
      process.exitCode = 1
    })
  }
}
