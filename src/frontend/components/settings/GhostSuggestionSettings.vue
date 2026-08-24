<script setup lang="ts">
import type { GhostSuggestion } from '../../composables/useGhostSuggestions'
import { useSortable } from '@vueuse/integrations/useSortable'
import { useMessage } from 'naive-ui'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useGhostSuggestions } from '../../composables/useGhostSuggestions'
import { filterCommandSuggestions } from '../../utils/ghostSuggestionMatching'
import { mergeFilteredSuggestionOrder } from '../../utils/ghostSuggestionOrdering'

const message = useMessage()
const {
  suggestions,
  enabledSuggestions,
  storeUpdatedAt,
  addSuggestion,
  updateSuggestion,
  toggleSuggestion,
  replaceSuggestions,
  reorderSuggestions,
} = useGhostSuggestions()

const showEditor = ref(false)
const editingId = ref<string | null>(null)
const searchQuery = ref('')
const sortableContainer = ref<HTMLElement | null>(null)
const sortableSuggestions = shallowRef<GhostSuggestion[]>([])
const batchMode = ref(false)
const selectedIds = ref<string[]>([])
const pendingDeleteIds = ref<string[]>([])
const undoSnapshot = ref<GhostSuggestion[] | null>(null)
const undoExpectedUpdatedAt = ref('')
const undoDeletedCount = ref(0)
let undoTimer: ReturnType<typeof setTimeout> | null = null
const form = ref({
  key: '',
  description: '',
  enabled: true,
})

const sortedSuggestions = computed(() => {
  return [...suggestions.value].sort((left, right) => left.sort_order - right.sort_order)
})

const filteredSuggestions = computed(() => {
  return filterCommandSuggestions(sortedSuggestions.value, searchQuery.value)
})

const hasSearchQuery = computed(() => searchQuery.value.trim().length > 0)
const selectedIdSet = computed(() => new Set(selectedIds.value))
const visibleIds = computed(() => sortableSuggestions.value.map(item => item.id))
const allVisibleSelected = computed(() => {
  return visibleIds.value.length > 0 && visibleIds.value.every(id => selectedIdSet.value.has(id))
})
const someVisibleSelected = computed(() => {
  return visibleIds.value.some(id => selectedIdSet.value.has(id)) && !allVisibleSelected.value
})
const deleteDialogTitle = computed(() => pendingDeleteIds.value.length > 1 ? '批量删除幽灵补全' : '删除幽灵补全')
const deleteDialogDescription = computed(() => {
  if (pendingDeleteIds.value.length > 1)
    return `确定删除选中的 ${pendingDeleteIds.value.length} 条幽灵补全吗？删除后可在 10 秒内撤销。`

  const id = pendingDeleteIds.value[0]
  const suggestion = sortedSuggestions.value.find(item => item.id === id)
  return suggestion ? `确定删除“${suggestion.key}”吗？删除后可在 10 秒内撤销。` : ''
})

const editorTitle = computed(() => editingId.value ? '编辑幽灵补全' : '添加幽灵补全')

const { option: setSortableOption } = useSortable(sortableContainer, sortableSuggestions, {
  animation: 180,
  handle: '.ghost-drag-handle',
  draggable: 'tr[data-suggestion-id]',
  ghostClass: 'sortable-ghost',
  chosenClass: 'sortable-chosen',
  dragClass: 'sortable-drag',
  forceFallback: true,
  fallbackTolerance: 4,
  onUpdate: (event) => {
    if (event.oldIndex === undefined || event.newIndex === undefined || event.oldIndex === event.newIndex)
      return

    const reorderedIds = sortableSuggestions.value.map(item => item.id)
    const [movedId] = reorderedIds.splice(event.oldIndex, 1)
    reorderedIds.splice(event.newIndex, 0, movedId)
    persistVisibleOrder(reorderedIds)
  },
})

watch(filteredSuggestions, (items) => {
  sortableSuggestions.value = [...items]
}, { immediate: true })

watch([batchMode, () => sortableSuggestions.value.length], () => {
  void nextTick(updateSortableDisabled)
})

onMounted(() => {
  void nextTick(updateSortableDisabled)
})

onBeforeUnmount(clearUndo)

function updateSortableDisabled() {
  setSortableOption('disabled', batchMode.value || sortableSuggestions.value.length < 2)
}

