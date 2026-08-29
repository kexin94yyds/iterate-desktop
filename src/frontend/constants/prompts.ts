import genericSystemPrompt from '../../../docs/SYSTEM_PROMPT.md?raw'

export interface PromptSection {
  base: string
  detail: string
}

export interface MCPToolConfig {
  id: string
  name: string
  description: string
  enabled: boolean
  canDisable: boolean
  icon: string
  iconBg: string
  darkIconBg: string
}

// 通用核心规范：不绑定个人知识库、私有目录或特定客户端。
export const CORE_RULES = genericSystemPrompt.trim()

export const PROMPT_SECTIONS = {
  zhi: {
    base: ``,
    detail: `## iterate 工具补充说明
- 需要用户决策、阶段确认或高风险授权时，通过 \`zhi\` / \`call_zhi\` 交还控制权。
- 每次调用都必须发送非空 \`message\`；已知工作区时附上准确的 \`project_path\`。
- 不得用普通文字、后台任务或第二个重复调用替代当前前台交互。`,
  } as PromptSection,

  memory: {
    base: ``,
    detail: `## 可选记忆工具
- 只有在记忆工具已启用且用户明确需要保存或恢复信息时，才按该工具的公开 schema 调用。
- 不假设用户拥有任何私有知识库、固定目录或专属同步流程。
- 写入前说明保存范围；不要记录密钥、令牌或与当前任务无关的隐私内容。`,
  } as PromptSection,

  sou: {
    base: ``,
    detail: `## 可选代码搜索工具
- 如果需要查找代码，并且当前客户端提供 \`sou\`，优先用它获取与当前任务相关的代码上下文。`,
  } as PromptSection,
}

export const DEFAULT_MCP_TOOLS: MCPToolConfig[] = [
  {
    id: 'zhi',
    name: 'Zhi 智能审查工具',
    description: '智能代码审查交互工具（iterate）',
    enabled: true,
    canDisable: false,
    icon: 'i-carbon-chat text-lg text-blue-600 dark:text-blue-400',
    iconBg: 'bg-blue-100',
    darkIconBg: 'dark:bg-blue-900',
  },
  {
    id: 'memory',
    name: '记忆管理工具',
    description: '可选的记忆存储和检索工具',
    enabled: true,
    canDisable: true,
    icon: 'i-carbon-data-base text-lg text-purple-600 dark:text-purple-400',
    iconBg: 'bg-purple-100',
    darkIconBg: 'dark:bg-purple-900',
  },
  {
    id: 'sou',
    name: '代码搜索工具',
    description: '在当前项目中搜索相关代码上下文',
    enabled: false,
    canDisable: true,
    icon: 'i-carbon-search text-lg text-green-600 dark:text-green-400',
    iconBg: 'bg-green-100',
    darkIconBg: 'dark:bg-green-900',
  },
]

export function generateFullPrompt(mcpTools: MCPToolConfig[]): string {
  const enabledTools = mcpTools.filter(tool => tool.enabled)
  const parts: string[] = [CORE_RULES]

  const baseParts = enabledTools
    .map(tool => PROMPT_SECTIONS[tool.id as keyof typeof PROMPT_SECTIONS]?.base)
    .filter(Boolean)

  if (baseParts.length > 0)
    parts[0] = `${parts[0]}\n${baseParts.join('\n')}`

  const detailParts = enabledTools
    .map(tool => PROMPT_SECTIONS[tool.id as keyof typeof PROMPT_SECTIONS]?.detail)
    .filter(Boolean)

  if (detailParts.length > 0)
    parts.push(...detailParts)

  return parts.join('\n\n')
}

export const REFERENCE_PROMPT = generateFullPrompt(DEFAULT_MCP_TOOLS)
