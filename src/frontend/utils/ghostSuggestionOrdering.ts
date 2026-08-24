export interface PrioritizedGhostSuggestion {
  id: string
  sort_order: number
}

function priorityValue(sortOrder: number, fallbackIndex: number): number {
  return Number.isFinite(sortOrder) && sortOrder > 0 ? sortOrder : fallbackIndex + 1
}

export function normalizeGhostSuggestionOrder<T extends PrioritizedGhostSuggestion>(items: readonly T[]): T[] {
  return items
    .map((item, index) => ({ item, index }))
    .sort((left, right) => {
      return priorityValue(left.item.sort_order, left.index) - priorityValue(right.item.sort_order, right.index)
        || left.index - right.index
    })
    .map(({ item }, index) => ({
      ...item,
      sort_order: index + 1,
    }))
}

export function mergeFilteredSuggestionOrder<T extends { id: string }>(
  allItems: readonly T[],
  reorderedFilteredIds: readonly string[],
): T[] {
  const itemById = new Map(allItems.map(item => [item.id, item]))
  const filteredIds = new Set<string>()

  for (const id of reorderedFilteredIds) {
    if (!itemById.has(id))
      throw new Error(`Unknown ghost suggestion id: ${id}`)
    if (filteredIds.has(id))
      throw new Error(`Duplicate ghost suggestion id: ${id}`)
    filteredIds.add(id)
  }

  const reorderedItems = reorderedFilteredIds.map(id => itemById.get(id)!)
  let reorderedIndex = 0

  return allItems.map((item) => {
    if (!filteredIds.has(item.id))
      return item

    const replacement = reorderedItems[reorderedIndex]
    reorderedIndex += 1
    return replacement
  })
}
