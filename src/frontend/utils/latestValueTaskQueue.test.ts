/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import { createLatestValueTaskQueue } from './latestValueTaskQueue.ts'

test('applies a newer collapse after an older delayed expand', async () => {
  const applied: boolean[] = []
  let releaseExpand!: () => void
  const expandGate = new Promise<void>((resolve) => {
    releaseExpand = resolve
  })
  const queue = createLatestValueTaskQueue<boolean>(async (expanded) => {
    if (expanded)
      await expandGate
    applied.push(expanded)
  })

  const expanding = queue.request(true)
  await Promise.resolve()
  const collapsing = queue.request(false)
  releaseExpand()
  await Promise.all([expanding, collapsing])

  assert.deepEqual(applied, [true, false])
  assert.equal(applied.at(-1), false)
})

test('coalesces intermediate values while preserving the latest request', async () => {
  const applied: string[] = []
  let releaseFirst!: () => void
  const firstGate = new Promise<void>((resolve) => {
    releaseFirst = resolve
  })
  const queue = createLatestValueTaskQueue<string>(async (value) => {
    if (value === 'first')
      await firstGate
    applied.push(value)
  })

  const first = queue.request('first')
  await Promise.resolve()
  void queue.request('middle')
  const last = queue.request('last')
  releaseFirst()
  await Promise.all([first, last])

  assert.deepEqual(applied, ['first', 'last'])
})
