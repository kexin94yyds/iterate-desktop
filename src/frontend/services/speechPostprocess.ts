import type { SpeechCorrectionMemoryEntry, SpeechMemoryEntry } from './speechContext.ts'
import { extractSpeechTerms, normalizeSpeechText, resolveMemoryOutput, resolveMemorySource } from './speechContext.ts'
import { refineSpeechSelfCorrectionText, refineSpeechSemanticText } from './speechSemanticResolver.ts'

const TRUSTED_CORRECTION_CONFIRM_THRESHOLD = 3

export interface SpeechPostprocessInput {
  text: string
  muscleMemoryEntries?: SpeechMemoryEntry[]
  correctionMemoryEntries?: SpeechCorrectionMemoryEntry[]
  contextTerms?: string[]
}

export interface SpeechPostprocessResult {
  text: string
  status: 'written' | 'memory-written' | 'correction-memory-written' | 'self-correction-written' | 'semantic-written'
  correctionEntry?: SpeechCorrectionMemoryEntry
  muscleEntry?: SpeechMemoryEntry
}

function isActiveMuscleMemoryEntry(entry: SpeechMemoryEntry) {
  return entry.isEnabled !== false && Number(entry.trainingCount || 0) >= 4
}

function applyDeterministicMuscleMemory(rawText: string, entries: SpeechMemoryEntry[]) {
  const normalizedRaw = normalizeSpeechText(rawText)
  if (!normalizedRaw)
    return null

  const candidates = entries
    .filter(isActiveMuscleMemoryEntry)
    .map((entry) => {
      const source = resolveMemorySource(entry)
      const output = resolveMemoryOutput(entry)
      return {
        entry,
        source,
        output,
        normalizedSource: normalizeSpeechText(source),
        trainingCount: Number(entry.trainingCount || 0),
      }
    })
    .filter(candidate =>
      candidate.normalizedSource
      && candidate.output
      && normalizeSpeechText(candidate.output) !== candidate.normalizedSource,
    )
    .sort((a, b) => {
      if (a.normalizedSource.length !== b.normalizedSource.length)
        return b.normalizedSource.length - a.normalizedSource.length
      return b.trainingCount - a.trainingCount
    })

  const exact = candidates.find(candidate => candidate.normalizedSource === normalizedRaw)
  return exact || null
}

function indexOfCaseInsensitive(haystack: string, needle: string) {
  return haystack.toLocaleLowerCase().indexOf(needle.toLocaleLowerCase())
}

function isAsciiWord(value: string) {
  return /^\w+$/.test(value)
}

function isAsciiWordChar(value: string) {
  return /^\w$/.test(value)
}

function observedRange(observedText: string, transcript: string) {
  const observed = observedText.trim()
  if (!observed)
    return null

  if (isAsciiWord(observed)) {
    const lowerTranscript = transcript.toLocaleLowerCase()
    const lowerObserved = observed.toLocaleLowerCase()
    let index = lowerTranscript.indexOf(lowerObserved)
    while (index >= 0) {
      const before = index > 0 ? transcript[index - 1] : ''
      const afterIndex = index + observed.length
      const after = afterIndex < transcript.length ? transcript[afterIndex] : ''
      if ((!before || !isAsciiWordChar(before)) && (!after || !isAsciiWordChar(after))) {
        return { start: index, end: afterIndex }
      }
      index = lowerTranscript.indexOf(lowerObserved, index + observed.length)
    }
  }

  const directIndex = indexOfCaseInsensitive(transcript, observed)
  if (directIndex >= 0)
    return { start: directIndex, end: directIndex + observed.length }

  if (normalizeSpeechText(transcript) === normalizeSpeechText(observed))
    return { start: 0, end: transcript.length }

  return null
}

function correctedTranscript(entry: SpeechCorrectionMemoryEntry, transcript: string) {
  const intended = String(entry.intendedText || '').trim()
  if (!intended || normalizeSpeechText(String(entry.observedText || '')) === normalizeSpeechText(intended))
    return null

  const range = observedRange(String(entry.observedText || ''), transcript)
  if (!range)
    return null

  return `${transcript.slice(0, range.start)}${intended}${transcript.slice(range.end)}`
}

function isExplicitComparisonContext(transcript: string, entry: SpeechCorrectionMemoryEntry) {
  const normalizedTranscript = normalizeSpeechText(transcript)
  const observed = normalizeSpeechText(String(entry.observedText || ''))
  const intended = normalizeSpeechText(String(entry.intendedText || ''))
  if (!observed || !intended || !normalizedTranscript.includes(observed) || !normalizedTranscript.includes(intended))
    return false

  return ['识别成', '不要把', '不是', '而是', '改成', '纠错', '替换', '->', '=>']
    .some(marker => transcript.toLocaleLowerCase().includes(marker.toLocaleLowerCase()))
}

function requiresStrongStyleContext(observed: string, intended: string) {
  return intended === 'style' && ['sell', 'sale', 'cell', 'ceo'].includes(observed)
}

function hasStrongStyleContext(transcript: string, normalizedContextTerms: Set<string>) {
  const exactTranscriptTerms = new Set(
    extractSpeechTerms(transcript).map(normalizeSpeechText).filter(Boolean),
  )
  if (exactTranscriptTerms.has('style') || exactTranscriptTerms.has('ui'))
    return true

  const normalizedTranscript = normalizeSpeechText(transcript)
  return ['css', 'styles', 'stylesheet', 'frontend', '样式', '前端', '界面', '组件', '设计']
    .some(term => normalizedContextTerms.has(normalizeSpeechText(term)) || normalizedTranscript.includes(normalizeSpeechText(term)))
}