function persistVisibleOrder(reorderedIds: string[]) {
  try {
    const merged = mergeFilteredSuggestionOrder(sortedSuggestions.value, reorderedIds)
    if (!reorderSuggestions(merged.map(item => item.id)))
      throw new Error('invalid ghost suggestion order')

    const itemById = new Map(merged.map(item => [item.id, item]))
    sortableSuggestions.value = reorderedIds.map(id => itemById.get(id)!)
    message.success(hasSearchQuery.value ? '筛选结果优先级已保存' : '幽灵补全优先级已保存')
  }
  catch (error) {
    console.error('[GhostSuggestions] reorder failed:', error)
    sortableSuggestions.value = [...filteredSuggestions.value]
    message.error('保存优先级失败，已恢复原顺序')
  }
}

function moveVisibleSuggestion(index: number, offset: number) {
  const destination = index + offset
  if (!sortableSuggestions.value[destination])
    return

  const ids = sortableSuggestions.value.map(item => item.id)
  const [movedId] = ids.splice(index, 1)
  ids.splice(destination, 0, movedId)
  persistVisibleOrder(ids)
}

function resetForm() {
  form.value = {
    key: '',
    description: '',
    enabled: true,
  }
}

function openAddEditor() {
  editingId.value = null
  resetForm()
  showEditor.value = true
}

function openEditEditor(item: GhostSuggestion) {
  editingId.value = item.id
  form.value = {
    key: item.key,
    description: item.description,
    enabled: item.enabled,
  }
  showEditor.value = true
}

function handleSave(): boolean {
  const input = {
    key: form.value.key,
    description: form.value.description,
    enabled: form.value.enabled,
  }
  const result = editingId.value
    ? updateSuggestion(editingId.value, input)
    : addSuggestion(input)

  if (!result.ok) {
    message.warning(result.reason)
    return false
  }

  message.success(editingId.value ? '已更新' : '已添加')
  resetForm()
  editingId.value = null
  return true
}

function handleToggle(item: GhostSuggestion, enabled: boolean) {
  if (!toggleSuggestion(item.id, enabled))
    message.error('更新失败')
}

function setBatchMode(enabled: boolean) {
  batchMode.value = enabled
  selectedIds.value = []
}

function toggleSelection(id: string, checked: boolean) {
  const next = new Set(selectedIds.value)
  if (checked)
    next.add(id)
  else
    next.delete(id)
  selectedIds.value = [...next]
}

function toggleVisibleSelection(checked: boolean) {
  const next = new Set(selectedIds.value)
  visibleIds.value.forEach(id => checked ? next.add(id) : next.delete(id))
  selectedIds.value = [...next]
}

function handleBatchEnabled(enabled: boolean) {
  if (selectedIds.value.length === 0)
    return

  const selected = selectedIdSet.value
  const now = new Date().toISOString()
  const next = sortedSuggestions.value.map(item => selected.has(item.id)
    ? { ...item, enabled, updated_at: now }
    : item)
  if (!replaceSuggestions(next)) {
    message.error('批量更新失败')
    return
  }

  message.success(`已${enabled ? '启用' : '停用'} ${selectedIds.value.length} 条`)
  selectedIds.value = []
}

function openDelete(ids: string[]) {
  pendingDeleteIds.value = [...new Set(ids)]
}

function handleDelete() {
  if (pendingDeleteIds.value.length === 0)
    return

  const deleting = new Set(pendingDeleteIds.value)
  const snapshot = sortedSuggestions.value.map(item => ({ ...item }))
  const next = snapshot.filter(item => !deleting.has(item.id))
  const deletedCount = snapshot.length - next.length
  if (deletedCount === 0 || !replaceSuggestions(next)) {
    message.error('删除失败')
    pendingDeleteIds.value = []
    return
  }

  pendingDeleteIds.value = []
  selectedIds.value = []
  offerUndo(snapshot, deletedCount, storeUpdatedAt.value)
  message.success(`已删除 ${deletedCount} 条`)
}

function offerUndo(snapshot: GhostSuggestion[], deletedCount: number, expectedUpdatedAt: string) {
  clearUndo()
  undoSnapshot.value = snapshot
  undoDeletedCount.value = deletedCount
  undoExpectedUpdatedAt.value = expectedUpdatedAt
  undoTimer = setTimeout(clearUndo, 10_000)
}

function clearUndo() {
  if (undoTimer !== null)
    clearTimeout(undoTimer)
  undoTimer = null
  undoSnapshot.value = null
  undoDeletedCount.value = 0
  undoExpectedUpdatedAt.value = ''
}

