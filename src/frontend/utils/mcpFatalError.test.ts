/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { isIgnorableMcpFatalError } from './mcpFatalError.ts'

describe('MCP fatal error filtering', () => {
  it('ignores browser ResizeObserver loop notifications from window errors', () => {
    assert.equal(
      isIgnorableMcpFatalError('ResizeObserver loop completed with undelivered notifications.', 'window_error'),
      true,
    )

    assert.equal(
      isIgnorableMcpFatalError(new Error('ResizeObserver loop limit exceeded'), 'window_error'),
      true,
    )

    assert.equal(
      isIgnorableMcpFatalError(' ResizeObserver loop completed with undelivered notifications.\n', 'window_error'),
      true,
    )
  })

  it('does not ignore ResizeObserver messages from non-window fatal sources', () => {
    assert.equal(
      isIgnorableMcpFatalError('ResizeObserver loop completed with undelivered notifications.', 'vue_error'),
      false,
    )
  })

  it('does not ignore unrelated window errors', () => {
    assert.equal(
      isIgnorableMcpFatalError('Cannot read properties of null', 'window_error'),
      false,
    )
  })
})
