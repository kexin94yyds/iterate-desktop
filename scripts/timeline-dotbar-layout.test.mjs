import { existsSync, readFileSync } from 'node:fs'
import assert from 'node:assert/strict'

const roots = ['src/frontend', 'ios-bridge-dev/src/frontend']

for (const root of roots) {
  if (!existsSync(root))
    continue

  const popup = readFileSync(`${root}/components/popup/McpPopup.vue`, 'utf8')
  const dotbar = readFileSync(`${root}/components/conversation/TimelineDotBar.vue`, 'utf8')

  assert.match(
    popup,
    /class="flex flex-1 min-h-0 overflow-hidden"/,
    `${root}: popup scroll row must be height-constrained`,
  )
  assert.match(
    popup,
    /class="[^"]*timeline-dot-column[^"]*flex-shrink-0[^"]*self-stretch[^"]*min-h-0[^"]*overflow-hidden[^"]*"/,
    `${root}: timeline column must clip to the popup content row`,
  )
  assert.match(
    dotbar,
    /\.timeline-dot-bar\s*\{[^}]*height:\s*100%;[^}]*max-height:\s*none;/s,
    `${root}: dot bar must fill, not expand beyond, its clipped parent`,
  )
}
