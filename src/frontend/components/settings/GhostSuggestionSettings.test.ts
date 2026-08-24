/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { describe, it } from 'node:test'

const source = await readFile(new URL('./GhostSuggestionSettings.vue', import.meta.url), 'utf8')

describe('GhostSuggestionSettings manual priority management', () => {
  it('supports handle-based sorting inside filtered results', () => {
    assert.match(source, /useSortable\(sortableContainer, sortableSuggestions/)
    assert.match(source, /handle: '\.ghost-drag-handle'/)
    assert.match(source, /mergeFilteredSuggestionOrder\(sortedSuggestions\.value, reorderedIds\)/)
    assert.match(source, /同类匹配内的补全优先级/)
    assert.doesNotMatch(source, /compareGhostSuggestionKeys/)
  })

  it('provides explicit batch enable, disable, and delete controls', () => {
    assert.match(source, />\s*批量管理\s*</)
    assert.match(source, /handleBatchEnabled\(true\)/)
    assert.match(source, /handleBatchEnabled\(false\)/)
    assert.match(source, /openDelete\(selectedIds\)/)
    assert.match(source, /确定删除选中的.*10 秒内撤销/)
  })

  it('reveals row actions on hover or keyboard focus without shifting the table', () => {
    assert.match(source, /\.ghost-table tbody tr:hover,\s*\.ghost-table tbody tr:focus-within/)
    assert.match(source, /\.ghost-table__action-list\s*\{[\s\S]*?opacity:\s*0;[\s\S]*?pointer-events:\s*none;/)
    assert.match(source, /\.ghost-table tbody tr:hover \.ghost-table__action-list,\s*\.ghost-table tbody tr:focus-within \.ghost-table__action-list/)
    assert.match(source, /\.sortable-chosen \.ghost-table__action-list,\s*\.sortable-drag \.ghost-table__action-list/)
    assert.match(source, /@media \(hover: none\), \(pointer: coarse\)[\s\S]*?\.ghost-table__action-list\s*\{[\s\S]*?opacity:\s*1;[\s\S]*?pointer-events:\s*auto;/)
  })

  it('keeps hover actions inside narrow settings windows and readable in both themes', () => {
    assert.match(source, /<td class="ghost-table__actions-cell">/)
    assert.match(source, /class="ghost-row-action ghost-row-action--edit"/)
    assert.match(source, /class="ghost-row-action ghost-row-action--delete"/)
    assert.match(source, /\.ghost-table__actions,\s*\n\.ghost-table__actions-cell\s*\{[\s\S]*?position:\s*sticky;[\s\S]*?right:\s*0;/)
    assert.match(source, /\.ghost-row-action\s*\{[\s\S]*?--n-text-color:\s*var\(--color-on-surface\)\s*!important;[\s\S]*?--n-color:/)
    assert.match(source, /\.ghost-row-action--delete\s*\{[\s\S]*?--n-text-color:\s*#dc2626\s*!important;/)
    assert.match(source, /\.ghost-table__description\s*\{[\s\S]*?color:\s*var\(--color-on-surface-secondary\);/)
    assert.match(source, /\.ghost-priority-number\s*\{[\s\S]*?color:\s*var\(--color-on-surface-muted\);/)
  })

  it('blocks snapshot undo after another store update', () => {
    assert.match(source, /storeUpdatedAt\.value !== undoExpectedUpdatedAt\.value/)
    assert.match(source, /为避免覆盖新改动，本次撤销已取消/)
    assert.match(source, /撤销删除/)
  })
})
