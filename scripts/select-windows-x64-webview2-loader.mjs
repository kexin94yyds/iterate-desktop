#!/usr/bin/env node

import { readdir, readFile } from 'node:fs/promises'
import { basename, relative, resolve, sep } from 'node:path'

const IMAGE_FILE_MACHINE_AMD64 = 0x8664
const searchRoot = resolve(process.argv[2] ?? 'target')

async function collectCandidates(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const candidates = []

  for (const entry of entries) {
    const entryPath = resolve(directory, entry.name)
    if (entry.isDirectory()) {
      candidates.push(...await collectCandidates(entryPath))
    }
    else if (entry.isFile() && basename(entryPath).toLowerCase() === 'webview2loader.dll') {
      candidates.push(entryPath)
    }
  }

  return candidates
}

async function readPeMachine(filePath) {
  const bytes = await readFile(filePath)
  if (bytes.length < 64 || bytes[0] !== 0x4d || bytes[1] !== 0x5a) {
    throw new Error('missing DOS MZ header')
  }

  const peOffset = bytes.readUInt32LE(0x3c)
  if (peOffset + 6 > bytes.length || bytes.toString('ascii', peOffset, peOffset + 4) !== 'PE\0\0') {
    throw new Error('missing PE signature')
  }

  return bytes.readUInt16LE(peOffset + 4)
}

function normalizedRelativePath(filePath) {
  return relative(process.cwd(), filePath).split(sep).join('/')
}

function preference(filePath) {
  const normalized = filePath.split(sep).join('/').toLowerCase()
  if (normalized.endsWith('/target/release/webview2loader.dll'))
    return 0
  if (normalized.includes('/target/release/') && normalized.includes('/x64/'))
    return 1
  if (normalized.includes('/x64/'))
    return 2
  if (normalized.includes('/target/release/'))
    return 3
  return 4
}

let candidates
try {
  candidates = await collectCandidates(searchRoot)
}
catch (error) {
  console.error(`Unable to scan WebView2 loader root ${searchRoot}: ${error.message}`)
  process.exit(1)
}

const inspected = []
for (const filePath of candidates) {
  try {
    inspected.push({ filePath, machine: await readPeMachine(filePath) })
  }
  catch (error) {
    inspected.push({ filePath, error: error.message })
  }
}

const selected = inspected
  .filter(candidate => candidate.machine === IMAGE_FILE_MACHINE_AMD64)
  .sort((left, right) => preference(left.filePath) - preference(right.filePath)
    || left.filePath.localeCompare(right.filePath))[0]

if (!selected) {
  console.error('No x86-64 WebView2Loader.dll was found. Inspected candidates:')
  for (const candidate of inspected) {
    const result = candidate.error ?? `PE machine 0x${candidate.machine.toString(16).padStart(4, '0')}`
    console.error(`- ${normalizedRelativePath(candidate.filePath)}: ${result}`)
  }
  process.exit(1)
}

console.log(normalizedRelativePath(selected.filePath))
