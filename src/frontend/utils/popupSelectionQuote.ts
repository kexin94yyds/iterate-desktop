export type PopupTextSelectionSource = 'request' | 'browser_ai'

export interface PopupTextSelection {
  text: string
  source: PopupTextSelectionSource
}

export function normalizePopupSelectionText(rawText: string, maxLength = 2000): string {
  const normalized = rawText
    .replace(/\u00A0/g, ' ')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n[ \t]+/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()

  if (normalized.length <= maxLength)
    return normalized

  return `${normalized.slice(0, maxLength).trimEnd()}\n...`
}

export function normalizePopupSelectionFragments(rawFragments: readonly string[], maxLength = 2000): string {
  return normalizePopupSelectionText(
    rawFragments
      .map(fragment => fragment.trim())
      .filter(Boolean)
      .join('\n'),
    maxLength,
  )
}

export function selectionTextInsideElement(
  container: HTMLElement | null,
  selection?: Selection | null,
  maxLength = 2000,
): string {
  if (!container)
    return ''

  const activeSelection = selection ?? (typeof window !== 'undefined' ? window.getSelection() : null)
  if (!activeSelection || activeSelection.isCollapsed || activeSelection.rangeCount === 0)
    return ''

  const range = activeSelection.getRangeAt(0)
  const commonAncestor = range.commonAncestorContainer
  const selectedElement = commonAncestor.nodeType === 1
    ? commonAncestor as Element
    : commonAncestor.parentElement

  if (!selectedElement || !container.contains(selectedElement))
    return ''

  return normalizePopupSelectionText(activeSelection.toString(), maxLength)
}

export function buildSelectedTextQuoteBlock(selection: PopupTextSelection | null | undefined): string {
  if (!selection?.text)
    return ''

  const quotedText = selection.text
    .split(/\r?\n/)
    .map(line => `> ${line}`)
    .join('\n')

  return quotedText
}

export function buildUserInputWithSelectedQuote(
  rawInput: string,
  fileRefs: string,
  selection: PopupTextSelection | null | undefined,
): string | null {
  const quoteBlock = buildSelectedTextQuoteBlock(selection)
  const trimmedInput = rawInput.trim()
  const trimmedFileRefs = fileRefs.trim()
  const combined = [quoteBlock, trimmedFileRefs, trimmedInput]
    .filter(Boolean)
    .join('\n\n')
    .trim()

  return combined || null
}
