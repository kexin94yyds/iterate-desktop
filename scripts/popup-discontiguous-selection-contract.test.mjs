import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const popupContent = readFileSync(
  new URL('../src/frontend/components/popup/PopupContent.vue', import.meta.url),
  'utf8',
)

test('ordinary drag replaces and Command-drag appends managed ranges', () => {
  assert.match(popupContent, /handleSelectionStart\(event: MouseEvent, source: PopupTextSelectionSource\)/)
  assert.match(popupContent, /event\.metaKey && !sourceChanged/)
  assert.match(popupContent, /event\.currentTarget\.focus\(\{ preventScroll: true \}\)/)
  assert.match(popupContent, /range\.cloneRange\(\)/)
  assert.match(popupContent, /rangesAreEqual\(existing\.range, item\.range\)/)
})

test('native selection remains visible without a managed highlight overlay', () => {
  const captureSelection = popupContent.match(
    /function captureTextSelection\([\s\S]*?(?=\nfunction handleRequestTextSelection)/,
  )?.[0]

  assert.ok(captureSelection)
  assert.doesNotMatch(captureSelection, /removeAllRanges\(\)/)
  assert.doesNotMatch(popupContent, /range\.getClientRects\(\)/)
  assert.doesNotMatch(popupContent, /managed-selection-highlight/)
  assert.match(popupContent, /function clearManagedSelection[\s\S]*?removeAllRanges\(\)/)
})

test('copy and quote actions prefer managed fragments and expose live feedback', () => {
  assert.match(popupContent, /resolveManagedTextSelection\(\)/)
  assert.match(popupContent, /document\.addEventListener\('copy', handleDocumentCopy\)/)
  assert.match(popupContent, /复制选中 \(\$\{selectedFragmentCount\.value\}\)/)
  assert.match(popupContent, /引用选中 \(\$\{selectedFragmentCount\.value\}\)/)
})

test('request changes and unmount clear ranges and listeners', () => {
  assert.match(popupContent, /watch\(\(\) => props\.request, \(\) => \{\s+clearManagedSelection\(\)/)
  assert.match(popupContent, /document\.removeEventListener\('copy', handleDocumentCopy\)/)
  assert.doesNotMatch(popupContent, /refreshSelectionHighlights/)
})

test('image clicks keep rendered Markdown stable and open an intentional preview', () => {
  assert.match(popupContent, /const renderedDisplayMessage = computed\(\(\) => renderMarkdown\(displayMessage\.value\)\)/)
  assert.match(popupContent, /v-html="renderedDisplayMessage"/)
  assert.match(popupContent, /v-html="renderedBrowserAiResponse"/)
  assert.match(popupContent, /if \(target\?\.closest\('img'\)\) \{\s+event\.preventDefault\(\)\s+return/)
  assert.match(popupContent, /previewImageSrc\.value = image\.currentSrc \|\| image\.src/)
  assert.match(popupContent, /<NImagePreview/)
  assert.match(popupContent, /@close="closeImagePreview"/)
  assert.match(popupContent, /@update:show="handleImagePreviewShowChange"/)
})
