import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '..')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

function walkFiles(dir, extensions) {
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  return entries.flatMap((entry) => {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory())
      return walkFiles(fullPath, extensions)
    return extensions.has(path.extname(entry.name)) ? [fullPath] : []
  })
}

function mcpDynamicClassTokens() {
  const rustSource = read('src/rust/mcp/commands.rs')
  const classValues = [...rustSource.matchAll(/\b(?:icon|icon_bg|dark_icon_bg):\s*"([^"]+)"/g)]
    .map(match => match[1])

  return [...new Set(classValues.flatMap(value => value.split(/\s+/).filter(Boolean)))].sort()
}

function unoScannedSource() {
  const frontendFiles = walkFiles(path.join(repoRoot, 'src/frontend'), new Set([
    '.css',
    '.html',
    '.js',
    '.ts',
    '.vue',
  ]))

  return [
    read('uno.config.ts'),
    ...frontendFiles.map(file => fs.readFileSync(file, 'utf8')),
  ].join('\n')
}

test('MCP tool card classes returned from Rust are visible to UnoCSS', () => {
  const scannedSource = unoScannedSource()
  const missingTokens = mcpDynamicClassTokens()
    .filter(token => !scannedSource.includes(token))

  assert.deepEqual(missingTokens, [])
})

test('MCP Carbon icon names exist in the installed icon collection', () => {
  const carbonIcons = JSON.parse(read('node_modules/@iconify-json/carbon/icons.json')).icons
  const missingIcons = mcpDynamicClassTokens()
    .filter(token => token.startsWith('i-carbon-'))
    .map(token => token.slice('i-carbon-'.length))
    .filter(name => !carbonIcons[name])

  assert.deepEqual(missingIcons, [])
})