function isBlockedByNegativeContext(entry: SpeechCorrectionMemoryEntry, transcript: string, normalizedContextTerms: Set<string>) {
  const observed = normalizeSpeechText(String(entry.observedText || ''))
  const intended = normalizeSpeechText(String(entry.intendedText || ''))
  const isStyleCorrectionPair = intended === 'style' && ['sell', 'sale', 'cell', 'ceo'].includes(observed)
  if (!isStyleCorrectionPair)
    return false

  const normalizedTranscript = normalizeSpeechText(transcript)
  return ['销售', '客户', '报价', '成交', '漏斗', '转化', '业务', 'sales', 'selling', 'customer', 'business', 'funnel', 'revenue', 'pricing']
    .some(term => normalizedContextTerms.has(normalizeSpeechText(term)) || normalizedTranscript.includes(normalizeSpeechText(term)))
}

function isContextAllowed(entry: SpeechCorrectionMemoryEntry, transcript: string, normalizedContextTerms: Set<string>) {
  const observed = normalizeSpeechText(String(entry.observedText || ''))
  const intended = normalizeSpeechText(String(entry.intendedText || ''))
  if (!intended)
    return false
  if (isBlockedByNegativeContext(entry, transcript, normalizedContextTerms))
    return false
  if (requiresStrongStyleContext(observed, intended))
    return hasStrongStyleContext(transcript, normalizedContextTerms)
  if (normalizedContextTerms.has(intended))
    return true

  const entryTerms = [entry.intendedText, ...(entry.contextTerms || [])]
    .map(value => normalizeSpeechText(String(value || '')))
    .filter(Boolean)

  return entryTerms.some(term =>
    normalizedContextTerms.has(term)
    || Array.from(normalizedContextTerms).some(context =>
      context.length >= 2 && term.length >= 2 && (context.includes(term) || term.includes(context)),
    ),
  )
}

function applyCorrectionMemory(rawText: string, entries: SpeechCorrectionMemoryEntry[], contextTerms: string[]) {
  const normalizedRaw = normalizeSpeechText(rawText)
  if (!normalizedRaw)
    return null

  const normalizedContextTerms = new Set(
    contextTerms
      .concat(extractSpeechTerms(rawText))
      .map(normalizeSpeechText)
      .filter(Boolean),
  )

  const matches = entries
    .filter(entry => entry.isEnabled !== false)
    .map((entry) => {
      const correctedText = correctedTranscript(entry, rawText)
      return { entry, correctedText }
    })
    .filter((match): match is { entry: SpeechCorrectionMemoryEntry, correctedText: string } =>
      Boolean(
        match.correctedText
        && Number(match.entry.confirmCount || 0) >= TRUSTED_CORRECTION_CONFIRM_THRESHOLD
        && Number(match.entry.rejectCount || 0) === 0
        && normalizeSpeechText(String(match.entry.intendedText || '')) !== normalizedRaw
        && match.correctedText !== rawText
        && !isExplicitComparisonContext(rawText, match.entry)
        && isContextAllowed(match.entry, rawText, normalizedContextTerms),
      ),
    )
    .sort((left, right) => {
      const leftScore = Number(left.entry.confirmCount || 0) * 3 + Number(left.entry.hitCount || 0) - Number(left.entry.rejectCount || 0) * 4
      const rightScore = Number(right.entry.confirmCount || 0) * 3 + Number(right.entry.hitCount || 0) - Number(right.entry.rejectCount || 0) * 4
      if (leftScore !== rightScore)
        return rightScore - leftScore
      const leftLength = normalizeSpeechText(String(left.entry.observedText || '')).length
      const rightLength = normalizeSpeechText(String(right.entry.observedText || '')).length
      if (leftLength !== rightLength)
        return rightLength - leftLength
      return String(right.entry.updatedAt || '').localeCompare(String(left.entry.updatedAt || ''))
    })

  return matches[0] || null
}

export function applySpeechPostprocess(input: SpeechPostprocessInput): SpeechPostprocessResult {
  const trimmed = input.text.trim()
  const correctionMatch = applyCorrectionMemory(trimmed, input.correctionMemoryEntries || [], input.contextTerms || [])
  const corrected = correctionMatch?.correctedText || trimmed
  const muscleMatch = applyDeterministicMuscleMemory(corrected, input.muscleMemoryEntries || [])
  const memoryText = muscleMatch?.output || null
  const textBeforeSemantic = memoryText || corrected
  const selfCorrectedText = refineSpeechSelfCorrectionText(textBeforeSemantic)
  const semanticText = refineSpeechSemanticText({
    text: selfCorrectedText,
    contextTerms: input.contextTerms,
  })

  if (memoryText) {
    return {
      text: semanticText,
      status: 'memory-written',
      correctionEntry: correctionMatch?.entry,
      muscleEntry: muscleMatch?.entry,
    }
  }

  if (correctionMatch) {
    return {
      text: semanticText,
      status: 'correction-memory-written',
      correctionEntry: correctionMatch.entry,
    }
  }

  if (selfCorrectedText !== textBeforeSemantic) {
    return {
      text: semanticText,
      status: 'self-correction-written',
    }
  }

  if (semanticText !== trimmed) {
    return {
      text: semanticText,
      status: 'semantic-written',
    }
  }

  return {
    text: trimmed,
    status: 'written',
  }
}
