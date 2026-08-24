<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { onMounted, reactive, ref } from 'vue'

interface CheckpointInfo {
  id: string
  checkpoint_id?: string
  name: string
  timestamp: string
  files: string[]
  message: string
  kind?: string
  deletable?: boolean
}

interface RestoreFileChange {
  path: string
  action: 'restore' | 'delete'
  exists_in_commit: boolean
}

interface RestoreCheckpointPreview {
  ok: boolean
  dry_run: boolean
  mode?: 'undo_change' | 'restore_snapshot' | string | null
  restore_plan_id?: string | null
  restore_plan_hash?: string | null
  status_snapshot?: unknown
  plan_expires_at?: string | null
  target_commit: string
  head_before?: string | null
  safety_checkpoint?: CheckpointInfo | null
  will_create_safety_checkpoint: boolean
  changed_files: RestoreFileChange[]
  warnings: string[]
  diff_summary?: string | null
}

interface Props {
  projectPath?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  close: []
}>()

const message = useMessage()
const checkpoints = ref<CheckpointInfo[]>([])
const loading = ref(false)
const expandedId = ref<string | null>(null)
const creating = ref(false)
const newCheckpointName = ref('')
const restoreModalVisible = ref(false)
const restoreModalLoading = ref(false)
const restoreTarget = ref<CheckpointInfo | null>(null)
const restorePreview = ref<RestoreCheckpointPreview | null>(null)
const restoreSelectedFiles = ref<string[]>([])
const selectedFilesByCheckpoint = reactive<Record<string, string[]>>({})

async function loadCheckpoints() {
  if (!props.projectPath) {
    console.log('CheckpointPanel: No project path provided')
    return
  }

  loading.value = true
  try {
    console.log('CheckpointPanel: Loading checkpoints for', props.projectPath)
    checkpoints.value = await invoke('list_checkpoints', {
      projectPath: props.projectPath,
    })
    for (const checkpoint of checkpoints.value) {
      const current = selectedFilesByCheckpoint[checkpoint.id]
      if (!current || current.length === 0) {
        selectedFilesByCheckpoint[checkpoint.id] = [...checkpoint.files]
        continue
      }

      const next = current.filter(file => checkpoint.files.includes(file))
      selectedFilesByCheckpoint[checkpoint.id] = next.length > 0 ? next : [...checkpoint.files]
    }
    for (const key of Object.keys(selectedFilesByCheckpoint)) {
      if (!checkpoints.value.some(checkpoint => checkpoint.id === key)) {
        delete selectedFilesByCheckpoint[key]
      }
    }
    console.log('CheckpointPanel: Loaded', checkpoints.value.length, 'checkpoints')
  }
  catch (error) {
    console.error('加载检查点失败:', error)
  }
  finally {
    loading.value = false
  }
}

function getSelectedFiles(checkpoint: CheckpointInfo): string[] {
  return selectedFilesByCheckpoint[checkpoint.id] ?? []
}

function setSelectedFiles(checkpointId: string, files: string[]) {
  selectedFilesByCheckpoint[checkpointId] = [...files]
}

function closeRestoreModal() {
  restoreModalVisible.value = false
  restoreTarget.value = null
  restorePreview.value = null
  restoreSelectedFiles.value = []
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  if (typeof error === 'string') {
    return error
  }

  try {
    return JSON.stringify(error)
  }
  catch {
    return String(error)
  }
}

function getRestoreModeLabel(preview: RestoreCheckpointPreview): string {
  if (preview.mode === 'undo_change') {
    return '撤销某轮改动（undo_change）'
  }

  return '恢复到快照（restore_snapshot）'
}

function getPlanHashShort(preview: RestoreCheckpointPreview): string {
  const hash = preview.restore_plan_hash?.trim()
  return hash ? hash.slice(0, 12) : '缺失'
}

async function handleRestore(checkpoint: CheckpointInfo) {
  if (!props.projectPath) {
    return
  }

  const selectedFiles = getSelectedFiles(checkpoint)
  if (selectedFiles.length === 0) {
    message.warning('请先选择要恢复的文件')
    return
  }

  restoreModalLoading.value = true
  try {
    const preview = await invoke<RestoreCheckpointPreview>('restore_checkpoint_safe', {
      projectPath: props.projectPath,
      stashId: checkpoint.id,
      dryRun: true,
      createSafetyCheckpoint: true,
      selectedFiles,
    })
    restoreTarget.value = checkpoint
    restorePreview.value = preview
    restoreSelectedFiles.value = [...selectedFiles]
    restoreModalVisible.value = true
  }
  catch (error) {
    message.error(`预览恢复失败: ${formatError(error)}`)
  }
  finally {
    restoreModalLoading.value = false
  }
}

