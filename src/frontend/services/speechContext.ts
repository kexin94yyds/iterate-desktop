export interface SpeechMemoryEntry {
  id?: string
  spokenPhrase?: string
  outputText?: string
  trainingCount?: number
  isEnabled?: boolean
}

export interface SpeechCorrectionMemoryEntry {
  id?: string
  observedText?: string
  intendedText?: string
  contextTerms?: string[]
  hitCount?: number
  confirmCount?: number
  rejectCount?: number
  isEnabled?: boolean
  updatedAt?: string
}

export interface SpeechVocabularyEntry {
  term: string
  count: number
  first_seen_at: string
  last_seen_at: string
}

export interface SpeechContextInput {
  requestMessage?: string
  userInput?: string
  muscleMemoryEntries?: SpeechMemoryEntry[]
  correctionMemoryEntries?: SpeechCorrectionMemoryEntry[]
  rememberedTerms?: string[]
  shortcutTerms?: string[]
  limit?: number
}

export const DEFAULT_CONTEXTUAL_STRINGS_LIMIT = 60
const MAX_CONTEXTUAL_STRING_LENGTH = 48

const COMMAND_TERMS = [
  'zhi',
  'call_zhi',
  'hui',
  'hui0',
  'hui1',
  'xi',
  'pai',
  'sou',
  'cha',
  'yan',
  'qiu',
  'ji',
  'plan',
  'debug',
  'computeruse',
  'sync',
]

const WORKFLOW_TERMS = [
  '派',
  '回',
  '继续',
  '确认',
  '计划',
  '调研',
  '同步',
  '审查',
]

const DOMAIN_TERMS = [
  'MCP',
  'Codex',
  'CSS',
  'style',
  'Swift',
  'Rust',
  'Tauri',
]

const EXPERIMENTAL_TERMS = [
  'WhisperKit',
  'SpeechAnalyzer',
  '肌肉记忆',
]

export function normalizeSpeechText(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '')
}

export function extractSpeechTerms(text: string, limit = 40) {
  if (!text)
    return []

  const terms: string[] = []
  const pattern = /[a-z][\w./-]+|\p{Script=Han}{2,}/giu
  for (const match of text.matchAll(pattern)) {
    terms.push(match[0])
    if (terms.length >= limit)
      break
  }

  for (const shortTerm of ['回', '派']) {
    if (text.includes(shortTerm))
      terms.push(shortTerm)
  }

  return terms
}

export function shouldRememberSpeechVocabularyTerm(rawTerm: string) {
  const term = rawTerm.trim()
  const characters = Array.from(term)
  if (!term || characters.length > MAX_CONTEXTUAL_STRING_LENGTH)
    return false
  if (characters.length < 2 && term !== '回' && term !== '派')
    return false
  if (/[\n\r/\\@?=]/.test(term))
    return false
  if (!/^[\p{L}\p{N}._:-]+$/u.test(term))
    return false

  const lower = term.toLowerCase()
  if (/token|secret|password|passwd|apikey|api_key|private|credential|auth/i.test(lower))
    return false
  const digitCount = characters.filter(character => /\d/.test(character)).length
  if (/^\d+$/.test(term) || digitCount / characters.length > 0.6)
    return false
  if (characters.length >= 12 && /^[0-9a-f]+$/i.test(term))
    return false

  return true
}

export function extractSafeSpeechVocabularyTerms(text: string, limit = 40) {
  const seen = new Set<string>()
  return extractSpeechTerms(text, limit * 2)
    .filter(shouldRememberSpeechVocabularyTerm)
    .filter((term) => {
      const normalized = term.trim().toLowerCase()
      if (seen.has(normalized))
        return false
      seen.add(normalized)
      return true
    })
    .slice(0, limit)
}

export function resolveMemorySource(entry: SpeechMemoryEntry) {
  return String(entry.spokenPhrase || '').trim()
}

export function resolveMemoryOutput(entry: SpeechMemoryEntry) {
  return String(entry.outputText || resolveMemorySource(entry)).trim()
}

function contextualTermsFromMuscleMemory(entries: SpeechMemoryEntry[]) {
  return entries
    .filter(entry => entry.isEnabled !== false)
    .sort((left, right) => Number(right.trainingCount || 0) - Number(left.trainingCount || 0))
    .flatMap(entry => [resolveMemoryOutput(entry), resolveMemorySource(entry)])
}

function contextualTermsFromCorrectionMemory(entries: SpeechCorrectionMemoryEntry[]) {
  return entries
    .filter(entry => entry.isEnabled !== false)
    .sort((left, right) => {
      const leftScore = Number(left.confirmCount || 0) + Number(left.hitCount || 0)
      const rightScore = Number(right.confirmCount || 0) + Number(right.hitCount || 0)
      if (leftScore !== rightScore)
        return rightScore - leftScore
      return String(right.updatedAt || '').localeCompare(String(left.updatedAt || ''))
    })
    .flatMap(entry => [
      String(entry.intendedText || '').trim(),
      ...(Array.isArray(entry.contextTerms) ? entry.contextTerms : []),
    ])
}

function addRankedTerms(terms: string[], hints: string[], seen: Set<string>, limit: number) {
  for (const term of terms) {
    const trimmed = String(term || '').trim()
    if (!trimmed || trimmed.length > MAX_CONTEXTUAL_STRING_LENGTH || trimmed.includes('\n'))
      continue

    const normalized = normalizeSpeechText(trimmed)
    if (!normalized || seen.has(normalized))
      continue

    seen.add(normalized)
    hints.push(trimmed)
    if (hints.length >= limit)
      break
  }
}

export function buildSpeechContextualStrings(input: SpeechContextInput = {}) {
  const limit = input.limit || DEFAULT_CONTEXTUAL_STRINGS_LIMIT
  const hints: string[] = []
  const seen = new Set<string>()
  const requestTerms = extractSpeechTerms(`${input.requestMessage || ''} ${input.userInput || ''}`)
  const correctionTerms = contextualTermsFromCorrectionMemory(input.correctionMemoryEntries || [])
  const muscleMemoryTerms = contextualTermsFromMuscleMemory(input.muscleMemoryEntries || [])

  addRankedTerms(correctionTerms, hints, seen, limit)
  addRankedTerms(requestTerms, hints, seen, limit)
  addRankedTerms(input.rememberedTerms || [], hints, seen, limit)
  addRankedTerms(DOMAIN_TERMS.concat(WORKFLOW_TERMS, muscleMemoryTerms), hints, seen, limit)
  addRankedTerms((input.shortcutTerms || []).concat(COMMAND_TERMS), hints, seen, limit)
  addRankedTerms(EXPERIMENTAL_TERMS, hints, seen, limit)

  return hints
}
