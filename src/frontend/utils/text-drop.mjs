function clampSelectionIndex(value, fallback, textLength) {
  if (!Number.isFinite(value))
    return fallback

  return Math.min(Math.max(Math.trunc(value), 0), textLength)
}

export function insertDroppedText(currentText, droppedText, selectionStart, selectionEnd) {
  const current = String(currentText ?? '')
  const inserted = String(droppedText ?? '')
  const fallback = current.length
  const rawStart = clampSelectionIndex(selectionStart, fallback, current.length)
  const rawEnd = clampSelectionIndex(selectionEnd, rawStart, current.length)
  const start = Math.min(rawStart, rawEnd)
  const end = Math.max(rawStart, rawEnd)

  return {
    value: `${current.slice(0, start)}${inserted}${current.slice(end)}`,
    cursor: start + inserted.length,
  }
}
