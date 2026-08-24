<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { onMounted, ref } from 'vue'

const message = useMessage()

const autoCheckpointEnabled = ref(true)
const loading = ref(true)

async function loadConfig() {
  loading.value = true
  try {
    const enabled = await invoke('get_auto_checkpoint_enabled')
    autoCheckpointEnabled.value = enabled as boolean
  }
  catch (error) {
    console.error('加载自动检查点配置失败:', error)
  }
  finally {
    loading.value = false
  }
}

async function toggleAutoCheckpoint(value: boolean) {
  if (loading.value)
    return

  loading.value = true
  const previous = autoCheckpointEnabled.value
  autoCheckpointEnabled.value = value
  try {
    await invoke('set_auto_checkpoint_enabled', { enabled: value })
    message.success(value ? '已开启自动检查点' : '已关闭自动检查点')
  }
  catch (error) {
    console.error('切换自动检查点失败:', error)
    autoCheckpointEnabled.value = previous
    message.error('切换自动检查点失败')
  }
  finally {
    loading.value = false
  }
}

onMounted(() => {
  loadConfig()
})
</script>

<template>
  <n-space vertical size="large">
    <div class="flex items-center justify-between">
      <div class="flex items-center">
        <div class="w-1.5 h-1.5 bg-success rounded-full mr-3 flex-shrink-0" />
        <div>
          <div class="text-sm font-medium leading-relaxed">
            自动检查点
          </div>
          <div class="text-xs opacity-60">
            开启后每次 zhi 自动提交工作区改动，并在后台监控文件变化创建 checkpoint。关闭后将停止新的自动提交和后台监控触发，重启后保持关闭。
          </div>
        </div>
      </div>
      <n-switch
        :value="autoCheckpointEnabled"
        :loading="loading"
        size="small"
        @update:value="toggleAutoCheckpoint"
      />
    </div>
  </n-space>
</template>
