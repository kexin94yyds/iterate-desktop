import assert from 'node:assert/strict'
import { hasOpenModifier } from './clickModifiers.ts'
import {
  buildOpenLocalPathInvokeArgs,
  isOutsideCurrentProject,
  isPotentialLocalMarkdownHref,
  resolveLocalMarkdownHref,
} from './localMarkdownLinks.ts'

const projectPath = '/Users/test/project'

assert.equal(isPotentialLocalMarkdownHref('https://example.com'), false)
assert.equal(isPotentialLocalMarkdownHref('mailto:test@example.com'), false)
assert.equal(isPotentialLocalMarkdownHref('#section'), false)
assert.equal(isPotentialLocalMarkdownHref('javascript:alert(1)'), false)
assert.equal(isPotentialLocalMarkdownHref('file:///Users/test/project/README.md'), true)
assert.equal(isPotentialLocalMarkdownHref('/Users/test/project/README.md'), true)
assert.equal(isPotentialLocalMarkdownHref('src/frontend/App.vue'), true)

assert.deepEqual(
  resolveLocalMarkdownHref('file:///Users/test/project/README%20copy.md', projectPath),
  { path: '/Users/test/project/README copy.md' },
)

assert.equal(
  isOutsideCurrentProject({ path: '/Users/test/project/src/main.rs:12' }, projectPath),
  false,
)

assert.equal(
  isOutsideCurrentProject({ path: '/Users/test/other-project/README.md' }, projectPath),
  true,
)

assert.equal(
  isOutsideCurrentProject({ path: '/Users/test/project/../other-project/README.md' }, projectPath),
  true,
)

assert.deepEqual(
  resolveLocalMarkdownHref('/Users/test/project/src/main.rs:12', projectPath),
  { path: '/Users/test/project/src/main.rs:12' },
)

assert.deepEqual(
  resolveLocalMarkdownHref('./src/frontend/App.vue', projectPath),
  { path: '/Users/test/project/src/frontend/App.vue' },
)

assert.deepEqual(
  resolveLocalMarkdownHref('../outside.md', projectPath),
  { path: '/Users/test/project/../outside.md' },
)

assert.deepEqual(
  buildOpenLocalPathInvokeArgs(
    { path: '/Users/test/project/src/main.rs:12:3' },
    projectPath,
    { metaKey: true, ctrlKey: false },
  ),
  {
    path: '/Users/test/project/src/main.rs:12:3',
    projectPath,
    preferEditor: true,
  },
)

assert.equal(hasOpenModifier({ metaKey: true, ctrlKey: false }), true)
assert.equal(hasOpenModifier({ metaKey: false, ctrlKey: true }), true)
assert.equal(hasOpenModifier({ metaKey: false, ctrlKey: false }), false)

assert.equal(resolveLocalMarkdownHref('src/frontend/App.vue', null), null)
assert.equal(resolveLocalMarkdownHref('https://example.com', projectPath), null)
