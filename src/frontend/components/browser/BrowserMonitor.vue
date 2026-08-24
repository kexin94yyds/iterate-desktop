<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useMessage } from 'naive-ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

interface AiCompletionEvent {
  url: string
  title: string
  site_name: string
  message_preview: string
  timestamp: string
}

interface PageState {
  url: string
  title: string
  site_name: string
  status: string
  last_check: string
}

interface BrowserMonitorStatus {
  connected: boolean
  monitoring: boolean
  pages: PageState[]
}

const message = useMessage()
const isMonitoring = ref(false)
const isConnecting = ref(false)
const status = ref<BrowserMonitorStatus | null>(null)
const completionEvents = ref<AiCompletionEvent[]>([])
const chromePort = ref(9222)

let unlistenCompletion: (() => void) | null = null

const monitoredPages = computed(() => status.value?.pages || [])

async function startMonitoring() {
  isConnecting.value = true
  try {
    await invoke('start_browser_monitoring', { port: chromePort.value })
    isMonitoring.value = true
    message.success('浏览器监控已启动')
    await refreshStatus()
  }
  catch (error: any) {
    message.error(`启动失败: ${error}`)
  }
  finally {
    isConnecting.value = false
  }
}

async function stopMonitoring() {
  try {
    await invoke('stop_browser_monitoring')
    isMonitoring.value = false
    status.value = null
    message.info('浏览器监控已停止')
  }
  catch (error: any) {
    message.error(`停止失败: ${error}`)
  }
}

async function refreshStatus() {
  try {
    status.value = await invoke('get_browser_monitor_status')
  }
  catch (error) {
    console.error('获取状态失败:', error)
  }
}

async function openUrl(url: string) {
  try {
    await invoke('open_browser_url', { url })
  }
  catch (error: any) {
    message.error(`打开 URL 失败: ${error}`)
  }
}

function clearEvents() {
  completionEvents.value = []
}

function formatTime(timestamp: string) {
  return new Date(timestamp).toLocaleTimeString()
}

onMounted(async () => {
  // 监听 AI 完成事件
  unlistenCompletion = await listen<AiCompletionEvent>('browser-ai-completed', (event) => {
    completionEvents.value.unshift(event.payload)
    // 只保留最近 20 条
    if (completionEvents.value.length > 20) {
      completionEvents.value = completionEvents.value.slice(0, 20)
    }
  })

  // 检查当前状态
  await refreshStatus()
  if (status.value?.monitoring) {
    isMonitoring.value = true
  }
})

onUnmounted(() => {
  if (unlistenCompletion) {
    unlistenCompletion()
  }
})
</script>

<template>
  <div class="browser-monitor p-4">
    <h2 class="text-lg font-bold mb-4">
      🌐 浏览器 AI 监控
    </h2>

    <!-- 连接设置 -->
    <div class="mb-4 p-3 bg-black-100 rounded-lg">
      <div class="flex items-center gap-4 mb-2">
        <label class="text-sm text-gray-400">Chrome 调试端口:</label>
        <input
          v-model.number="chromePort"
          type="number"
          class="w-24 px-2 py-1 bg-black-200 rounded text-sm"
          :disabled="isMonitoring"
        >
      </div>
      <p class="text-xs text-gray-500">
        请先以调试模式启动 Chrome：<br>
        <code class="text-blue-400">/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port={{ chromePort }}</code>
      </p>
    </div>

    <!-- 控制按钮 -->
    <div class="flex gap-2 mb-4">
      <button
        v-if="!isMonitoring"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-white text-sm"
        :disabled="isConnecting"
        @click="startMonitoring"
      >
        {{ isConnecting ? '连接中...' : '开始监控' }}
      </button>
      <button
        v-else
        class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded text-white text-sm"
        @click="stopMonitoring"
      >
        停止监控
      </button>
      <button
        v-if="isMonitoring"
        class="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded text-white text-sm"
        @click="refreshStatus"
      >
        刷新状态
      </button>
    </div>

    <!-- 监控状态 -->
    <div v-if="isMonitoring && status" class="mb-4">
      <h3 class="text-sm font-semibold mb-2 text-gray-300">
        监控中的页面 ({{ monitoredPages.length }})
      </h3>
      <div v-if="monitoredPages.length === 0" class="text-sm text-gray-500">
        暂无检测到支持的 AI 页面
      </div>
      <div v-else class="space-y-2">
        <div
          v-for="page in monitoredPages"
          :key="page.url"
          class="p-2 bg-black-200 rounded flex items-center justify-between"
        >
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium truncate">
              {{ page.site_name }}
            </div>
            <div class="text-xs text-gray-400 truncate">
              {{ page.title || page.url }}
            </div>
          </div>
          <div class="ml-2 flex items-center gap-2">
            <span
              class="px-2 py-0.5 text-xs rounded"
              :class="{
                'bg-green-600': page.status === 'Idle',
                'bg-yellow-600': page.status === 'Generating',
                'bg-blue-600': page.status === 'Completed',
              }"
            >
              {{ page.status === 'Generating' ? '生成中' : page.status === 'Completed' ? '已完成' : '空闲' }}
            </span>
            <button
              class="text-blue-400 hover:text-blue-300 text-xs"
              @click="openUrl(page.url)"
            >
              打开
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- AI 完成事件列表 -->
    <div v-if="completionEvents.length > 0">
      <div class="flex items-center justify-between mb-2">
        <h3 class="text-sm font-semibold text-gray-300">
          完成通知 ({{ completionEvents.length }})
        </h3>
        <button
          class="text-xs text-gray-400 hover:text-white"
          @click="clearEvents"
        >
          清空
        </button>
      </div>
      <div class="space-y-2 max-h-64 overflow-y-auto">
        <div
          v-for="(event, index) in completionEvents"
          :key="index"
          class="p-3 bg-black-200 rounded-lg cursor-pointer hover:bg-black-300 transition-colors"
          @click="openUrl(event.url)"
        >
          <div class="flex items-center justify-between mb-1">
            <span class="text-sm font-medium text-blue-400">
              {{ event.site_name }}
            </span>
            <span class="text-xs text-gray-500">
              {{ formatTime(event.timestamp) }}
            </span>
          </div>
          <div class="text-xs text-gray-400 truncate mb-1">
            {{ event.title }}
          </div>
          <div v-if="event.message_preview" class="text-xs text-gray-500 line-clamp-2">
            {{ event.message_preview }}
          </div>
          <div class="text-xs text-blue-400 mt-1 truncate hover:underline">
            {{ event.url }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
