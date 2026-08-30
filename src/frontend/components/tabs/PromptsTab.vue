<script setup lang="ts">
import { useMessage } from 'naive-ui'
import { computed, onMounted, ref } from 'vue'
import { useMcpToolsReactive } from '../../composables/useMcpTools'
import { generateFullPrompt } from '../../constants/prompts'
import { SETUP_PROMPT_CONTENT } from '../../constants/setupPrompt'
import { buildFormalRouteSetupPrompt } from '../settings/useMobileConnectionSetup'

const message = useMessage()

// 使用全局MCP工具状态
const { mcpTools, loading: mcpLoading, loadMcpTools, enabledTools } = useMcpToolsReactive()

// 根据MCP工具状态动态生成提示词
const promptContent = computed(() => {
  // 将后端数据格式转换为前端格式
  const frontendTools = mcpTools.value.map(tool => ({
    id: tool.id === 'ji' ? 'memory' : tool.id, // 后端用ji，前端用memory
    name: tool.name,
    description: tool.description,
    enabled: tool.enabled,
    canDisable: tool.can_disable,
    icon: tool.icon,
    iconBg: tool.icon_bg,
    darkIconBg: tool.dark_icon_bg,
  })).filter((tool) => {
    // 只包含有提示词配置的工具
    return tool.id === 'zhi' || tool.id === 'memory' || tool.id === 'sou'
  })

  return generateFullPrompt(frontendTools)
})

const copyButtonText = ref('复制')
const setupCopyText = ref('复制')
const iphoneQrCopyText = ref('复制配置提示词')
const showIphoneQrPrompt = ref(false)

const setupPromptContent = SETUP_PROMPT_CONTENT
const iphoneQrPromptContent = buildFormalRouteSetupPrompt()

async function copyIphoneQrPrompt() {
  try {
    await navigator.clipboard.writeText(iphoneQrPromptContent)
    iphoneQrCopyText.value = '已复制'
    message.success('配置提示词已复制')
  }
  catch (error) {
    iphoneQrCopyText.value = '复制失败'
    message.error('配置提示词复制失败，请重试')
    console.error('复制 iPhone 二维码配置提示词失败:', error)
  }
  finally {
    setTimeout(() => {
      iphoneQrCopyText.value = '复制配置提示词'
    }, 2000)
  }
}

// 复制安装提示词
async function copySetupPrompt() {
  try {
    await navigator.clipboard.writeText(setupPromptContent)
    setupCopyText.value = '已复制'
    setTimeout(() => {
      setupCopyText.value = '复制'
    }, 2000)
  }
  catch (error) {
    setupCopyText.value = '复制失败'
    setTimeout(() => {
      setupCopyText.value = '复制'
    }, 2000)
    console.error('复制失败:', error)
  }
}

// 复制参考提示词内容
async function copyPromptContent() {
  try {
    await navigator.clipboard.writeText(promptContent.value)
    copyButtonText.value = '已复制'
    setTimeout(() => {
      copyButtonText.value = '复制'
    }, 2000)
  }
  catch (error) {
    copyButtonText.value = '复制失败'
    setTimeout(() => {
      copyButtonText.value = '复制'
    }, 2000)
    console.error('复制失败:', error)
  }
}

// 组件挂载时加载MCP工具配置
onMounted(async () => {
  if (mcpTools.value.length === 0) {
    try {
      await loadMcpTools()
    }
    catch (error) {
      console.error('加载MCP工具配置失败:', error)
    }
  }
})
</script>

