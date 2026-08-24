import type { SpeechLayerIdentity, SpeechSnapshot } from './globalSpeechSession.ts'
import { GlobalSpeechSessionGuard } from './globalSpeechSession.ts'

export interface SpeechInsertPayload {
  identity: SpeechLayerIdentity
  request_id: string
  window_label: string
  text: string
  mode: 'final'
  insert_id: string
}

export type SpeechInsertDecision = 'apply' | 'acknowledge' | 'ignore' | 'reject'
export type SpeechInsertAuthority = 'local-session' | 'authenticated-ipc'
export type SpeechInsertRejectionReason
  = | 'inactive-lease'
    | 'request-mismatch'
    | 'window-mismatch'
    | 'invalid-mode'
    | 'missing-insert-id'
    | 'empty-text'
    | 'identity-mismatch'
    | 'capacity-exhausted'

export class SpeechInsertGuard {
  private readonly session = new GlobalSpeechSessionGuard()
  private readonly inserts = new Map<string, 'pending' | 'applied'>()
  private readonly capacity: number
  private requestId = ''
  private windowLabel = ''

  constructor(capacity = 128) {
    this.capacity = capacity
  }

  updateSnapshot(snapshot: SpeechSnapshot) {
    this.session.applySnapshot(snapshot)
  }

  activateLease(requestId: string, windowLabel: string) {
    const changed = requestId !== this.requestId || windowLabel !== this.windowLabel
    this.requestId = requestId
    this.windowLabel = windowLabel
    if (changed)
      this.inserts.clear()
  }

  invalidateLease() {
    this.requestId = ''
    this.windowLabel = ''
    this.inserts.clear()
  }

  classify(payload: SpeechInsertPayload, authority: SpeechInsertAuthority = 'local-session'): SpeechInsertDecision {
    if (this.rejectionReason(payload, authority))
      return 'reject'

    const existing = this.inserts.get(payload.insert_id)
    if (existing === 'applied') {
      this.touchApplied(payload.insert_id)
      return 'acknowledge'
    }
    if (existing === 'pending')
      return 'ignore'

    if (!this.reserve(payload.insert_id))
      return 'reject'
    return 'apply'
  }

  rejectionReason(
    payload: SpeechInsertPayload,
    authority: SpeechInsertAuthority = 'local-session',
  ): SpeechInsertRejectionReason | null {
    if (!this.requestId)
      return 'inactive-lease'
    if (payload.request_id !== this.requestId)
      return 'request-mismatch'
    if (payload.window_label !== this.windowLabel)
      return 'window-mismatch'
    if (payload.mode !== 'final')
      return 'invalid-mode'
    if (!payload.insert_id)
      return 'missing-insert-id'
    if (!payload.text)
      return 'empty-text'
    if (authority !== 'authenticated-ipc' && !this.session.isCurrent(payload.identity))
      return 'identity-mismatch'
    if (!this.inserts.has(payload.insert_id) && this.inserts.size >= Math.max(1, this.capacity)) {
      const hasAppliedInsert = [...this.inserts.values()].includes('applied')
      if (!hasAppliedInsert)
        return 'capacity-exhausted'
    }
    return null
  }

  markApplied(insertId: string) {
    if (this.inserts.has(insertId))
      this.touchApplied(insertId)
  }

  release(insertId: string) {
    this.inserts.delete(insertId)
  }

  retainedInsertCount() {
    return this.inserts.size
  }

  private reserve(insertId: string) {
    if (this.inserts.size >= Math.max(1, this.capacity)) {
      const evictable = [...this.inserts].find(([, state]) => state === 'applied')?.[0]
      if (evictable === undefined)
        return false
      this.inserts.delete(evictable)
    }
    this.inserts.set(insertId, 'pending')
    return true
  }

  private touchApplied(insertId: string) {
    this.inserts.delete(insertId)
    this.inserts.set(insertId, 'applied')
  }
}
