export interface CodexLiveTranscriptState {
  text: string
  role: string | undefined
  finalized: boolean
}

export function advanceCodexLiveTranscript(
  state: CodexLiveTranscriptState,
  rawText: string,
  role: string | undefined,
  isFinal: boolean,
): CodexLiveTranscriptState | null {
  const prefix = role === 'assistant' ? 'Codex：' : role === 'user' ? '你：' : ''
  if (isFinal) {
    const text = rawText.trim()
    return {
      text: text ? `${prefix}${text}` : state.text,
      role: role ?? state.role,
      finalized: true,
    }
  }

  if (!rawText)
    return null

  return {
    text: state.role !== role || state.finalized
      ? `${prefix}${rawText.trimStart()}`
      : state.text + rawText,
    role,
    finalized: false,
  }
}
