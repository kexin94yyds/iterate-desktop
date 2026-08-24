export interface LatestTaskToken {
  generation: number
  scope: string
  revision: number
}

/**
 * Issues monotonically newer tokens for async work where only the latest task
 * is allowed to continue after an await boundary.
 */
export function createLatestTaskGuard() {
  let generation = 0

  return {
    issue(scope: string, revision: number): LatestTaskToken {
      generation += 1
      return { generation, scope, revision }
    },
    invalidate() {
      generation += 1
    },
    isCurrent(token: LatestTaskToken, scope: string) {
      return token.generation === generation && token.scope === scope
    },
  }
}