function undoDelete() {
  const snapshot = undoSnapshot.value
  if (!snapshot)
    return

  if (storeUpdatedAt.value !== undoExpectedUpdatedAt.value) {
    message.warning('词表已在其他位置更新，为避免覆盖新改动，本次撤销已取消')
    clearUndo()
    return
  }

  if (!replaceSuggestions(snapshot)) {
    message.error('撤销失败')
    return
  }

  message.success(`已恢复 ${undoDeletedCount.value} 条幽灵补全`)
  clearUndo()
}
</script>

<template>
  <n-space vertical size="large">
    <div class="flex items-center justify-between gap-3">
      <div class="text-sm opacity-70">
        已启用 {{ enabledSuggestions.length }} / 共 {{ suggestions.length }}
        <span v-if="hasSearchQuery"> · 找到 {{ filteredSuggestions.length }} 条</span>
      </div>
      <n-space size="small">
        <n-button v-if="!batchMode" size="small" secondary @click="setBatchMode(true)">
          <template #icon>
            <div class="i-carbon-checkbox-checked w-4 h-4" />
          </template>
          批量管理
        </n-button>
        <n-button v-if="!batchMode" type="primary" size="small" @click="openAddEditor">
          <template #icon>
            <div class="i-carbon-add w-4 h-4" />
          </template>
          添加
        </n-button>
      </n-space>
    </div>

    <n-input
      v-model:value="searchQuery"
      clearable
      placeholder="优先匹配完整词和开头；同类结果按手动优先级"
      aria-label="搜索幽灵补全词"
    >
      <template #prefix>
        <div class="i-carbon-search w-4 h-4 opacity-55" />
      </template>
    </n-input>

    <div v-if="batchMode" class="ghost-batch-toolbar">
      <n-checkbox
        :checked="allVisibleSelected"
        :indeterminate="someVisibleSelected"
        @update:checked="toggleVisibleSelection"
      >
        全选当前 {{ sortableSuggestions.length }} 条
      </n-checkbox>
      <span class="ghost-batch-toolbar__count">已选 {{ selectedIds.length }} 条</span>
      <div class="ghost-batch-toolbar__actions">
        <n-button size="tiny" :disabled="selectedIds.length === 0" @click="handleBatchEnabled(true)">
          启用
        </n-button>
        <n-button size="tiny" :disabled="selectedIds.length === 0" @click="handleBatchEnabled(false)">
          停用
        </n-button>
        <n-button size="tiny" type="error" :disabled="selectedIds.length === 0" @click="openDelete(selectedIds)">
          删除
        </n-button>
        <n-button size="tiny" quaternary @click="setBatchMode(false)">
          完成
        </n-button>
      </div>
    </div>

    <n-alert v-if="undoSnapshot" type="warning" :show-icon="false" class="ghost-undo-alert">
      已删除 {{ undoDeletedCount }} 条幽灵补全，可在 10 秒内撤销。
      <template #action>
        <n-button size="tiny" secondary @click="undoDelete">
          撤销删除
        </n-button>
      </template>
    </n-alert>

    <div v-if="!batchMode && filteredSuggestions.length > 1" class="ghost-order-hint">
      <span class="i-carbon-drag-vertical h-4 w-4" />
      {{ hasSearchQuery ? '拖动搜索结果可调整同类匹配内的补全优先级' : '拖动手柄调整幽灵补全优先级' }}
    </div>

    <n-empty
      v-if="filteredSuggestions.length === 0"
      :description="hasSearchQuery ? '没有匹配的幽灵补全词条' : '暂无幽灵补全词条'"
    />

    <div v-else class="ghost-table-wrap">
      <table class="ghost-table">
        <thead>
          <tr>
            <th class="ghost-table__priority">
              优先
            </th>
            <th v-if="batchMode" class="ghost-table__selection">
              选择
            </th>
            <th class="ghost-table__enabled">
              状态
            </th>
            <th>触发词</th>
            <th>描述</th>
            <th class="ghost-table__actions">
              操作
            </th>
          </tr>
        </thead>
        <tbody ref="sortableContainer">
          <tr
            v-for="(item, index) in sortableSuggestions"
            :key="item.id"
            :data-suggestion-id="item.id"
            :class="{ 'ghost-table__row--selected': selectedIdSet.has(item.id) }"
          >
            <td class="ghost-table__priority-cell">
              <button
                v-if="!batchMode"
                type="button"
                class="ghost-drag-handle"
                :aria-label="`拖动调整 ${item.key} 的优先级`"
                :title="`拖动调整 ${item.key} 的优先级；方向键也可移动`"
                @keydown.up.prevent="moveVisibleSuggestion(index, -1)"
                @keydown.down.prevent="moveVisibleSuggestion(index, 1)"
              >
                <span class="i-carbon-drag-vertical h-4 w-4" />
              </button>
              <span class="ghost-priority-number">{{ index + 1 }}</span>
            </td>
            <td v-if="batchMode" class="ghost-table__selection">
              <n-checkbox
                :checked="selectedIdSet.has(item.id)"
                :aria-label="`选择 ${item.key}`"
                @update:checked="(checked: boolean) => toggleSelection(item.id, checked)"
              />
            </td>
            <td>
              <n-switch
                :value="item.enabled"
                size="small"
                :disabled="batchMode"
                @update:value="(enabled: boolean) => handleToggle(item, enabled)"
              />
            </td>
            <td>
              <code class="ghost-table__key">{{ item.key }}</code>
            </td>
            <td>
              <span v-if="item.description" class="ghost-table__description">{{ item.description }}</span>
              <span v-else class="ghost-table__muted">-</span>
            </td>
            <td class="ghost-table__actions-cell">
              <div v-if="!batchMode" class="ghost-table__action-list">
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button class="ghost-row-action ghost-row-action--edit" size="tiny" quaternary circle @click="openEditEditor(item)">
                      <template #icon>
                        <div class="i-carbon-edit w-4 h-4" />
                      </template>
                    </n-button>
                  </template>
                  编辑
                </n-tooltip>
                <n-tooltip trigger="hover">
                  <template #trigger>
                    <n-button class="ghost-row-action ghost-row-action--delete" size="tiny" type="error" quaternary circle @click="openDelete([item.id])">
                      <template #icon>
                        <div class="i-carbon-trash-can w-4 h-4" />
                      </template>
                    </n-button>
                  </template>
                  删除
                </n-tooltip>
              </div>
              <span v-else class="ghost-table__muted">-</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </n-space>

  <n-modal
    v-model:show="showEditor"
    preset="dialog"
    :title="editorTitle"
    positive-text="保存"
    negative-text="取消"
    @positive-click="handleSave"
  >
    <n-form label-placement="top" class="mt-4">
      <n-form-item label="触发词">
        <n-input v-model:value="form.key" placeholder="例如 hui、ji、sync" maxlength="32" />
      </n-form-item>
      <n-form-item label="描述">
        <n-input v-model:value="form.description" placeholder="显示在候选列表中的说明" maxlength="80" />
      </n-form-item>
      <n-form-item label="启用">
        <n-switch v-model:value="form.enabled" />
      </n-form-item>
    </n-form>
  </n-modal>

  <n-modal
    :show="pendingDeleteIds.length > 0"
    preset="dialog"
    :title="deleteDialogTitle"
    positive-text="删除"
    negative-text="取消"
    type="warning"
    @positive-click="handleDelete"
    @negative-click="pendingDeleteIds = []"
    @close="pendingDeleteIds = []"
  >
    <div class="mt-3 text-sm">
      {{ deleteDialogDescription }}
    </div>
  </n-modal>
