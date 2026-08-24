import { readFileSync } from 'node:fs'
import assert from 'node:assert/strict'

const source = readFileSync('src/frontend/test/components/McpPopupTest.vue', 'utf8')

assert.match(
  source,
  /const (?:mockAppConfig|appConfig) = computed\(/,
  'McpPopup test harness must provide the real appConfig prop shape',
)

assert.match(
  source,
  /<McpPopup\b[^>]*:app-config="(?:mockAppConfig|appConfig)"/,
  'McpPopup test harness must pass appConfig to the real component',
)

assert.doesNotMatch(
  source,
  /<McpPopup\b[^>]*:current-theme=/,
  'McpPopup test harness must not use the removed current-theme prop',
)