<template>
  <div class="max-w-3xl mx-auto tab-content">
    <n-space
      vertical
      size="medium"
    >
      <!-- 使用说明书卡片 -->
      <n-card size="small">
        <template #header>
          <n-space align="center">
            <div class="w-10 h-10 rounded-lg bg-green-100 dark:bg-green-900 flex items-center justify-center">
              <div class="i-carbon-information text-lg text-green-600 dark:text-green-400" />
            </div>
            <div>
              <div class="text-lg font-medium mb-1 tracking-tight">
                使用说明书
              </div>
              <div class="text-sm opacity-60 font-normal">
                先按系统安装，再接入 MCP，并复制通用系统提示词
              </div>
            </div>
          </n-space>
        </template>

        <n-space vertical size="small">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-2">
            <div class="rounded-lg border border-blue-200/70 dark:border-blue-800/70 bg-blue-50/70 dark:bg-blue-950/30 p-3">
              <div class="font-medium text-sm mb-1">
                macOS 安装
              </div>
              <div class="text-xs leading-5 opacity-75">
                将 iterate.app 放入“应用程序”，MCP command 使用
                /Applications/iterate.app/Contents/MacOS/mcp-server。
              </div>
            </div>
            <div class="rounded-lg border border-cyan-200/70 dark:border-cyan-800/70 bg-cyan-50/70 dark:bg-cyan-950/30 p-3">
              <div class="font-medium text-sm mb-1">
                Windows 安装
              </div>
              <div class="text-xs leading-5 opacity-75">
                解压后运行 Install iterate.bat，MCP command 位于
                %LOCALAPPDATA%\iterate\bin\mcp-server.exe。
              </div>
            </div>
          </div>
          <div class="flex items-center text-sm leading-relaxed">
            <div class="w-1.5 h-1.5 bg-blue-500 rounded-full mr-3 flex-shrink-0" />
            <span class="opacity-90">首次安装或接入新的 AI 客户端，复制“iterate 安装”发给 AI</span>
          </div>
          <div class="flex items-center text-sm leading-relaxed">
            <div class="w-1.5 h-1.5 bg-purple-500 rounded-full mr-3 flex-shrink-0" />
            <span class="opacity-90">顶部二维码尚不可用，使用“iPhone 连接配置”让 AI 配通正式连接</span>
          </div>
          <div class="flex items-center text-sm leading-relaxed">
            <div class="w-1.5 h-1.5 bg-orange-500 rounded-full mr-3 flex-shrink-0" />
            <span class="opacity-90">日常使用 iterate，把“通用系统提示词”加入 AI 的系统提示中</span>
          </div>
          <div class="flex items-center text-sm leading-relaxed">
            <div class="w-1.5 h-1.5 bg-green-500 rounded-full mr-3 flex-shrink-0" />
            <span class="opacity-90">通用模板不依赖个人知识库，并会跟随“MCP 工具”页面的开关更新</span>
          </div>
        </n-space>
      </n-card>

      <!-- iterate 安装卡片 -->
      <n-card size="small">
        <template #header>
          <n-space
            align="center"
            justify="space-between"
          >
            <n-space align="center">
              <div class="w-10 h-10 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center">
                <div class="i-carbon-plug text-lg text-blue-600 dark:text-blue-400" />
              </div>
              <div>
                <div class="text-lg font-medium mb-1 tracking-tight">
                  iterate 安装
                </div>
                <div class="text-sm opacity-60 font-normal">
                  复制后发给 AI，由 AI 继续完成安装、客户端接入和验证
                </div>
              </div>
            </n-space>
            <n-button
              type="primary"
              size="small"
              @click="copySetupPrompt"
            >
              <template #icon>
                <div class="i-carbon-copy text-sm" />
              </template>
              {{ setupCopyText }}
            </n-button>
          </n-space>
        </template>

        <n-card embedded>
          <div class="text-sm font-mono leading-relaxed">
            <pre class="whitespace-pre-wrap my-0 opacity-90">{{ setupPromptContent }}</pre>
          </div>
        </n-card>
      </n-card>

      <!-- iPhone 连接配置卡片 -->
      <n-card size="small">
        <template #header>
          <n-space
            align="center"
            justify="space-between"
          >
            <n-space align="center">
              <div class="w-10 h-10 rounded-lg bg-purple-100 dark:bg-purple-900 flex items-center justify-center">
                <div class="i-carbon-qr-code text-lg text-purple-600 dark:text-purple-400" />
              </div>
              <div>
                <div class="text-lg font-medium mb-1 tracking-tight">
                  iPhone 连接配置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  复制给 AI，配置正式连接并启用顶部二维码
                </div>
              </div>
            </n-space>

            <n-space align="center" size="small">
              <n-button
                quaternary
                size="small"
                @click="showIphoneQrPrompt = !showIphoneQrPrompt"
              >
                {{ showIphoneQrPrompt ? '收起完整提示词' : '查看完整提示词' }}
              </n-button>
              <n-button
                type="primary"
                size="small"
                @click="copyIphoneQrPrompt"
              >
                <template #icon>
                  <div class="i-carbon-copy text-sm" />
                </template>
                {{ iphoneQrCopyText }}
              </n-button>
            </n-space>
          </n-space>
        </template>

        <div class="rounded-lg border border-purple-200/70 dark:border-purple-700/70 bg-purple-50/70 dark:bg-purple-950/30 p-4">
          <div class="text-sm leading-6 opacity-90">
            把配置提示词发送给当前电脑上的 AI。AI 会先识别 macOS、Windows 或 Linux，并检查已有连接；首次使用时，在你确认后配置正式公网；已有连接出现故障时，只修复原来的连接。完成后，回到 iterate 点击顶部二维码。
          </div>
          <div class="mt-3 flex items-start gap-2 text-xs leading-5 text-purple-700 dark:text-purple-300">
            <div class="i-carbon-locked mt-0.5 flex-shrink-0" />
            <span>账号登录、域名、DNS 和管理员权限始终由你确认。AI 不会索取或展示凭据。</span>
          </div>
        </div>

        <n-card
          v-if="showIphoneQrPrompt"
          embedded
          class="mt-4"
        >
          <div class="text-sm font-mono leading-relaxed">
            <pre class="whitespace-pre-wrap my-0 opacity-90">{{ iphoneQrPromptContent }}</pre>
          </div>
        </n-card>
      </n-card>

      <!-- 通用系统提示词卡片 -->
      <n-card size="small">
        <!-- 卡片头部 -->
        <template #header>
          <n-space
            align="center"
            justify="space-between"
          >
            <n-space align="center">
              <!-- 图标 -->
              <div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900 flex items-center justify-center">
                <div class="i-carbon-document text-lg text-orange-600 dark:text-orange-400" />
              </div>

              <!-- 标题信息 -->
              <div>
                <div class="text-lg font-medium mb-1 tracking-tight">
                  通用系统提示词
                </div>
                <div class="text-sm opacity-60 font-normal">
                  适用于 macOS / Windows，不依赖个人知识库
                </div>
              </div>
            </n-space>

            <!-- 复制按钮 -->
            <n-button
              type="primary"
              size="small"
              @click="copyPromptContent"
            >
              <template #icon>
                <div class="i-carbon-copy text-sm" />
              </template>
              {{ copyButtonText }}
            </n-button>
          </n-space>
        </template>

        <!-- 工具状态说明 -->
        <div class="flex items-center text-sm leading-relaxed mb-4">
          <div
            class="w-1.5 h-1.5 rounded-full mr-3 flex-shrink-0"
            :class="mcpLoading ? 'bg-yellow-500' : 'bg-green-500'"
          />
          <span class="opacity-90">
            <template v-if="mcpLoading">
              正在加载 MCP 工具配置...
            </template>
            <template v-else>
              当前已启用 {{ enabledTools.length }} / {{ mcpTools.length }} 个 MCP 工具，
              可在“MCP 工具”页面管理工具开关
            </template>
          </span>
        </div>

        <!-- 启用工具列表 -->
        <div class="mb-4">
          <div class="text-sm font-medium mb-2 opacity-80">
            已启用的工具模块：
          </div>
          <n-space v-if="!mcpLoading && enabledTools.length > 0">
            <n-tag
              v-for="tool in enabledTools"
              :key="tool.id"
              size="small"
              :bordered="false"
            >
              <template #icon>
                <div :class="tool.icon" />
              </template>
              {{ tool.name }}
            </n-tag>
          </n-space>
          <div
            v-else-if="!mcpLoading && enabledTools.length === 0"
            class="text-sm opacity-60"
          >
            暂无启用的工具
          </div>
          <n-skeleton
            v-else
            text
            :repeat="2"
          />
        </div>

        <!-- 内容区域 -->
        <n-card embedded>
          <div class="text-sm font-mono leading-relaxed">
            <pre class="whitespace-pre-wrap my-0 opacity-90">{{ promptContent }}</pre>
          </div>
        </n-card>
      </n-card>
    </n-space>
  </div>
</template>
