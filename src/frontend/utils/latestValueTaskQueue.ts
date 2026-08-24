export interface LatestValueTaskQueue<T> {
  request: (value: T) => Promise<void>
}

/**
 * Serializes an async side effect while coalescing intermediate requests.
 * A newer request is always applied after the currently running task, so the
 * final external state matches the latest requested value.
 */
export function createLatestValueTaskQueue<T>(
  worker: (value: T) => Promise<void>,
  onError: (error: unknown) => void = () => {},
): LatestValueTaskQueue<T> {
  let latestValue: T
  let requestedRevision = 0
  let appliedRevision = 0
  let running: Promise<void> | null = null

  async function drain() {
    while (appliedRevision < requestedRevision) {
      const revision = requestedRevision
      const value = latestValue
      try {
        await worker(value)
      }
      catch (error) {
        onError(error)
      }
      appliedRevision = revision
    }
  }

  function request(value: T): Promise<void> {
    latestValue = value
    requestedRevision += 1
    if (!running) {
      running = drain().finally(() => {
        running = null
      })
    }
    return running
  }

  return { request }
}
