/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { mergeFilteredSuggestionOrder, normalizeGhostSuggestionOrder } from './ghostSuggestionOrdering.ts'

const items = [
  { id: 'a', key: 'alpha' },
  { id: 'b', key: 'beta' },
  { id: 'c', key: 'charlie' },
  { id: 'd', key: 'delta' },
  { id: 'e', key: 'echo' },
]

describe('ghost suggestion manual ordering', () => {
  it('normalizes the persisted manual priority without falling back to alphabetic order', () => {
    const prioritized = normalizeGhostSuggestionOrder([
      { id: 'alpha', key: 'alpha', sort_order: 30 },
      { id: 'gamma', key: 'gamma', sort_order: 10 },
      { id: 'beta', key: 'beta', sort_order: 20 },
    ])

    assert.deepEqual(prioritized.map(item => item.id), ['gamma', 'beta', 'alpha'])
    assert.deepEqual(prioritized.map(item => item.sort_order), [1, 2, 3])
  })

  it('reorders only filtered slots while leaving nonmatches anchored', () => {
    assert.deepEqual(
      mergeFilteredSuggestionOrder(items, ['e', 'c', 'a']).map(item => item.id),
      ['e', 'b', 'c', 'd', 'a'],
    )
  })

  it('supports moving the last filtered match to first priority', () => {
    assert.deepEqual(
      mergeFilteredSuggestionOrder(items, ['c', 'a']).map(item => item.id),
      ['c', 'b', 'a', 'd', 'e'],
    )
  })

  it('rejects duplicate or unknown ids instead of corrupting the full order', () => {
    assert.throws(() => mergeFilteredSuggestionOrder(items, ['a', 'a']))
    assert.throws(() => mergeFilteredSuggestionOrder(items, ['a', 'missing']))
  })
})
