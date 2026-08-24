export interface GhostSuggestionStoreSyncSnapshot {
  updatedAt?: string | null
  suggestions: readonly unknown[]
}

export interface CacheRollbackGuardOptions {
  defaultKeys?: readonly string[]
  minMissingUserKeys?: number
}

const DEFAULT_MIN_MISSING_USER_KEYS = 3

export function storeTimestamp(store: Pick<GhostSuggestionStoreSyncSnapshot, 'updatedAt'> | null): number {
  if (!store?.updatedAt)
    return 0

  const timestamp = Date.parse(store.updatedAt)
  return Number.isFinite(timestamp) ? timestamp : 0
}

export function storesHaveSameSuggestions(
  a: Pick<GhostSuggestionStoreSyncSnapshot, 'suggestions'> | null,
  b: Pick<GhostSuggestionStoreSyncSnapshot, 'suggestions'> | null,
): boolean {
  if (!a || !b)
    return false

  return JSON.stringify(a.suggestions) === JSON.stringify(b.suggestions)
}

function suggestionKey(item: unknown): string | null {
  if (!item || typeof item !== 'object' || !('key' in item))
    return null

  const key = (item as { key?: unknown }).key
  return typeof key === 'string' && key.trim() ? key.trim().toLowerCase() : null
}

export function missingSuggestionKeys(
  candidateStore: Pick<GhostSuggestionStoreSyncSnapshot, 'suggestions'> | null,
  baselineStore: Pick<GhostSuggestionStoreSyncSnapshot, 'suggestions'> | null,
): string[] {
  if (!candidateStore || !baselineStore)
    return []

  const candidateKeys = new Set(
    candidateStore.suggestions
      .map(suggestionKey)
      .filter((key): key is string => !!key),
  )

  const missingKeys: string[] = []
  for (const item of baselineStore.suggestions) {
    const key = suggestionKey(item)
    if (key && !candidateKeys.has(key))
      missingKeys.push(key)
  }

  return missingKeys
}

export function shouldPreventCacheRollback(
  candidateStore: GhostSuggestionStoreSyncSnapshot | null,
  baselineStore: GhostSuggestionStoreSyncSnapshot | null,
  options: CacheRollbackGuardOptions = {},
): boolean {
  if (!candidateStore || !baselineStore)
    return false

  if (candidateStore.suggestions.length >= baselineStore.suggestions.length)
    return false

  const defaultKeys = new Set((options.defaultKeys ?? []).map(key => key.toLowerCase()))
  const missingUserKeyCount = missingSuggestionKeys(candidateStore, baselineStore)
    .filter(key => !defaultKeys.has(key))
    .length
  const threshold = options.minMissingUserKeys ?? DEFAULT_MIN_MISSING_USER_KEYS

  return missingUserKeyCount >= threshold
}

export function shouldApplyIncomingStore(
  incomingStore: GhostSuggestionStoreSyncSnapshot,
  localStore: GhostSuggestionStoreSyncSnapshot | null,
): boolean {
  if (!localStore)
    return true

  const incomingTimestamp = storeTimestamp(incomingStore)
  const localTimestamp = storeTimestamp(localStore)
  const incomingIsNewer = incomingTimestamp > localTimestamp
  const sameTimestampWithDifferentSuggestions = incomingTimestamp === localTimestamp
    && !storesHaveSameSuggestions(incomingStore, localStore)

  return incomingIsNewer || sameTimestampWithDifferentSuggestions
}