</template>

<style scoped>
.ghost-batch-toolbar {
  display: flex;
  min-height: 36px;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  border: 1px solid rgba(96, 165, 250, 0.32);
  border-radius: 8px;
  background: rgba(59, 130, 246, 0.08);
}

.ghost-batch-toolbar__count {
  color: var(--color-on-surface-secondary);
  font-size: 12px;
}

.ghost-batch-toolbar__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-left: auto;
}

.ghost-order-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--color-on-surface-muted);
  font-size: 12px;
}

.ghost-undo-alert {
  align-items: center;
}

.ghost-table-wrap {
  overflow-x: auto;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.ghost-table {
  width: 100%;
  min-width: 680px;
  border-collapse: collapse;
  table-layout: fixed;
}

.ghost-table th,
.ghost-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-divider);
  text-align: left;
  vertical-align: middle;
}

.ghost-table th {
  color: var(--color-on-surface-secondary);
  font-size: 12px;
  font-weight: 500;
}

.ghost-table tbody tr:last-child td {
  border-bottom: none;
}

.ghost-table tbody tr {
  transition:
    background-color 180ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 180ms cubic-bezier(0.16, 1, 0.3, 1),
    opacity 180ms cubic-bezier(0.16, 1, 0.3, 1),
    transform 180ms cubic-bezier(0.16, 1, 0.3, 1);
}

