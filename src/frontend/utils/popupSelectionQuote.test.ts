import assert from 'node:assert/strict'
import {
  buildSelectedTextQuoteBlock,
  buildUserInputWithSelectedQuote,
  normalizePopupSelectionFragments,
  normalizePopupSelectionText,
  selectionTextInsideElement,
} from './popupSelectionQuote.ts'

function createContainer(acceptedElement: unknown): HTMLElement {
  return {
    contains(element: unknown) {
      return element === acceptedElement
    },
  } as unknown as HTMLElement
}

function createSelection(
  text: string,
  commonAncestorContainer: unknown,
  overrides: Partial<Selection> = {},
): Selection {
  return {
    isCollapsed: false,
    rangeCount: 1,
    getRangeAt() {
      return { commonAncestorContainer } as Range
    },
    toString() {
      return text
    },
    ...overrides,
  } as unknown as Selection
}

assert.equal(
  normalizePopupSelectionText('  first line  \n\n\n second line  '),
  'first line\n\nsecond line',
)

assert.equal(
  normalizePopupSelectionText('abcdef', 3),
  'abc\n...',
)

assert.equal(
  normalizePopupSelectionFragments(['  first fragment  ', '', 'second\nfragment']),
  'first fragment\nsecond\nfragment',
)

assert.equal(
  normalizePopupSelectionFragments(['abcd', 'efgh'], 6),
  'abcd\ne\n...',
)

assert.equal(
  buildSelectedTextQuoteBlock({ source: 'request', text: 'line 1\nline 2' }),
  '> line 1\n> line 2',
)

assert.equal(
  buildUserInputWithSelectedQuote(
    '请解释这段',
    '@/tmp/a.ts',
    { source: 'browser_ai', text: 'selected text' },
  ),
  '> selected text\n\n@/tmp/a.ts\n\n请解释这段',
)

{
  const selectedElement = { nodeType: 1 }
  assert.equal(
    selectionTextInsideElement(
      createContainer(selectedElement),
      createSelection('  selected   text  ', selectedElement),
    ),
    'selected   text',
  )
}

{
  const selectedElement = { nodeType: 1 }
  const outsideElement = { nodeType: 1 }
  assert.equal(
    selectionTextInsideElement(
      createContainer(selectedElement),
      createSelection('outside text', outsideElement),
    ),
    '',
  )
}

{
  const selectedElement = { nodeType: 1 }
  assert.equal(
    selectionTextInsideElement(
      createContainer(selectedElement),
      createSelection('collapsed text', selectedElement, { isCollapsed: true }),
    ),
    '',
  )
}

{
  const selectedElement = { nodeType: 1 }
  const textNode = { nodeType: 3, parentElement: selectedElement }
  assert.equal(
    selectionTextInsideElement(
      createContainer(selectedElement),
      createSelection('line 1\n\n\n line 2', textNode),
    ),
    'line 1\n\nline 2',
  )
}
