<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

interface WindowInstance {
  pid: number
  project_path: string
  window_title: string
  registered_at: string
}

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const instances = ref<WindowInstance[]>([])
const selectedIndex = ref(0)
const loading = ref(false)

// 当前进程 ID
const currentPid = ref(0)

// 所有窗口实例（包括当前窗口）
const allInstances = computed(() => instances.value)

// 加载所有窗口实例
async function loadInstances() {
  loading.value = true
  try {
    instances.value = await invoke<WindowInstance[]>('get_all_window_instances')
    // 获取当前进程 ID（通过实例列表中最新注册的）
    if (instances.value.length > 0) {
      // 当前窗口应该是最近注册的
      const sorted = [...instances.value].sort((a, b) =>
        new Date(b.registered_at).getTime() - new Date(a.registered_at).getTime(),
      )
      currentPid.value = sorted[0]?.pid || 0
    }
  }
  catch (error) {
    console.error('加载窗口实例失败:', error)
  }
  finally {
    loading.value = false
  }
}

// 激活选中的窗口
async function activateSelected(clickedIndex?: number) {
  console.log('[DEBUG] activateSelected 被调用')
  console.log('[DEBUG] 传入的 clickedIndex:', clickedIndex)
  console.log('[DEBUG] 当前 selectedIndex.value:', selectedIndex.value)
  console.log('[DEBUG] allInstances.value.length:', allInstances.value.length)

  if (allInstances.value.length === 0) {
    console.log('[DEBUG] 没有实例，关闭')
    emit('close')
    return
  }

  // 如果传入了点击的索引，使用它；否则使用 selectedIndex
  const targetIndex = clickedIndex !== undefined ? clickedIndex : selectedIndex.value
  console.log('[DEBUG] 最终使用的 targetIndex:', targetIndex)

  const selected = allInstances.value[targetIndex]
  console.log('[DEBUG] 选中的实例:', selected)
  console.log('[DEBUG] 所有实例:', JSON.stringify(allInstances.value, null, 2))

  if (selected) {
    try {
      console.log('[DEBUG] 调用 activate_window_instance，PID:', selected.pid)
      await invoke('activate_window_instance', { pid: selected.pid })
      console.log('[DEBUG] 激活成功')
    }
    catch (error) {
      console.error('[DEBUG] 激活窗口失败:', error)
    }
    finally {
      emit('close')
    }
  }
  else {
    console.log('[DEBUG] selected 为空，关闭')
    emit('close')
  }
}

// 键盘事件处理
function handleKeydown(event: KeyboardEvent) {
  if (!props.visible)
    return

  switch (event.key) {
    case 'ArrowUp':
      event.preventDefault()
      event.stopPropagation()
      if (allInstances.value.length > 0) {
        if (selectedIndex.value > 0) {
          selectedIndex.value--
        }
        else {
          selectedIndex.value = allInstances.value.length - 1
        }
      }
      break
    case 'ArrowDown':
      event.preventDefault()
      event.stopPropagation()
      if (allInstances.value.length > 0) {
        if (selectedIndex.value < allInstances.value.length - 1) {
          selectedIndex.value++
        }
        else {
          selectedIndex.value = 0
        }
      }
      break
    case 'Enter':
      event.preventDefault()
      event.stopPropagation()
      activateSelected()
      break
    case 'Escape':
    case 'Tab':
      event.preventDefault()
      event.stopPropagation()
      emit('close')
      break
  }
}

// 监听可见性变化
watch(() => props.visible, (visible) => {
  if (visible) {
    loadInstances()
    selectedIndex.value = 0
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

// 从路径中提取项目名称
function getProjectName(path: string): string {
  const parts = path.split('/')
  return parts[parts.length - 1] || path
}
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="visible"
        class="fixed inset-0 z-[9999] flex items-start justify-center pt-20"
        @click.self="$emit('close')"
      >
        <!-- 背景遮罩 -->
        <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" />

        <!-- 选择器面板 -->
        <div class="relative z-10 w-[500px] max-h-[400px] bg-surface-50 rounded-xl shadow-2xl overflow-hidden border border-border">
          <!-- 标题 -->
          <div class="px-4 py-3 border-b border-border text-sm text-on-surface-secondary font-medium">
            选择切换的窗口
          </div>

          <!-- 窗口列表 -->
          <div class="overflow-y-auto max-h-[320px]">
            <template v-if="loading">
              <div class="px-4 py-8 text-center text-on-surface-muted">
                加载中...
              </div>
            </template>

            <template v-else-if="allInstances.length === 0">
              <div class="px-4 py-8 text-center text-on-surface-muted">
                没有打开的窗口
              </div>
            </template>

            <template v-else>
              <div
                v-for="(instance, index) in allInstances"
                :key="instance.pid"
                class="flex items-center gap-3 px-4 py-3 cursor-pointer transition-all duration-150"
                :class="[
                  index === selectedIndex
                    ? 'bg-gray-200/80'
                    : 'hover:bg-gray-100/50',
                ]"
                @click="() => { console.log('[DEBUG] 点击了索引:', index); activateSelected(index) }"
                @mouseenter="selectedIndex = index"
              >
                <!-- iterate 图标 -->
                <img src="/icons/icon-256.png" alt="iterate" class="w-8 h-8 rounded-lg flex-shrink-0">

                <!-- 窗口信息 -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="font-semibold truncate text-gray-900">iterate</span>
                    <span class="text-gray-400">—</span>
                    <span class="truncate text-gray-600">{{ getProjectName(instance.project_path) }}</span>
                  </div>
                  <div class="text-xs truncate mt-0.5 text-gray-500">
                    {{ instance.project_path }}
                  </div>
                </div>
              </div>
            </template>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.selected-item {
  background-color: #e5e7eb;
}
</style>
