/* eslint-disable test/no-import-node-test */
import type { GhostSuggestionStoreSyncSnapshot } from './ghostSuggestionStoreSync.ts'
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import {
  missingSuggestionKeys,
  shouldApplyIncomingStore,
  shouldPreventCacheRollback,
  storesHaveSameSuggestions,
  storeTimestamp,
} from './ghostSuggestionStoreSync.ts'

function store(updatedAt: string | null, keys: string[]): GhostSuggestionStoreSyncSnapshot {
  return {
    updatedAt,
    suggestions: keys.map((key, index) => ({
      id: key,
      key,
      description: key,
      enabled: true,
      sort_order: index + 1,
    })),
  }
}

describe('ghost suggestion store sync', () => {
  it('applies incoming stores when there is no local store', () => {
    assert.equal(
      shouldApplyIncomingStore(store('2026-05-22T00:00:00.000Z', ['activity']), null),
      true,
    )
  })

  it('applies newer incoming stores', () => {
    assert.equal(
      shouldApplyIncomingStore(
        store('2026-05-22T00:01:00.000Z', ['activity']),
        store('2026-05-22T00:00:00.000Z', ['activity']),
      ),
      true,
    )
  })

  it('ignores older incoming stores', () => {
    assert.equal(
      shouldApplyIncomingStore(
        store('2026-05-22T00:00:00.000Z', ['old']),
        store('2026-05-22T00:01:00.000Z', ['new']),
      ),
      false,
    )
  })

  it('keeps same-timestamp conflict refresh behavior', () => {
    assert.equal(
      shouldApplyIncomingStore(
        store('2026-05-22T00:00:00.000Z', ['incoming']),
        store('2026-05-22T00:00:00.000Z', ['local']),
      ),
      true,
    )

    assert.equal(
      shouldApplyIncomingStore(
        store('2026-05-22T00:00:00.000Z', ['same']),
        store('2026-05-22T00:00:00.000Z', ['same']),
      ),
      false,
    )
  })

  it('handles invalid timestamps defensively', () => {
    assert.equal(storeTimestamp(store('not-a-date', ['activity'])), 0)

    assert.equal(
      shouldApplyIncomingStore(
        store('not-a-date', ['incoming']),
        store('2026-05-22T00:00:00.000Z', ['local']),
      ),
      false,
    )

    assert.equal(
      shouldApplyIncomingStore(
        store('2026-05-22T00:00:00.000Z', ['incoming']),
        store('not-a-date', ['local']),
      ),
      true,
    )
  })

  it('compares suggestion content structurally', () => {
    assert.equal(
      storesHaveSameSuggestions(
        store('2026-05-22T00:00:00.000Z', ['activity']),
        store('2026-05-22T00:01:00.000Z', ['activity']),
      ),
      true,
    )
    assert.equal(
      storesHaveSameSuggestions(
        store('2026-05-22T00:00:00.000Z', ['activity']),
        store('2026-05-22T00:01:00.000Z', ['other']),
      ),
      false,
    )
  })

  it('detects missing suggestion keys', () => {
    assert.deepEqual(
      missingSuggestionKeys(
        store('2026-05-30T00:00:00.000Z', ['ji', 'cha']),
        store('2026-05-21T00:00:00.000Z', ['ji', 'activity', 'skill', 'cha']),
      ),
      ['activity', 'skill'],
    )
  })

  it('prevents smaller default-like caches from rolling back multiple user suggestions', () => {
    assert.equal(
      shouldPreventCacheRollback(
        store('2026-05-30T00:00:00.000Z', ['ji', 'cha', 'hui']),
        store('2026-05-21T00:00:00.000Z', ['ji', 'cha', 'hui', 'activity', 'skill', 'computeruse']),
        { defaultKeys: ['ji', 'cha', 'hui'] },
      ),
      true,
    )
  })

  it('allows small explicit reductions and larger stores', () => {
    assert.equal(
      shouldPreventCacheRollback(
        store('2026-05-30T00:00:00.000Z', ['ji', 'cha', 'activity']),
        store('2026-05-21T00:00:00.000Z', ['ji', 'cha', 'activity', 'skill']),
        { defaultKeys: ['ji', 'cha'] },
      ),
      false,
    )

    assert.equal(
      shouldPreventCacheRollback(
        store('2026-05-30T00:00:00.000Z', ['ji', 'cha', 'activity', 'skill']),
        store('2026-05-21T00:00:00.000Z', ['ji', 'cha', 'activity']),
        { defaultKeys: ['ji', 'cha'] },
      ),
      false,
    )
  })
})