async function confirmRestore() {
  if (!props.projectPath || !restoreTarget.value || !restorePreview.value) {
    return
  }

  const selectedFiles = [...restoreSelectedFiles.value]
  if (selectedFiles.length === 0) {
    message.warning('请先选择要恢复的文件')
    return
  }

  const expectedPlanHash = restorePreview.value.restore_plan_hash?.trim()
  if (!expectedPlanHash) {
    message.error('缺少恢复计划 hash，请重新预览后再确认')
    return
  }

  restoreModalLoading.value = true
  try {
    const result = await invoke<RestoreCheckpointPreview>('restore_checkpoint_safe', {
      projectPath: props.projectPath,
      stashId: restoreTarget.value.id,
      dryRun: false,
      createSafetyCheckpoint: true,
      selectedFiles,
      expectedPlanHash,
    })
    const safety = result.safety_checkpoint
    if (safety) {
      message.success(`已恢复到: ${restoreTarget.value.name || restoreTarget.value.id}，恢复前已保存为 ${safety.checkpoint_id || safety.id}`)
    }
    else {
      message.success(`已恢复到: ${restoreTarget.value.name || restoreTarget.value.id}`)
    }
    closeRestoreModal()
    emit('close')
    await loadCheckpoints()
  }
  catch (error) {
    const errorMessage = formatError(error)
    if (errorMessage.includes('预览已过期')) {
      message.error('预览已过期，请重新预览后再确认')
    }
    else {
      message.error(`恢复失败: ${errorMessage}`)
    }
  }
  finally {
    restoreModalLoading.value = false
  }
}

async function handleDelete(checkpoint: CheckpointInfo) {
  if (!props.projectPath) {
    return
  }

  try {
    await invoke('delete_checkpoint', {
      projectPath: props.projectPath,
      stashId: checkpoint.id,
    })
    message.success('检查点已删除')
    await loadCheckpoints()
  }
  catch (error) {
    message.error(`删除失败: ${error}`)
  }
}

