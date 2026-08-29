import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8')
}

test('canonical installation guide covers macOS, Windows, MCP startup, and verification', () => {
  const guide = read('docs/INSTALLATION.md')

  assert.ok(guide.length > 2_000, 'public installation guide must not be empty or a stub')
  assert.match(guide, /## macOS/)
  assert.match(guide, /## Windows/)
  assert.match(guide, /\/Applications\/iterate\.app\/Contents\/MacOS\/mcp-server/)
  assert.match(guide, /%LOCALAPPDATA%\\iterate\\bin\\mcp-server\.exe/)
  assert.match(guide, /%USERPROFILE%\\\.codeium\\windsurf\\mcp_config\.json/)
  assert.match(guide, /客户端自动启动 MCP|自动拉起 MCP/)
  assert.match(guide, /完全退出并重新打开|完全重启/)
  assert.match(guide, /最小验证/)
  assert.match(guide, /通用系统提示词/)
})

test('installation assistant prompt is cross-platform and resolves the MCP command per OS', () => {
  const source = read('docs/INSTALL_PROMPT.md')
  const setupModule = read('src/frontend/constants/setupPrompt.ts')

  assert.match(source, /macOS/)
  assert.match(source, /Windows/)
  assert.match(source, /\/Applications\/iterate\.app\/Contents\/MacOS\/mcp-server/)
  assert.match(source, /%LOCALAPPDATA%\\iterate\\bin\\mcp-server\.exe/)
  assert.match(source, /%USERPROFILE%\\\.codex\\config\.toml/)
  assert.match(source, /解析成当前用户的绝对路径/)
  assert.match(source, /完全退出并重新打开/)
  assert.match(source, /最小调用/)
  assert.match(setupModule, /docs\/INSTALL_PROMPT\.md\?raw/)
})

test('generic system prompt calls iterate with a non-empty handoff and has no private workflow dependency', () => {
  const prompt = read('docs/SYSTEM_PROMPT.md')
  const promptModule = read('src/frontend/constants/prompts.ts')

  assert.match(prompt, /前台主会话/)
  assert.match(prompt, /每次回复.*最后一步/)
  assert.match(prompt, /message/)
  assert.match(prompt, /predefined_options/)
  assert.match(prompt, /project_path/)
  assert.match(prompt, /继续对话.*true/)
  assert.match(prompt, /继续对话.*false/)
  assert.match(prompt, /输出文字.*zhi|只输出.*zhi/)
  assert.doesNotMatch(prompt, /\.cunzhi-knowledge|\.cunzhi-memory|Relearn|Codex execution policy/i)
  assert.match(promptModule, /docs\/SYSTEM_PROMPT\.md\?raw/)
  assert.doesNotMatch(promptModule, /\.cunzhi-knowledge|\.cunzhi-memory/)
})

test('README, in-app manual, and release packaging use the canonical cross-platform guide', () => {
  const readme = read('README.md')
  const promptsTab = read('src/frontend/components/tabs/PromptsTab.vue')
  const releaseWorkflow = read('.github/workflows/release.yml')
  const windowsWorkflow = read('.github/workflows/windows-package.yml')
  const macosDelivery = read('scripts/prepare-macos-delivery.sh')

  assert.match(readme, /macOS[\s\S]*Windows[\s\S]*docs\/INSTALLATION\.md/)
  assert.match(readme, /通用系统提示词/)
  assert.match(promptsTab, /macOS 安装/)
  assert.match(promptsTab, /Windows 安装/)
  assert.match(promptsTab, /通用系统提示词/)
  assert.match(promptsTab, /不依赖个人知识库/)

  for (const source of [releaseWorkflow, windowsWorkflow, macosDelivery]) {
    assert.match(source, /docs[\\/]+INSTALLATION\.md/)
    assert.doesNotMatch(source, /docs[\\/]+iterate_安装指南\.md/)
    assert.doesNotMatch(source, /docs[\\/]+release[\\/]+INSTALLATION\.md/)
  }
})
