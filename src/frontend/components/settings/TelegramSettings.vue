<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useMessage } from 'naive-ui'
import { onMounted, ref } from 'vue'
import { API_BASE_URL, API_EXAMPLES } from '../../constants/telegram'

interface TelegramConfig {
  enabled: boolean
  bot_token: string
  chat_id: string
  hide_frontend_popup: boolean
  api_base_url: string
}

const emit = defineEmits(['telegramConfigChange'])

// Naive UI 消息实例
const message = useMessage()

// 配置状态
const telegramConfig = ref<TelegramConfig>({
  enabled: false,
  bot_token: '',
  chat_id: '',
  hide_frontend_popup: false,
  api_base_url: API_BASE_URL,
})

// 测试状态
const isTesting = ref(false)

// Chat ID自动获取状态
const isDetectingChatId = ref(false)
const detectedChatInfo = ref<any>(null)

// 设置向导状态
const showSetupWizard = ref(false)
const setupStep = ref(1)

// 加载Telegram配置
async function loadTelegramConfig() {
  try {
    const config = await invoke('get_telegram_config') as TelegramConfig
    telegramConfig.value = config
  }
  catch (error) {
    console.error('加载Telegram配置失败:', error)
    message.error('加载Telegram配置失败')
  }
}

// 保存配置
async function saveTelegramConfig() {
  try {
    await invoke('set_telegram_config', { telegramConfig: telegramConfig.value })
    message.success('Telegram配置已保存')
    emit('telegramConfigChange', telegramConfig.value)
  }
  catch (error) {
    console.error('保存Telegram配置失败:', error)
    message.error('保存Telegram配置失败')
  }
}

// 切换启用状态
async function toggleTelegramEnabled() {
  telegramConfig.value.enabled = !telegramConfig.value.enabled
  await saveTelegramConfig()
}

async function saveAndTest() {
  if (!telegramConfig.value.bot_token.trim()) {
    message.warning('请输入Bot Token')
    return
  }

  if (!telegramConfig.value.chat_id.trim()) {
    message.warning('请输入Chat ID')
    return
  }

  try {
    isTesting.value = true

    // 先保存配置
    await saveTelegramConfig()

    // 然后测试连接
    const result = await invoke('test_telegram_connection_cmd', {
      botToken: telegramConfig.value.bot_token,
      chatId: telegramConfig.value.chat_id,
    }) as string

    message.success(result)
  }
  catch (error) {
    console.error('测试Telegram连接失败:', error)
    message.error(typeof error === 'string' ? error : '测试连接失败')
  }
  finally {
    isTesting.value = false
  }
}

// 自动获取Chat ID
async function autoGetChatId() {
  if (!telegramConfig.value.bot_token.trim()) {
    message.warning('请先输入Bot Token')
    return
  }

  try {
    isDetectingChatId.value = true
    detectedChatInfo.value = null

    // 监听Chat ID检测事件
    // 使用静态导入的listen函数

    // 定义清理函数数组
    const cleanupFunctions: (() => void)[] = []

    const unlistenStart = await listen('chat-id-detection-started', () => {
      message.info('开始监听消息，请向Bot发送任意消息...')
    })
    cleanupFunctions.push(unlistenStart)

    const unlistenDetected = await listen('chat-id-detected', (event: any) => {
      detectedChatInfo.value = event.payload
      message.success(`检测到Chat ID: ${event.payload.chat_id}`)
      isDetectingChatId.value = false

      // 自动填入Chat ID
      telegramConfig.value.chat_id = event.payload.chat_id
      saveTelegramConfig()

      // 清理所有监听器
      cleanupFunctions.forEach(cleanup => cleanup())
    })
    cleanupFunctions.push(unlistenDetected)

    const unlistenTimeout = await listen('chat-id-detection-timeout', () => {
      message.warning('检测超时，请确保Bot Token正确并向Bot发送消息')
      isDetectingChatId.value = false

      // 清理所有监听器
      cleanupFunctions.forEach(cleanup => cleanup())
    })
    cleanupFunctions.push(unlistenTimeout)

    // 开始自动获取
    await invoke('auto_get_chat_id', { botToken: telegramConfig.value.bot_token })
  }
  catch (error) {
    console.error('自动获取Chat ID失败:', error)
    message.error('自动获取Chat ID失败')
    isDetectingChatId.value = false
  }
}

// 开启设置向导
function startSetupWizard() {
  showSetupWizard.value = true
  setupStep.value = 1
}

// 关闭设置向导
function closeSetupWizard() {
  showSetupWizard.value = false
  setupStep.value = 1
}

// 组件挂载时加载配置
onMounted(() => {
  loadTelegramConfig()
})
</script>

