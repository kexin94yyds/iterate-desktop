export interface CommandSuggestion {
  key: string
  description: string
}

interface MatchingOptions {
  acceptedSuggestionToken?: string
  isComposing?: boolean
}

export function filterCommandSuggestions<T extends CommandSuggestion>(
  suggestions: T[],
  rawQuery: string,
): T[] {
  const query = rawQuery.trim().toLocaleLowerCase()
  if (!query)
    return suggestions

  return suggestions
    .map((suggestion, index) => {
      const key = suggestion.key.toLocaleLowerCase()
      const description = suggestion.description.toLocaleLowerCase()
      const matchRank = key === query
        ? 0
        : key.startsWith(query)
          ? 1
          : key.includes(query)
            ? 2
            : description.includes(query)
              ? 3
              : null

      return {
        suggestion,
        index,
        matchRank,
      }
    })
    .filter((entry): entry is typeof entry & { matchRank: number } => entry.matchRank !== null)
    .sort((left, right) => left.matchRank - right.matchRank || left.index - right.index)
    .map(({ suggestion }) => suggestion)
}

export function getMatchingCommandSuggestions(
  suggestions: CommandSuggestion[],
  rawToken: string,
  options: MatchingOptions = {},
): CommandSuggestion[] {
  const token = rawToken.toLowerCase()
  if (!token || options.isComposing)
    return []

  if (options.acceptedSuggestionToken && token === options.acceptedSuggestionToken.toLowerCase())
    return []

  return suggestions
    .map((suggestion, index) => ({
      suggestion,
      index,
      key: suggestion.key.toLowerCase(),
    }))
    .filter(({ key }) => key.startsWith(token))
    .sort((left, right) => {
      const leftExact = left.key === token
      const rightExact = right.key === token
      if (leftExact !== rightExact)
        return leftExact ? -1 : 1

      return left.index - right.index
    })
    .map(({ suggestion }) => suggestion)
}

export function getCommandSuggestionSuffix(suggestion: CommandSuggestion | null, token: string): string {
  if (!suggestion || !token)
    return ''

  return suggestion.key.slice(token.length)
}

export function hasVisibleCommandSuggestion(suggestion: CommandSuggestion | null, token: string): boolean {
  return getCommandSuggestionSuffix(suggestion, token).length > 0
}
