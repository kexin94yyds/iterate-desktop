import type { SpeechMode } from './speechModes'

const OLLAMA_BASE_URL = 'http://localhost:11434'

interface OllamaChatResponse {
  message?: {
    content?: string
  }
  error?: string
  total_duration?: number
  load_duration?: number
  prompt_eval_count?: number
  prompt_eval_duration?: number
  eval_count?: number
  eval_duration?: number
}

export interface DictionaryEntryLike {
  word?: string
  replacement?: string
  spokenPhrase?: string
  outputText?: string
  trainingCount?: number
  isEnabled?: boolean
  source?: string
}

export interface EnhanceTranscriptInput {
  text: string
  mode: SpeechMode
  dictionaryEntries?: DictionaryEntryLike[]
}

export interface OllamaDurationMetrics {
  totalDurationMs?: number
  loadDurationMs?: number
  promptEvalCount?: number
  promptEvalDurationMs?: number
  evalCount?: number
  evalDurationMs?: number
}

export interface EnhanceTranscriptResult {
  text: string
  metrics?: OllamaDurationMetrics
}

function nanosToMs(value?: number) {
  return typeof value === 'number' ? Math.round(value / 1_000_000) : undefined
}

function buildDictionaryBlock(entries: DictionaryEntryLike[] = []) {
  const lines = entries
    .map((entry) => {
      const source = (entry.word || entry.spokenPhrase || '').trim()
      const target = (entry.replacement || entry.outputText || source).trim()
      if (!source)
        return ''
      return source === target ? `- ${source}` : `- ${source} => ${target}`
    })
    .filter(Boolean)
    .slice(0, 80)

  if (!lines.length) {
    return ''
  }

  return [
    '',
    '个人词典：以下术语和替换规则优先保留或修正。',
    ...lines,
  ].join('\n')
}

function buildPrompt(input: EnhanceTranscriptInput) {
  return [
    input.mode.prompt,
    buildDictionaryBlock(input.dictionaryEntries),
    '',
    '待处理语音转写文本：',
    input.text,
    '',
    '只输出最终可写回文本，不要输出解释、前缀、代码块或思考过程。',
  ].join('\n')
}

function stripModelNoise(text: string) {
  let cleaned = text.replace(/<think>[\s\S]*?<\/think>/gi, '').trim()
  cleaned = cleaned.replace(/^```\w*\s*/u, '').replace(/\s*```$/u, '').trim()
  cleaned = cleaned.replace(/^(?:输出|结果|整理后|润色后|最终文本)[:：]\s*/u, '').trim()
  return cleaned
}

function resolveNumPredict(text: string) {
  const length = text.trim().length
  if (length < 160)
    return 384
  if (length < 700)
    return 1024
  if (length < 1600)
    return 2048
  return 3072
}

export async function enhanceTranscript(input: EnhanceTranscriptInput): Promise<EnhanceTranscriptResult> {
  const trimmed = input.text.trim()
  if (!trimmed) {
    return { text: '' }
  }

  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(), input.mode.enhancementTimeoutMs)

  try {
    const response = await fetch(`${OLLAMA_BASE_URL}/api/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: input.mode.ollamaModel,
        messages: [
          {
            role: 'user',
            content: buildPrompt(input),
          },
        ],
        stream: false,
        think: false,
        keep_alive: '10m',
        options: {
          temperature: 0.15,
          num_predict: resolveNumPredict(trimmed),
        },
      }),
      signal: controller.signal,
    })

    if (!response.ok) {
      throw new Error(`Ollama returned HTTP ${response.status}`)
    }

    const payload = await response.json() as OllamaChatResponse
    if (payload.error) {
      throw new Error(payload.error)
    }

    const enhanced = stripModelNoise(payload.message?.content || '')
    if (!enhanced) {
      throw new Error('Ollama returned empty text')
    }

    return {
      text: enhanced,
      metrics: {
        totalDurationMs: nanosToMs(payload.total_duration),
        loadDurationMs: nanosToMs(payload.load_duration),
        promptEvalCount: payload.prompt_eval_count,
        promptEvalDurationMs: nanosToMs(payload.prompt_eval_duration),
        evalCount: payload.eval_count,
        evalDurationMs: nanosToMs(payload.eval_duration),
      },
    }
  }
  catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new Error(`Ollama enhancement timed out after ${Math.round(input.mode.enhancementTimeoutMs / 1000)}s`)
    }
    throw error
  }
  finally {
    window.clearTimeout(timeout)
  }
}