<template>
  <!-- 设置内容 -->
  <n-space vertical size="large">
    <!-- 启用Telegram Bot -->
    <div class="flex items-center justify-between">
      <div class="flex items-center">
        <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 flex-shrink-0" />
        <div>
          <div class="text-sm font-medium leading-relaxed">
            启用Telegram机器人
          </div>
          <div class="text-xs opacity-60">
            启用后可以通过Telegram Bot接收通知消息
          </div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <n-button
          v-if="!telegramConfig.enabled && (!telegramConfig.bot_token.trim() || !telegramConfig.chat_id.trim())"
          size="small" type="primary" @click="startSetupWizard"
        >
          一键设置
        </n-button>
        <n-switch :value="telegramConfig.enabled" size="small" @update:value="toggleTelegramEnabled" />
      </div>
    </div>

    <!-- 配置项区域 - 条件显示 -->
    <n-collapse-transition :show="telegramConfig.enabled">
      <n-space vertical size="large">
        <!-- Bot Token设置 -->
        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div class="flex items-start">
            <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 mt-2 flex-shrink-0" />
            <div class="flex-1">
              <div class="text-sm font-medium mb-3 leading-relaxed">
                Bot Token
              </div>
              <div class="text-xs opacity-60 mb-3">
                从 @BotFather 获取的Bot Token，用于验证Bot身份。不知道如何获取？点击下方"设置指引"查看完整教程
              </div>
              <n-space vertical size="small">
                <n-input
                  v-model:value="telegramConfig.bot_token" type="text"
                  placeholder="请输入Bot Token (例如: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz)" size="small"
                  :disabled="isTesting" @blur="saveTelegramConfig"
                />
                <n-button size="small" type="info" @click="startSetupWizard">
                  📋 设置指引
                </n-button>
              </n-space>
            </div>
          </div>
        </div>

        <!-- Chat ID设置 -->
        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div class="flex items-start">
            <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 mt-2 flex-shrink-0" />
            <div class="flex-1">
              <div class="text-sm font-medium mb-3 leading-relaxed">
                Chat ID
              </div>
              <div class="text-xs opacity-60 mb-3">
                目标聊天的ID，可以是个人聊天或群组聊天的ID。不知道如何获取？点击"详细指引"查看完整教程
              </div>
              <n-space vertical size="small">
                <n-input
                  v-model:value="telegramConfig.chat_id" type="text"
                  placeholder="请输入Chat ID (例如: 123456789 或 -123456789)" size="small"
                  :disabled="isTesting || isDetectingChatId" @blur="saveTelegramConfig"
                />
                <n-button
                  size="small" type="primary" :loading="isDetectingChatId"
                  :disabled="!telegramConfig.bot_token.trim() || isTesting" @click="autoGetChatId"
                >
                  {{ isDetectingChatId ? '监听中...' : '自动获取' }}
                </n-button>
                <div v-if="detectedChatInfo" class="text-xs text-success-600 dark:text-success-400">
                  ✅ 已检测到: {{ detectedChatInfo.chat_title }} ({{ detectedChatInfo.username }})
                </div>
              </n-space>
            </div>
          </div>
        </div>

        <!-- API服务器URL设置 -->
        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div class="flex items-start">
            <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 mt-2 flex-shrink-0" />
            <div class="flex-1">
              <div class="text-sm font-medium mb-3 leading-relaxed">
                API服务器URL
              </div>
              <div class="text-xs opacity-60 mb-3">
                API服务器地址，支持自定义代理
              </div>
              <n-space vertical size="small">
                <n-input
                  v-model:value="telegramConfig.api_base_url" type="text"
                  :placeholder="API_BASE_URL" size="small"
                  :disabled="isTesting" @blur="saveTelegramConfig"
                />
              </n-space>
              <div class="text-xs opacity-60 mt-2">
                💡 官方: {{ API_EXAMPLES.official }} | 代理: {{ API_EXAMPLES.proxy_example }}
              </div>
            </div>
          </div>
        </div>

        <!-- 隐藏前端弹窗设置 -->
        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div class="flex items-center justify-between">
            <div class="flex items-center">
              <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 flex-shrink-0" />
              <div>
                <div class="text-sm font-medium leading-relaxed">
                  隐藏前端弹窗
                </div>
                <div class="text-xs opacity-60">
                  启用后仅通过Telegram交互，不显示前端弹窗界面
                </div>
              </div>
            </div>
            <n-switch
              v-model:value="telegramConfig.hide_frontend_popup" size="small"
              @update:value="saveTelegramConfig"
            />
          </div>
        </div>

        <!-- 保存并测试按钮 -->
        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div class="flex items-start">
            <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 mt-2 flex-shrink-0" />
            <div class="flex-1">
              <div class="text-sm font-medium mb-3 leading-relaxed">
                连接测试
              </div>
              <div class="text-xs opacity-60 mb-3">
                保存配置并发送测试消息验证连接
              </div>
              <n-button
                type="primary" size="small" :loading="isTesting"
                :disabled="!telegramConfig.bot_token.trim() || !telegramConfig.chat_id.trim()" @click="saveAndTest"
              >
                {{ isTesting ? '测试中...' : '测试连接' }}
              </n-button>
            </div>
          </div>
        </div>
      </n-space>
    </n-collapse-transition>
  </n-space>

  <!-- 设置向导模态框 -->
  <n-modal v-model:show="showSetupWizard" preset="card" title="Telegram Bot 设置向导" style="width: 600px; margin: 0 20px;">
    <n-steps :current="setupStep" size="small">
      <n-step title="创建Bot" />
      <n-step title="获取Token" />
      <n-step title="获取Chat ID" />
      <n-step title="完成设置" />
    </n-steps>

    <div class="mt-6">
      <!-- 步骤1: 创建Bot -->
      <div v-if="setupStep === 1" class="space-y-4">
        <h3 class="text-lg font-medium">
          第一步：创建Telegram Bot
        </h3>
        <div class="space-y-3 text-sm">
          <p>1. 在Telegram中搜索并打开 <code class="bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-200 px-2 py-1 rounded font-medium">@BotFather</code></p>
          <p>2. 发送命令 <code class="bg-green-100 dark:bg-green-800 text-green-800 dark:text-green-200 px-2 py-1 rounded font-medium">/newbot</code></p>
          <p>3. 按提示输入Bot的名称和用户名</p>
          <p>4. 创建成功后，BotFather会发送Bot Token给你</p>
        </div>
        <n-space justify="end">
          <n-button @click="closeSetupWizard">
            取消
          </n-button>
          <n-button type="primary" @click="setupStep = 2">
            下一步
          </n-button>
        </n-space>
      </div>

      <!-- 步骤2: 获取Token -->
      <div v-if="setupStep === 2" class="space-y-4">
        <h3 class="text-lg font-medium">
          第二步：输入Bot Token
        </h3>
        <div class="space-y-3 text-sm">
          <p>将BotFather发送给你的Token粘贴到下面：</p>
          <n-input
            v-model:value="telegramConfig.bot_token" type="text"
            placeholder="例如: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz" size="small"
          />
        </div>
        <n-space justify="end">
          <n-button @click="setupStep = 1">
            上一步
          </n-button>
          <n-button type="primary" :disabled="!telegramConfig.bot_token.trim()" @click="setupStep = 3">
            下一步
          </n-button>
        </n-space>
      </div>

      <!-- 步骤3: 获取Chat ID -->
      <div v-if="setupStep === 3" class="space-y-4">
        <h3 class="text-lg font-medium">
          第三步：获取Chat ID
        </h3>
        <div class="space-y-2 text-sm">
          <n-card size="small">
            <h4 class="font-medium mb-2">
              方式一：自动获取（推荐）
            </h4>
            <n-button
              size="small" type="primary" :loading="isDetectingChatId"
              :disabled="!telegramConfig.bot_token.trim()" @click="autoGetChatId"
            >
              {{ isDetectingChatId ? '监听中，请发送消息...' : '开始自动获取' }}
            </n-button>
            <div v-if="detectedChatInfo" class="mt-2 text-sm text-success-600 dark:text-success-400">
              ✅ 检测成功: {{ detectedChatInfo.chat_id }}
            </div>
          </n-card>

          <n-card size="small">
            <h4 class="font-medium mb-2">
              方式二：手动获取
            </h4>
            <div class="text-sm space-y-2">
              <div class="p-2 rounded border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800">
                <code class="text-xs break-all text-gray-700 dark:text-gray-300">
                  {{ telegramConfig.api_base_url }}{{ telegramConfig.bot_token || 'YOUR_BOT_TOKEN' }}/getUpdates
                </code>
              </div>
            </div>
            <n-input
              v-model:value="telegramConfig.chat_id" type="text" placeholder="手动输入Chat ID" size="small"
              class="mt-2"
            />
          </n-card>
        </div>
        <n-space justify="end">
          <n-button @click="setupStep = 2">
            上一步
          </n-button>
          <n-button type="primary" :disabled="!telegramConfig.chat_id.trim()" @click="setupStep = 4">
            下一步
          </n-button>
        </n-space>
      </div>

      <!-- 步骤4: 完成设置 -->
      <div v-if="setupStep === 4" class="space-y-4">
        <h3 class="text-lg font-medium">
          第四步：完成设置
        </h3>
        <div class="space-y-2 text-sm">
          <div>
            <h4 class="font-medium mb-2">
              配置确认
            </h4>
            <n-card size="small" class="bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700">
              <div class="space-y-1 text-sm">
                <div class="text-gray-700 dark:text-gray-300">
                  <span class="font-medium">Bot Token:</span>
                  <code class="ml-2 text-gray-600 dark:text-gray-400">{{ telegramConfig.bot_token.substring(0, 20) }}...</code>
                </div>
                <div class="text-gray-700 dark:text-gray-300">
                  <span class="font-medium">Chat ID:</span>
                  <code class="ml-2 text-gray-600 dark:text-gray-400">{{ telegramConfig.chat_id }}</code>
                </div>
              </div>
            </n-card>
          </div>

          <div>
            <h4 class="font-medium mb-2">
              测试连接
            </h4>
            <n-button type="primary" size="small" :loading="isTesting" @click="saveAndTest">
              {{ isTesting ? '测试中...' : '测试连接' }}
            </n-button>
          </div>
        </div>
        <n-space justify="end">
          <n-button @click="setupStep = 3">
            上一步
          </n-button>
          <n-button type="primary" @click="closeSetupWizard">
            完成
          </n-button>
        </n-space>
      </div>
    </div>
  </n-modal>
</template>
