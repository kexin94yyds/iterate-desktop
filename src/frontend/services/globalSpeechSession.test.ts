/* eslint-disable test/no-import-node-test */
import type { SpeechLayerIdentity, SpeechSnapshot } from './globalSpeechSession.ts'
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import {
  deriveSpeechRenderPhase,
  GlobalSpeechSessionGuard,
  sameSpeechIdentity,
} from './globalSpeechSession.ts'

function identity(epoch = 1, control = 1, session = 1, revision = 1): SpeechLayerIdentity {
  return {
    schema_version: '1',
    owner_epoch_hi: String(epoch),
    owner_epoch_lo: String(epoch),
    control_seq: String(control),
    session_sequence: String(session),
    revision: String(revision),
  }
}

function snapshot(current = identity(), phase: SpeechSnapshot['phase'] = 'Arming'): SpeechSnapshot {
  return {
    owner_epoch: Array.from({ length: 16 }, () => Number(current.owner_epoch_hi)),
    desired_state: phase === 'Idle' ? 'Off' : 'On',
    phase,
    visible: phase !== 'Idle' && phase !== 'Terminal',
    control_seq: Number(current.control_seq),
    session: Number(current.session_sequence),
    identity: current,
    partial_len: 0,
    writeback_outcome: 'NotDispatched',
  }
}

describe('global speech session guard', () => {
  it('applies only the newest complete identity in an owner epoch', () => {
    const guard = new GlobalSpeechSessionGuard()
    const first = snapshot(identity(1, 3, 2, 4))
    const newest = snapshot(identity(1, 4, 3, 1), 'Listening')

    assert.equal(guard.applySnapshot(first), true)
    assert.equal(guard.applySnapshot(newest), true)
    assert.equal(guard.applySnapshot(first), false)
    assert.equal(guard.snapshot(), newest)
  })

  it('accepts a full current snapshot on reload and rejects the retired epoch', () => {
    const guard = new GlobalSpeechSessionGuard()
    const oldOwner = snapshot(identity(1, 8, 5, 3), 'Listening')
    const replacement = snapshot(identity(2, 1, 1, 1), 'Arming')

    assert.equal(guard.applySnapshot(oldOwner), true)
    assert.equal(guard.applySnapshot(replacement), true)
    assert.equal(guard.applySnapshot(oldOwner), false)
    assert.equal(guard.isCurrent(replacement.identity!), true)
  })

  it('rejects old epoch, control, session, and revision directive replies', () => {
    const guard = new GlobalSpeechSessionGuard()
    const current = identity(7, 9, 4, 6)
    guard.applySnapshot(snapshot(current, 'Processing'))

    for (const stale of [
      identity(6, 9, 4, 6),
      identity(7, 8, 4, 6),
      identity(7, 9, 3, 6),
      identity(7, 9, 4, 5),
    ]) {
      assert.equal(guard.isCurrent(stale), false)
      assert.equal(guard.claimDirective('process', stale), false)
    }
    assert.equal(sameSpeechIdentity(current, { ...current }), true)
  })

  it('drops prepare and processing completions after cancellation or a newer session', () => {
    const guard = new GlobalSpeechSessionGuard()
    const preparing = identity(3, 1, 1, 2)
    guard.applySnapshot(snapshot(preparing, 'Arming'))
    assert.equal(guard.claimDirective('configure', preparing), true)

    const cancelled = identity(3, 2, 1, 3)
    guard.applySnapshot(snapshot(cancelled, 'Terminal'))
    assert.equal(guard.isCurrent(preparing), false)

    const processing = identity(3, 3, 2, 4)
    guard.applySnapshot(snapshot(processing, 'Processing'))
    assert.equal(guard.claimDirective('process', processing), true)
    guard.applySnapshot(snapshot(identity(3, 4, 3, 1), 'Arming'))
    assert.equal(guard.isCurrent(processing), false)
  })

  it('claims each directive exactly once', () => {
    const guard = new GlobalSpeechSessionGuard()
    const current = identity(4, 2, 2, 7)
    guard.applySnapshot(snapshot(current, 'Processing'))

    assert.equal(guard.claimDirective('process', current), true)
    assert.equal(guard.claimDirective('process', current), false)
    assert.equal(guard.claimDirective('configure', current), true)
    assert.equal(guard.claimDirective('configure', current), false)
  })

  it('derives rendering state solely from the Rust phase and outcome', () => {
    assert.equal(deriveSpeechRenderPhase(snapshot(identity(), 'Idle')), 'idle')
    assert.equal(deriveSpeechRenderPhase(snapshot(identity(), 'Arming')), 'starting')
    assert.equal(deriveSpeechRenderPhase(snapshot(identity(), 'Listening')), 'listening')
    assert.equal(deriveSpeechRenderPhase(snapshot(identity(), 'Finishing')), 'stopping')
    assert.equal(deriveSpeechRenderPhase(snapshot(identity(), 'Processing')), 'processing')
    assert.equal(deriveSpeechRenderPhase({ ...snapshot(identity(), 'Terminal'), writeback_outcome: 'Acknowledged' }), 'success')
    assert.equal(deriveSpeechRenderPhase({ ...snapshot(identity(), 'Terminal'), writeback_outcome: 'DispatchedUnverified' }), 'idle')
    assert.equal(deriveSpeechRenderPhase({ ...snapshot(identity(), 'Terminal'), writeback_outcome: 'FailedBeforeDispatch' }), 'error')
  })
})