.ghost-table tbody tr:hover,
.ghost-table tbody tr:focus-within {
  background: rgba(96, 165, 250, 0.08);
}

.ghost-table__row--selected {
  background: rgba(59, 130, 246, 0.1);
}

.ghost-table__priority {
  width: 86px;
}

.ghost-table__selection {
  width: 62px;
  text-align: center !important;
}

.ghost-table__enabled {
  width: 76px;
}

.ghost-table__actions {
  width: 88px;
}

.ghost-table__actions,
.ghost-table__actions-cell {
  position: sticky;
  right: 0;
  z-index: 1;
  background: var(--color-container);
  box-shadow: -12px 0 18px -18px var(--color-on-surface-muted);
}

.ghost-table__actions {
  z-index: 2;
}

.ghost-table tbody tr:hover .ghost-table__actions-cell,
.ghost-table tbody tr:focus-within .ghost-table__actions-cell {
  background: color-mix(in srgb, var(--color-container) 92%, #60a5fa 8%);
}

.ghost-table__row--selected .ghost-table__actions-cell,
.sortable-chosen .ghost-table__actions-cell {
  background: color-mix(in srgb, var(--color-container) 90%, #3b82f6 10%);
}

.ghost-table__priority-cell {
  white-space: nowrap;
}

.ghost-drag-handle {
  display: inline-flex;
  width: 28px;
  height: 28px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--color-on-surface-muted);
  cursor: grab;
  transition:
    color 140ms ease-out,
    background-color 140ms ease-out,
    transform 140ms ease-out;
  vertical-align: middle;
}

.ghost-drag-handle:hover,
.ghost-drag-handle:focus-visible {
  background: rgba(96, 165, 250, 0.14);
  color: #93c5fd;
  outline: none;
}

.ghost-drag-handle:active {
  cursor: grabbing;
  transform: scale(0.94);
}

.ghost-priority-number {
  display: inline-block;
  min-width: 20px;
  margin-left: 5px;
  color: var(--color-on-surface-muted);
  font-size: 11px;
  text-align: right;
  vertical-align: middle;
}

.ghost-table__key {
  display: inline-block;
  max-width: 100%;
  padding: 2px 6px;
  overflow: hidden;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-on-surface) 8%, transparent);
  color: color-mix(in srgb, #3b82f6 70%, var(--color-on-surface));
  font-size: 12px;
  text-overflow: ellipsis;
  vertical-align: middle;
  white-space: nowrap;
}

.ghost-table__description {
  display: block;
  overflow: hidden;
  color: var(--color-on-surface-secondary);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ghost-table__muted {
  color: var(--color-on-surface-muted);
}

.ghost-row-action {
  --n-text-color: var(--color-on-surface) !important;
  --n-text-color-hover: #2563eb !important;
  --n-text-color-pressed: #1d4ed8 !important;
  --n-color: color-mix(in srgb, var(--color-on-surface) 7%, transparent) !important;
  --n-color-hover: color-mix(in srgb, #3b82f6 15%, transparent) !important;
  --n-color-pressed: color-mix(in srgb, #3b82f6 22%, transparent) !important;
}

.ghost-row-action--delete {
  --n-text-color: #dc2626 !important;
  --n-text-color-hover: #b91c1c !important;
  --n-text-color-pressed: #991b1b !important;
}

.ghost-table__action-list {
  display: grid;
  grid-template-columns: repeat(2, 28px);
  gap: 2px;
  justify-content: end;
  opacity: 0;
  pointer-events: none;
  transform: translateX(4px);
  transition:
    opacity 140ms ease-out,
    transform 140ms ease-out;
}

.ghost-table tbody tr:hover .ghost-table__action-list,
.ghost-table tbody tr:focus-within .ghost-table__action-list,
.sortable-chosen .ghost-table__action-list,
.sortable-drag .ghost-table__action-list {
  opacity: 1;
  pointer-events: auto;
  transform: translateX(0);
}

.sortable-ghost {
  opacity: 0.28;
}

.sortable-chosen {
  background: rgba(59, 130, 246, 0.12);
}

.sortable-drag {
  background: var(--color-container);
  box-shadow: 0 14px 30px rgba(0, 0, 0, 0.28);
  transform: scale(1.008);
}

@media (hover: none), (pointer: coarse) {
  .ghost-table__action-list {
    opacity: 1;
    pointer-events: auto;
    transform: translateX(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .ghost-table tbody tr,
  .ghost-drag-handle,
  .ghost-table__action-list {
    transition-duration: 0.01ms;
  }
}
</style>