async function handleCreate() {
  if (!props.projectPath) {
    return
  }

  const name = newCheckpointName.value.trim() || `检查点 ${new Date().toLocaleTimeString('zh-CN')}`
  creating.value = true
  try {
    await invoke('create_checkpoint', {
      projectPath: props.projectPath,
      message: name,
    })
    message.success(`已创建检查点: ${name}`)
    newCheckpointName.value = ''
    await loadCheckpoints()
  }
  catch (error) {
    message.error(`创建失败: ${error}`)
  }
  finally {
    creating.value = false
  }
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()

  if (diff < 60000) {
    return '刚刚'
  }
  if (diff < 3600000) {
    return `${Math.floor(diff / 60000)} 分钟前`
  }
  if (diff < 86400000) {
    return `${Math.floor(diff / 3600000)} 小时前`
  }

  return date.toLocaleString('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function getFileName(path: string): string {
  return path.split('/').pop() || path
}

function getShortHash(id: string): string {
  return id.slice(0, 8)
}

function getPreviewFiles(files: string[]): string[] {
  return files.slice(0, 3)
}

function getRemainingFileCount(files: string[]): number {
  return Math.max(files.length - 3, 0)
}

function getCheckpointKindLabel(checkpoint: CheckpointInfo): string {
  return checkpoint.kind === 'stash' ? 'Stash' : 'Git'
}

function getCheckpointKindType(checkpoint: CheckpointInfo): 'default' | 'info' | 'success' {
  return checkpoint.kind === 'stash' ? 'default' : 'success'
}

onMounted(() => {
  loadCheckpoints()
})
</script>

<template>
  <div class="checkpoint-panel">
    <div class="panel-header">
      <h3 class="panel-title">
        <div class="i-carbon-reset w-4 h-4" />
        检查点
      </h3>
      <n-button quaternary circle size="small" @click="emit('close')">
        <template #icon>
          <div class="i-carbon-close w-4 h-4" />
        </template>
      </n-button>
    </div>

    <div class="create-section">
      <n-input
        v-model:value="newCheckpointName"
        placeholder="检查点名称（可选）"
        size="small"
        :disabled="creating"
        @keyup.enter="handleCreate"
      />
      <n-button
        type="primary"
        size="small"
        :loading="creating"
        :disabled="!props.projectPath"
        @click="handleCreate"
      >
        <template #icon>
          <div class="i-carbon-add w-4 h-4" />
        </template>
        创建
      </n-button>
    </div>

    <div class="panel-content">
      <n-spin :show="loading">
        <div v-if="checkpoints.length > 0" class="impact-hint">
          恢复会覆盖这些文件在当前工作区的内容，请先确认边界。
        </div>

        <div v-if="checkpoints.length === 0 && !loading" class="empty-state">
          <div class="i-carbon-document-blank w-8 h-8 text-gray-400" />
          <p class="text-gray-400 text-sm mt-2">
            暂无检查点
          </p>
        </div>

        <div v-else class="checkpoint-list">
          <div
            v-for="cp in checkpoints"
            :key="cp.id"
            class="checkpoint-item"
          >
            <div class="checkpoint-header" @click="toggleExpand(cp.id)">
              <div class="checkpoint-info">
                <div class="checkpoint-time">
                  {{ formatTime(cp.timestamp) }}
                </div>
                <div class="checkpoint-name">
                  {{ cp.name || cp.id }}
                </div>
                <div class="checkpoint-meta">
                  <span v-if="cp.checkpoint_id">{{ cp.checkpoint_id }}</span>
                  <span>{{ getShortHash(cp.id) }}</span>
                </div>
                <div v-if="cp.files.length > 0" class="checkpoint-preview">
                  <span
                    v-for="file in getPreviewFiles(cp.files)"
                    :key="`${cp.id}-${file}`"
                    class="preview-file"
                    :title="file"
                  >
                    {{ file }}
                  </span>
                  <span
                    v-if="getRemainingFileCount(cp.files) > 0"
                    class="preview-more"
                  >
                    +{{ getRemainingFileCount(cp.files) }}
                  </span>
                </div>
              </div>
              <div class="checkpoint-actions">
                <n-tag
                  size="small"
                  :bordered="false"
                  :type="getCheckpointKindType(cp)"
                >
                  {{ getCheckpointKindLabel(cp) }}
                </n-tag>
                <n-tag
                  v-if="cp.files.length > 0"
                  size="small"
                  :bordered="false"
                  type="info"
                >
                  {{ cp.files.length }} 文件
                </n-tag>
                <n-button
                  size="tiny"
                  type="primary"
                  ghost
                  @click.stop="handleRestore(cp)"
                >
                  恢复
                </n-button>
                <n-button
                  v-if="cp.deletable"
                  size="tiny"
                  quaternary
                  @click.stop="handleDelete(cp)"
                >
                  <template #icon>
                    <div class="i-carbon-trash-can w-3 h-3" />
                  </template>
                </n-button>
              </div>
            </div>

            <n-collapse-transition :show="expandedId === cp.id">
              <div class="checkpoint-files">
                <n-checkbox-group
                  :value="getSelectedFiles(cp)"
                  @update:value="value => setSelectedFiles(cp.id, value)"
                >
                  <div
                    v-for="file in cp.files"
                    :key="file"
                    class="file-item"
                  >
                    <n-checkbox :value="file">
                      <span class="file-name" :title="file">
                        {{ getFileName(file) }}
                      </span>
                    </n-checkbox>
                  </div>
                </n-checkbox-group>
              </div>
            </n-collapse-transition>
          </div>
        </div>
      </n-spin>
    </div>
  </div>

  <n-modal
    v-model:show="restoreModalVisible"
    preset="card"
    title="文件级恢复预览"
    style="width: 640px; max-width: calc(100vw - 32px);"
  >
    <div class="restore-preview-modal">
      <div v-if="restorePreview" class="restore-preview-notice">
        <div>当前是文件级恢复，不是函数/hunk 级。</div>
        <div>模式：{{ getRestoreModeLabel(restorePreview) }}</div>
        <div>不会执行 git reset。</div>
      </div>

      <div v-if="restorePreview" class="restore-preview-summary">
        <div class="restore-preview-line">
          <span>目标提交</span>
          <code>{{ restorePreview.target_commit.slice(0, 8) }}</code>
        </div>
        <div v-if="restorePreview.head_before" class="restore-preview-line">
          <span>当前 HEAD</span>
          <code>{{ restorePreview.head_before.slice(0, 8) }}</code>
        </div>
        <div class="restore-preview-line">
          <span>恢复范围</span>
          <span>{{ restorePreview.changed_files.length }} 个文件</span>
        </div>
        <div class="restore-preview-line">
          <span>安全检查点</span>
          <span>{{ restorePreview.will_create_safety_checkpoint ? '恢复前会创建' : '无需创建' }}</span>
        </div>
        <div class="restore-preview-line">
          <span>Plan hash</span>
          <code :class="{ 'missing-plan-hash': !restorePreview.restore_plan_hash }">
            {{ getPlanHashShort(restorePreview) }}
          </code>
        </div>
        <div v-if="restorePreview.diff_summary" class="restore-preview-line">
          <span>变更摘要</span>
          <span>{{ restorePreview.diff_summary }}</span>
        </div>
      </div>

      <div v-if="restorePreview && !restorePreview.restore_plan_hash" class="restore-preview-error">
        缺少恢复计划 hash，不能执行恢复。请重新预览。
      </div>

      <div v-if="restorePreview?.warnings?.length" class="restore-preview-warnings">
        <div
          v-for="warning in restorePreview.warnings"
          :key="warning"
          class="restore-warning-item"
        >
          {{ warning }}
        </div>
      </div>

      <div class="restore-preview-files">
        <div
          v-for="file in restorePreview?.changed_files || []"
          :key="file.path"
          class="restore-preview-file"
        >
          <n-tag size="small" :bordered="false" :type="file.action === 'restore' ? 'success' : 'warning'">
            {{ file.action === 'restore' ? '覆盖' : '删除' }}
          </n-tag>
          <span class="restore-preview-path" :title="file.path">
            {{ file.path }}
          </span>
        </div>
      </div>

      <div class="restore-preview-actions">
        <n-button @click="closeRestoreModal">
          取消
        </n-button>
        <n-button
          type="primary"
          :loading="restoreModalLoading"
          :disabled="!restorePreview?.restore_plan_hash"
          @click="confirmRestore"
        >
          确认恢复这 {{ restorePreview?.changed_files.length || 0 }} 个文件
        </n-button>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.checkpoint-panel {
  background: var(--n-color);
  border-radius: 8px;
  overflow: hidden;
  max-height: 400px;
  display: flex;
  flex-direction: column;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--n-border-color);
}

