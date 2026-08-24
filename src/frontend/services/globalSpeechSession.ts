export interface SpeechLayerIdentity {
  schema_version: string
  owner_epoch_hi: string
  owner_epoch_lo: string
  control_seq: string
  session_sequence: string
  revision: string
}

export interface SpeechSnapshot {
  owner_epoch: number[] | null
  desired_state: 'Off' | 'On'
  phase: 'Idle' | 'Arming' | 'Listening' | 'Finishing' | 'Processing' | 'Committing' | 'Terminal'
  visible: boolean
  control_seq: number
  session: number | null
  identity: SpeechLayerIdentity | null
  partial_len: number
  writeback_outcome: 'NotDispatched' | 'Dispatched' | 'DispatchedUnverified' | 'Acknowledged' | 'FailedBeforeDispatch' | 'UnknownAfterDispatch'
}

export type SpeechRenderPhase = 'idle' | 'starting' | 'listening' | 'stopping' | 'processing' | 'success' | 'error'

export function sameSpeechIdentity(_left: SpeechLayerIdentity | null, _right: SpeechLayerIdentity | null) {
  if (!_left || !_right)
    return _left === _right
  return _left.schema_version === _right.schema_version
    && _left.owner_epoch_hi === _right.owner_epoch_hi
    && _left.owner_epoch_lo === _right.owner_epoch_lo
    && _left.control_seq === _right.control_seq
    && _left.session_sequence === _right.session_sequence
    && _left.revision === _right.revision
}

export function deriveSpeechRenderPhase(snapshot: SpeechSnapshot): SpeechRenderPhase {
  switch (snapshot.phase) {
    case 'Arming':
      return 'starting'
    case 'Listening':
      return 'listening'
    case 'Finishing':
      return 'stopping'
    case 'Processing':
    case 'Committing':
      return 'processing'
    case 'Terminal':
      if (snapshot.writeback_outcome === 'Acknowledged')
        return 'success'
      if (snapshot.writeback_outcome === 'FailedBeforeDispatch' || snapshot.writeback_outcome === 'UnknownAfterDispatch')
        return 'error'
      return 'idle'
    default:
      return 'idle'
  }
}

export class GlobalSpeechSessionGuard {
  private current: SpeechSnapshot | null = null
  private retiredEpochs = new Set<string>()
  private claimed = new Set<string>()

  applySnapshot(snapshot: SpeechSnapshot) {
    const incomingEpoch = snapshotEpochKey(snapshot)
    if (!incomingEpoch || this.retiredEpochs.has(incomingEpoch))
      return false

    if (!this.current) {
      this.current = snapshot
      return true
    }

    const currentEpoch = snapshotEpochKey(this.current)
    if (incomingEpoch !== currentEpoch) {
      if (currentEpoch)
        this.retiredEpochs.add(currentEpoch)
      this.current = snapshot
      this.claimed.clear()
      return true
    }

    if (compareSnapshotOrder(snapshot, this.current) <= 0)
      return false

    this.current = snapshot
    return true
  }

  snapshot() {
    return this.current
  }

  isCurrent(identity: SpeechLayerIdentity) {
    return sameSpeechIdentity(this.current?.identity ?? null, identity)
  }

  claimDirective(kind: 'configure' | 'process', identity: SpeechLayerIdentity) {
    if (!this.isCurrent(identity))
      return false
    const key = `${kind}:${identityKey(identity)}`
    if (this.claimed.has(key))
      return false
    this.claimed.add(key)
    return true
  }
}

function identityKey(identity: SpeechLayerIdentity) {
  return [
    identity.schema_version,
    identity.owner_epoch_hi,
    identity.owner_epoch_lo,
    identity.control_seq,
    identity.session_sequence,
    identity.revision,
  ].join(':')
}

function snapshotEpochKey(snapshot: SpeechSnapshot) {
  if (snapshot.owner_epoch)
    return snapshot.owner_epoch.join('.')
  if (snapshot.identity)
    return `${snapshot.identity.owner_epoch_hi}:${snapshot.identity.owner_epoch_lo}`
  return ''
}

function compareSnapshotOrder(left: SpeechSnapshot, right: SpeechSnapshot) {
  const leftIdentity = left.identity
  const rightIdentity = right.identity
  if (!leftIdentity)
    return rightIdentity ? -1 : Number(BigInt(left.control_seq) - BigInt(right.control_seq))
  if (!rightIdentity)
    return 1
  for (const field of ['control_seq', 'session_sequence', 'revision'] as const) {
    const compared = BigInt(leftIdentity[field]) - BigInt(rightIdentity[field])
    if (compared !== 0n)
      return compared > 0n ? 1 : -1
  }
  return 0
}
