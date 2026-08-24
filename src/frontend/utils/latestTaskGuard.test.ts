/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import { createLatestTaskGuard } from './latestTaskGuard.ts'

test('a newer command invalidates an older async continuation', () => {
  const guard = createLatestTaskGuard()
  const start = guard.issue('epoch-a', 1)
  const stop = guard.issue('epoch-a', 2)

  assert.equal(guard.isCurrent(start, 'epoch-a'), false)
  assert.equal(guard.isCurrent(stop, 'epoch-a'), true)
})

test('an epoch change or owner loss invalidates pending work', () => {
  const guard = createLatestTaskGuard()
  const pending = guard.issue('epoch-a', 4)

  guard.invalidate()

  assert.equal(guard.isCurrent(pending, 'epoch-a'), false)
  assert.equal(guard.isCurrent(pending, 'epoch-b'), false)
})

test('a delayed start continuation cannot run after a newer stop', async () => {
  const guard = createLatestTaskGuard()
  const start = guard.issue('epoch-a', 1)
  let releaseOldStop!: () => void
  const oldStop = new Promise<void>((resolve) => {
    releaseOldStop = resolve
  })
  let restarted = false

  const delayedStart = (async () => {
    await oldStop
    if (!guard.isCurrent(start, 'epoch-a'))
      return
    restarted = true
  })()

  const stop = guard.issue('epoch-a', 2)
  releaseOldStop()
  await delayedStart

  assert.equal(restarted, false)
  assert.equal(guard.isCurrent(stop, 'epoch-a'), true)
})
