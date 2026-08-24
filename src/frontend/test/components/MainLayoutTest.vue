<script setup lang="ts">
import { onMounted, ref } from 'vue'
import MainLayout from '../../components/layout/MainLayout.vue'
import { applyThemeVariables } from '../../theme'

// Props
const props = defineProps<{
  showControls?: boolean
}>()

// 默认显示控制面板
const showControls = ref(props.showControls !== false)

// 模拟主界面所需的 props
const currentTheme = ref('light')
const alwaysOnTop = ref(false)
const audioNotificationEnabled = ref(true)
const audioUrl = ref('')
const windowWidth = ref(600)
const windowHeight = ref(800)
const fixedWindowSize = ref(false)

// 模拟事件处理
function handleThemeChange(theme: string) {
  currentTheme.value = theme
  // 真正应用主题 CSS 变量，让沙盒里的黑/白主题可切换
  applyThemeVariables(theme)
  console.log('主题切换:', theme)
}

// 挂载时应用初始主题，保证沙盒与控制面板状态一致
onMounted(() => {
  applyThemeVariables(currentTheme.value)
})

function handleToggleAlwaysOnTop() {
  alwaysOnTop.value = !alwaysOnTop.value
  console.log('置顶切换:', alwaysOnTop.value)
}

function handleToggleAudioNotification() {
  audioNotificationEnabled.value = !audioNotificationEnabled.value
  console.log('音频通知切换:', audioNotificationEnabled.value)
}

function handleUpdateAudioUrl(url: string) {
  audioUrl.value = url
  console.log('音频URL更新:', url)
}

function handleTestAudio() {
  console.log('测试音频播放')
}

function handleUpdateWindowSize(size: { width: number, height: number, fixed: boolean }) {
  windowWidth.value = size.width
  windowHeight.value = size.height
  fixedWindowSize.value = size.fixed
  console.log('窗口大小更新:', size)
}
</script>

<template>
  <div class="main-layout-test">
    <!-- 控制面板模式 -->
    <div v-if="showControls">
      <n-card title="主界面布局测试 - 真实组件">
        <template #header-extra>
          <n-space>
            <n-tag size="small" type="info">
              引用: MainLayout.vue
            </n-tag>
            <n-button size="small" @click="handleThemeChange(currentTheme === 'dark' ? 'light' : 'dark')">
              切换主题
            </n-button>
          </n-space>
        </template>

        <!-- 控制面板 -->
        <div class="control-panel">
          <n-card title="测试控制" size="small">
            <n-space vertical size="medium">
              <n-space align="center" justify="space-between">
                <span>当前主题:</span>
                <n-tag size="small" :type="currentTheme === 'dark' ? 'warning' : 'info'">
                  {{ currentTheme }}
                </n-tag>
              </n-space>

              <n-space align="center" justify="space-between">
                <span>置顶状态:</span>
                <n-tag size="small" :type="alwaysOnTop ? 'success' : 'default'">
                  {{ alwaysOnTop ? '已启用' : '已禁用' }}
                </n-tag>
              </n-space>

              <n-space align="center" justify="space-between">
                <span>音频通知:</span>
                <n-tag size="small" :type="audioNotificationEnabled ? 'success' : 'default'">
                  {{ audioNotificationEnabled ? '已启用' : '已禁用' }}
                </n-tag>
              </n-space>

              <n-space align="center" justify="space-between">
                <span>音频URL:</span>
                <n-tag size="small" type="info">
                  {{ audioUrl || '(默认)' }}
                </n-tag>
              </n-space>

              <n-space align="center" justify="space-between">
                <span>窗口大小:</span>
                <n-tag size="small" type="info">
                  {{ windowWidth }} × {{ windowHeight }} / {{ fixedWindowSize ? '固定' : '自适应' }}
                </n-tag>
              </n-space>
            </n-space>
          </n-card>
        </div>

        <!-- 真实的主界面组件 -->
        <div class="main-layout-container">
          <MainLayout
            :current-theme="currentTheme"
            :always-on-top="alwaysOnTop"
            :audio-notification-enabled="audioNotificationEnabled"
            :audio-url="audioUrl"
            :window-width="windowWidth"
            :window-height="windowHeight"
            :fixed-window-size="fixedWindowSize"
            @theme-change="handleThemeChange"
            @toggle-always-on-top="handleToggleAlwaysOnTop"
            @toggle-audio-notification="handleToggleAudioNotification"
            @update-audio-url="handleUpdateAudioUrl"
            @test-audio="handleTestAudio"
            @update-window-size="handleUpdateWindowSize"
          />
        </div>

        <!-- 说明信息 -->
        <div class="info-panel">
          <n-card title="测试说明" size="small">
            <n-space vertical size="small">
              <div class="flex items-center text-sm">
                <div class="w-1.5 h-1.5 bg-green-500 rounded-full mr-3 flex-shrink-0" />
                这是真实的 MainLayout 组件，所有修改都会实时反映
              </div>
              <div class="flex items-center text-sm">
                <div class="w-1.5 h-1.5 bg-green-500 rounded-full mr-3 flex-shrink-0" />
                可以测试主题切换、设置修改等功能
              </div>
              <div class="flex items-center text-sm">
                <div class="w-1.5 h-1.5 bg-green-500 rounded-full mr-3 flex-shrink-0" />
                事件会在控制台输出，方便调试
              </div>
              <div class="flex items-center text-sm">
                <div class="w-1.5 h-1.5 bg-blue-500 rounded-full mr-3 flex-shrink-0" />
                <span class="opacity-70">src/frontend/components/layout/MainLayout.vue</span>
              </div>
            </n-space>
          </n-card>
        </div>
      </n-card>
    </div>

    <!-- 纯净模式 - 只显示主界面 -->
    <div v-else class="pure-mode">
      <MainLayout
        :current-theme="currentTheme"
        :always-on-top="alwaysOnTop"
        :audio-notification-enabled="audioNotificationEnabled"
        :audio-url="audioUrl"
        :window-width="windowWidth"
        :window-height="windowHeight"
        :fixed-window-size="fixedWindowSize"
        @theme-change="handleThemeChange"
        @toggle-always-on-top="handleToggleAlwaysOnTop"
        @toggle-audio-notification="handleToggleAudioNotification"
        @update-audio-url="handleUpdateAudioUrl"
        @test-audio="handleTestAudio"
        @update-window-size="handleUpdateWindowSize"
      />
    </div>
  </div>
</template>

<style scoped>
.main-layout-test {
  max-width: 1200px;
  margin: 0 auto;
}

.control-panel {
  margin-bottom: 20px;
}

.main-layout-container {
  margin: 20px 0;
  border: 2px dashed var(--border-color);
  border-radius: 8px;
  padding: 20px;
  background: var(--card-color);
}

.info-panel {
  margin-top: 20px;
}

/* 纯净模式 */
.pure-mode {
  width: 100%;
  height: 100%;
}
</style>