.create-section {
  display: flex;
  gap: 8px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--n-border-color);
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  margin: 0;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.impact-hint {
  margin-bottom: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 12px;
  color: #8a5a00;
  background: rgba(255, 196, 87, 0.18);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px;
}

.checkpoint-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.checkpoint-item {
  background: var(--n-color-modal);
  border-radius: 6px;
  overflow: hidden;
}

.checkpoint-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.checkpoint-header:hover {
  background: var(--n-color-hover);
}

.checkpoint-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.checkpoint-time {
  font-size: 11px;
  color: var(--n-text-color-3);
}

.checkpoint-name {
  font-size: 13px;
  font-weight: 500;
}

.checkpoint-meta {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: var(--n-text-color-3);
}

.checkpoint-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.checkpoint-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-top: 4px;
}

.preview-file,
.preview-more {
  max-width: 220px;
  padding: 1px 6px;
  border-radius: 999px;
  font-size: 11px;
  line-height: 18px;
  color: var(--n-text-color-2);
  background: var(--n-tag-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-more {
  color: var(--n-text-color-3);
}

.checkpoint-files {
  padding: 8px 12px;
  background: var(--n-color);
  border-top: 1px solid var(--n-border-color);
}

.file-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  font-size: 12px;
}

.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.restore-preview-modal {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.restore-preview-summary {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border: 1px solid var(--n-border-color);
  border-radius: 6px;
  background: var(--n-color-modal);
  font-size: 12px;
}

.restore-preview-notice {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 6px;
  color: var(--n-text-color-2);
  background: var(--n-color-modal);
  font-size: 12px;
  line-height: 1.5;
}

.restore-preview-line {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.restore-preview-line span:first-child {
  color: var(--n-text-color-3);
}

.missing-plan-hash {
  color: #c97a00;
}

.restore-preview-error {
  padding: 8px 10px;
  border-radius: 6px;
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.12);
  font-size: 12px;
}

.restore-preview-warnings {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 6px;
  color: #8a5a00;
  background: rgba(255, 196, 87, 0.18);
  font-size: 12px;
}

.restore-warning-item {
  line-height: 1.4;
}

.restore-preview-files {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 220px;
  overflow-y: auto;
  padding-right: 2px;
}

.restore-preview-file {
  display: flex;
  align-items: center;
  gap: 8px;
}

.restore-preview-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}

.restore-preview-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 4px;
}
</style>
