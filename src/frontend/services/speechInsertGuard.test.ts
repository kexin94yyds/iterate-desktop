/* eslint-disable test/no-import-node-test */
import type { SpeechLayerIdentity, SpeechSnapshot } from './globalSpeechSession.ts'
import type { SpeechInsertPayload } from './speechInsertGuard.ts'
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { SpeechInsertGuard } from './speechInsertGuard.ts'

function identity(epoch = 1, control = 2, session = 3, revision = 4): SpeechLayerIdentity {
  return {
    schema_version: '1',
    owner_epoch_hi: String(epoch),
    owner_epoch_lo: String(epoch),
    control_seq: String(control),
    session_sequence: String(session),
    revision: String(revision),
  }
}

function snapshot(current = identity()): SpeechSnapshot {
  return {
    owner_epoch: Array.from({ length: 16 }, () => Number(current.owner_epoch_hi)),
    desired_state: 'Off',
    phase: 'Committing',
    visible: false,
    control_seq: Number(current.control_seq),
    session: Number(current.session_sequence),
    identity: current,
    partial_len: 0,
    writeback_outcome: 'Dispatched',
  }
}

function payload(overrides: Partial<SpeechInsertPayload> = {}): SpeechInsertPayload {
  return {
    identity: identity(),
    request_id: 'request-1',
    window_label: 'main',
    text: 'hello',
    mode: 'final',
    insert_id: 'insert-1',
    ...overrides,
  }
}

describe('speech insert guard', () => {
  it('requires the complete current identity, request, window, mode, and insert id', () => {
    const guard = new SpeechInsertGuard()
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    assert.equal(guard.classify(payload()), 'apply')

    for (const rejected of [
      payload({ identity: identity(9) }),
      payload({ identity: identity(1, 8) }),
      payload({ identity: identity(1, 2, 8) }),
      payload({ identity: identity(1, 2, 3, 8) }),
      payload({ request_id: 'request-2' }),
      payload({ window_label: 'other' }),
      payload({ insert_id: '' }),
    ]) {
      assert.equal(guard.classify(rejected), 'reject')
    }
  })

  it('accepts a remote owner identity only with authenticated IPC authority', () => {
    const guard = new SpeechInsertGuard()
    guard.updateSnapshot(snapshot(identity()))
    guard.activateLease('request-1', 'main')
    const remoteInsert = payload({ identity: identity(9) })

    assert.equal(guard.classify(remoteInsert), 'reject')
    assert.equal(guard.rejectionReason(remoteInsert), 'identity-mismatch')
    assert.equal(guard.classify(remoteInsert, 'authenticated-ipc'), 'apply')

    const wrongRequest = payload({
      identity: identity(9),
      request_id: 'request-2',
      insert_id: 'insert-2',
    })
    assert.equal(guard.classify(wrongRequest, 'authenticated-ipc'), 'reject')
  })

  it('ignores an in-flight duplicate and consistently acknowledges an applied duplicate', () => {
    const guard = new SpeechInsertGuard()
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    const insert = payload()

    assert.equal(guard.classify(insert), 'apply')
    assert.equal(guard.classify(insert), 'ignore')
    guard.markApplied(insert.insert_id)
    assert.equal(guard.classify(insert), 'acknowledge')
  })

  it('releases failed insertion for a safe live retry', () => {
    const guard = new SpeechInsertGuard()
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    assert.equal(guard.classify(payload()), 'apply')
    guard.release('insert-1')
    assert.equal(guard.classify(payload()), 'apply')
  })

  it('bounds retained dedupe state', () => {
    const guard = new SpeechInsertGuard(3)
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    for (let index = 0; index < 10; index++) {
      const insert = payload({ insert_id: `insert-${index}` })
      assert.equal(guard.classify(insert), 'apply')
      guard.markApplied(insert.insert_id)
    }
    assert.equal(guard.retainedInsertCount(), 3)
  })

  it('never evicts an in-flight reservation to admit another frame', () => {
    const guard = new SpeechInsertGuard(2)
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    assert.equal(guard.classify(payload({ insert_id: 'pending-1' })), 'apply')
    assert.equal(guard.classify(payload({ insert_id: 'pending-2' })), 'apply')
    assert.equal(guard.classify(payload({ insert_id: 'pending-3' })), 'reject')
    assert.equal(guard.classify(payload({ insert_id: 'pending-1' })), 'ignore')
  })

  it('invalidates all inserts when the request lease disconnects', () => {
    const guard = new SpeechInsertGuard()
    guard.updateSnapshot(snapshot())
    guard.activateLease('request-1', 'main')
    assert.equal(guard.classify(payload()), 'apply')
    guard.markApplied('insert-1')
    guard.invalidateLease()

    assert.equal(guard.classify(payload()), 'reject')
    assert.equal(guard.retainedInsertCount(), 0)
  })
})
