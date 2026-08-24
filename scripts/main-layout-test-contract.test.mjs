import { readFileSync } from 'node:fs'
import assert from 'node:assert/strict'

const source = readFileSync('src/frontend/test/components/MainLayoutTest.vue', 'utf8')

assert.match(source, /const windowWidth = ref\(/, 'MainLayout test harness must define windowWidth')
assert.match(source, /const windowHeight = ref\(/, 'MainLayout test harness must define windowHeight')
assert.match(source, /const fixedWindowSize = ref\(/, 'MainLayout test harness must define fixedWindowSize')

const mainLayoutUsages = source.match(/<MainLayout[\s\S]*?\/>/g) ?? []
assert.equal(mainLayoutUsages.length, 2, 'expected controlled and pure MainLayout test usages')

for (const usage of mainLayoutUsages) {
  assert.match(usage, /:window-width="windowWidth"/, 'MainLayout usage must pass windowWidth')
  assert.match(usage, /:window-height="windowHeight"/, 'MainLayout usage must pass windowHeight')
  assert.match(usage, /:fixed-window-size="fixedWindowSize"/, 'MainLayout usage must pass fixedWindowSize')
}
