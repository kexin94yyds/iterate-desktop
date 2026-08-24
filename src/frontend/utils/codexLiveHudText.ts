import type { DesktopCodexLiveExecutionPhase } from '../services/desktopCodexLiveControl'

export function resolveCodexLiveHudText(input: {
  executionPhase?: DesktopCodexLiveExecutionPhase
  statusText?: string
  taskProgressText?: string
  latestTranscript?: string
  fallbackStatusText?: string
}): string {
  const statusText = input.statusText?.trim() || ''
  const taskProgressText = input.taskProgressText?.trim() || ''
  const latestTranscript = input.latestTranscript?.trim() || ''
  const fallbackStatusText = input.fallbackStatusText?.trim() || ''
  const executionActive = input.executionPhase !== undefined
    && input.executionPhase !== 'waiting'

  if (executionActive)
    return taskProgressText || statusText || latestTranscript || fallbackStatusText || 'GPT-Live 正在准备'

  return latestTranscript || statusText || fallbackStatusText || 'GPT-Live 正在准备'
}
