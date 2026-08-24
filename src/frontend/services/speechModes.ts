export type SpeechModeId = 'polish' | 'structure'

export interface SpeechMode {
  id: SpeechModeId
  name: string
  description: string
  shortcut: string
  ollamaModel: string
  enhancementTimeoutMs: number
  shortFastPath: boolean
  prompt: string
}

const COMMON_RULES = `通用规则：
1) 保留原意，不总结压缩，不扩写，不添加原文没有的信息。
2) 删除口头禅和明显重复，如"嗯/啊/那个/就是/这个/然后那个"。
3) 修正明显语音识别错误、常见同音误识别、专有名词和技术术语。
4) 中英文混排时保留英文术语，并在中英文之间加半角空格。
5) 直接输出最终可写回文本，不要解释、前缀、代码块或思考过程。`

const POLISH_PROMPT = `你是语音转写润色助手。把输入整理成自然、顺口、可直接发送的一段文字。
${COMMON_RULES}
短句保持短，不要改成列表。`

const STRUCTURE_PROMPT = `你是语音口述结构化助手。把输入整理成清晰的结构化内容。
${COMMON_RULES}
优先使用分组、短标题、项目符号或 1. 2. 3. 步骤；适合对比时可用表格。`

export const SPEECH_MODES: Record<SpeechModeId, SpeechMode> = {
  polish: {
    id: 'polish',
    name: '中文润色',
    description: '短句、聊天回复和日常输入，清理口头禅并补标点。',
    shortcut: 'Fn / Ctrl+1',
    ollamaModel: 'qwen2.5:7b',
    enhancementTimeoutMs: 60_000,
    shortFastPath: true,
    prompt: POLISH_PROMPT,
  },
  structure: {
    id: 'structure',
    name: '结构化整理',
    description: '长段口述整理为条目、步骤、分组或表格。',
    shortcut: 'Ctrl+2',
    ollamaModel: 'qwen2.5:7b',
    enhancementTimeoutMs: 60_000,
    shortFastPath: false,
    prompt: STRUCTURE_PROMPT,
  },
}

export const DEFAULT_SPEECH_MODE_ID: SpeechModeId = 'polish'

export function resolveSpeechMode(modeId?: string | null) {
  return SPEECH_MODES[(modeId || DEFAULT_SPEECH_MODE_ID) as SpeechModeId] || SPEECH_MODES[DEFAULT_SPEECH_MODE_ID]
}
