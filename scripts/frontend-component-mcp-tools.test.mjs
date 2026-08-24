import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '..')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

test('component test fixture mocks MCP tools as an array of tool configs', () => {
  const source = read('src/frontend/test/main.ts')

  assert.match(source, /command === 'get_mcp_tools_config'/)
  assert.match(source, /id:\s*'ji'/)
  assert.match(source, /name:\s*'记忆管理'/)
  assert.match(source, /can_disable:\s*true/)
  assert.match(source, /icon_bg:\s*'bg-/)
})

test('component test fixture registers Naive UI components used by McpToolsTab', () => {
  const source = read('src/frontend/test/main.ts')

  for (const component of [
    'NCollapse',
    'NCollapseItem',
    'NEmpty',
    'NFormItem',
    'NModal',
    'NSpin',
  ]) {
    assert.match(source, new RegExp(`\\b${component}\\b`), `${component} is not registered`)
  }
})

test('PromptsTab reuses the verified iPhone route prompt in a focused copy card', () => {
  const source = read('src/frontend/components/tabs/PromptsTab.vue')
  const layoutSource = read('src/frontend/components/layout/MainLayout.vue')
  const mobilePromptSource = read('src/frontend/components/settings/useMobileConnectionSetup.ts')
  const mobileWizardSource = read('src/frontend/components/settings/MobileConnectionWizard.vue')

  assert.match(layoutSource, /<n-tab-pane name="prompts" tab="使用说明书">/)
  assert.doesNotMatch(layoutSource, /tab="参考提示词"/)
  assert.match(source, /import \{ buildFormalRouteSetupPrompt \} from '\.\.\/settings\/useMobileConnectionSetup'/)
  assert.match(source, /const iphoneQrPromptContent = buildFormalRouteSetupPrompt\(\)/)
  assert.match(source, /iPhone 连接配置/)
  assert.match(source, /配置正式连接并启用顶部二维码/)
  assert.match(source, /当前电脑上的 AI/)
  assert.match(source, /复制配置提示词/)
  assert.match(source, /查看完整提示词/)
  assert.match(source, /await navigator\.clipboard\.writeText\(iphoneQrPromptContent\)/)
  assert.match(source, /message\.success\('配置提示词已复制'\)/)
  assert.match(source, /iterate 安装/)
  assert.match(source, /核心系统提示词/)
  assert.match(mobilePromptSource, /当前电脑上的 iterate/)
  assert.match(mobilePromptSource, /macOS、Windows 还是 Linux/)
  assert.match(mobilePromptSource, /%USERPROFILE%\\\\\.cloudflared/)
  assert.match(mobilePromptSource, /Windows Service/)
  assert.doesNotMatch(mobilePromptSource, /这台 Mac/)
  assert.doesNotMatch(mobileWizardSource, /这台 Mac/)

  assert.match(source, /使用说明书/)

  const usageCard = source.indexOf('<!-- 使用说明书卡片 -->')
  const installCard = source.indexOf('<!-- iterate 安装卡片 -->')
  const iphoneCard = source.indexOf('<!-- iPhone 连接配置卡片 -->')
  const systemCard = source.indexOf('<!-- 核心系统提示词卡片 -->')
  assert.ok(
    usageCard < installCard && installCard < iphoneCard && iphoneCard < systemCard,
    'cards must follow usage, install, iPhone setup, then system prompt',
  )
})
