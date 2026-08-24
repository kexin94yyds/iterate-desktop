#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, renameSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'

function fail(message) {
  throw new Error(message)
}

function parseArgs(argv) {
  const options = { verify: false }
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index]
    if (value === '--verify') {
      options.verify = true
      continue
    }
    if (value === '--repo-root' || value === '--app') {
      const next = argv[index + 1]
      if (!next)
        fail(`${value} requires a path`)
      options[value === '--repo-root' ? 'repoRoot' : 'appPath'] = path.resolve(next)
      index += 1
      continue
    }
    fail(`unknown option: ${value}`)
  }
  if (!options.repoRoot)
    fail('--repo-root is required')
  if (!options.appPath)
    fail('--app is required')
  return options
}

function git(repoRoot, args, encoding = 'utf8') {
  return execFileSync('git', ['-C', repoRoot, ...args], {
    encoding,
    stdio: ['ignore', 'pipe', 'pipe'],
  })
}

function currentBranch(repoRoot) {
  try {
    return git(repoRoot, ['symbolic-ref', '--short', '-q', 'HEAD']).trim()
  }
  catch {
    return null
  }
}

function worktreeSnapshot(repoRoot) {
  const files = git(repoRoot, ['ls-files', '-co', '--exclude-standard', '-z'], 'buffer')
    .toString('utf8')
    .split('\0')
    .filter(Boolean)
    .sort()
  const hash = createHash('sha256')
  for (const relativePath of files) {
    const absolutePath = path.join(repoRoot, relativePath)
    hash.update(relativePath)
    hash.update('\0')
    if (existsSync(absolutePath) && statSync(absolutePath).isFile())
      hash.update(readFileSync(absolutePath))
    hash.update('\0')
  }
  return hash.digest('hex')
}

function buildReceipt(repoRoot) {
  const status = git(repoRoot, ['status', '--porcelain=v1', '-z', '--untracked-files=all'])
  const dirty = status.length > 0
  if (dirty && process.env.CUNZHI_MACOS_ALLOW_DIRTY_SOURCE !== '1') {
    fail('refusing to create a source receipt from a dirty worktree; set CUNZHI_MACOS_ALLOW_DIRTY_SOURCE=1 only for an intentional dirty development build')
  }
  const snapshot = worktreeSnapshot(repoRoot)
  return {
    schema_version: 1,
    product: 'iterate',
    generated_at: new Date().toISOString(),
    source_branch: currentBranch(repoRoot),
    source_commit: git(repoRoot, ['rev-parse', 'HEAD']).trim(),
    source_tree: git(repoRoot, ['rev-parse', 'HEAD^{tree}']).trim(),
    dirty,
    dirty_snapshot_sha256: dirty ? createHash('sha256').update(status).update(snapshot).digest('hex') : null,
    worktree_snapshot_sha256: snapshot,
  }
}

function receiptPath(appPath) {
  return path.join(appPath, 'Contents', 'Resources', 'cunzhi-source-receipt.json')
}

function comparable(receipt) {
  return {
    schema_version: receipt.schema_version,
    product: receipt.product,
    source_branch: receipt.source_branch,
    source_commit: receipt.source_commit,
    source_tree: receipt.source_tree,
    dirty: receipt.dirty,
    dirty_snapshot_sha256: receipt.dirty_snapshot_sha256,
    worktree_snapshot_sha256: receipt.worktree_snapshot_sha256,
  }
}

function writeReceipt(appPath, receipt) {
  const output = receiptPath(appPath)
  mkdirSync(path.dirname(output), { recursive: true })
  const temporary = `${output}.tmp-${process.pid}`
  writeFileSync(temporary, `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o644 })
  renameSync(temporary, output)
  return output
}

function verifyReceipt(appPath, expected) {
  const input = receiptPath(appPath)
  if (!existsSync(input))
    fail(`missing source receipt: ${input}`)
  const actual = JSON.parse(readFileSync(input, 'utf8'))
  const actualComparable = comparable(actual)
  const expectedComparable = comparable(expected)
  for (const key of Object.keys(expectedComparable)) {
    if (actualComparable[key] !== expectedComparable[key])
      fail(`source receipt mismatch for ${key}: expected ${expectedComparable[key]}, received ${actualComparable[key]}`)
  }
  return { input, receipt: actual }
}

const options = parseArgs(process.argv.slice(2))
if (!existsSync(options.appPath))
  fail(`missing app bundle: ${options.appPath}`)

const expected = buildReceipt(options.repoRoot)
if (options.verify) {
  const verified = verifyReceipt(options.appPath, expected)
  process.stdout.write(`${JSON.stringify({ status: 'verified', path: verified.input, receipt: verified.receipt })}\n`)
}
else {
  const output = writeReceipt(options.appPath, expected)
  process.stdout.write(`${JSON.stringify({ status: 'written', path: output, receipt: expected })}\n`)
}
