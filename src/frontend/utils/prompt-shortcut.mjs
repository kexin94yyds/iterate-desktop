export const PROMPT_SHORTCUT_LIMIT = 9

export function getPromptShortcutIndex(event) {
  if (
    !event.altKey
    || event.metaKey
    || event.ctrlKey
    || event.shiftKey
    || event.isComposing
    || event.repeat
  ) {
    return -1
  }

  const match = /^Digit([1-9])$/.exec(event.code ?? '')
  return match ? Number.parseInt(match[1], 10) - 1 : -1
}
