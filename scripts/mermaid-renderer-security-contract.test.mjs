import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const popupContent = await readFile(
  new URL('../src/frontend/components/popup/PopupContent.vue', import.meta.url),
  'utf8',
)

test('popup Mermaid renderer uses strict security mode in the Tauri renderer', () => {
  const initializeMatch = popupContent.match(/mermaid\.initialize\(\{([\s\S]*?)\n\s*\}\)/)
  assert.ok(initializeMatch, 'missing Mermaid initialize block')

  const initializeBlock = initializeMatch[1]
  assert.match(initializeBlock, /securityLevel:\s*'strict'/)
  assert.doesNotMatch(initializeBlock, /securityLevel:\s*'loose'/)
  assert.match(initializeBlock, /flowchart:\s*\{[\s\S]*?htmlLabels:\s*false/)
})

test('Mermaid code fences escape source before rendering markdown HTML', () => {
  const mermaidBranch = popupContent.match(/if \(lang === 'mermaid'\) \{([\s\S]*?)\n\s*\}/)
  assert.ok(mermaidBranch, 'missing Mermaid markdown branch')

  assert.match(mermaidBranch[1], /\$\{escapeHtml\(str\)\}/)
})
