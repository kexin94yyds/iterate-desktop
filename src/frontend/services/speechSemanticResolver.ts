import { normalizeSpeechText } from './speechContext.ts'

export interface SpeechSemanticResolverInput {
  text: string
  contextTerms?: string[]
}

const SEMANTIC_CONTEXT_TERMS = [
  '语音',
  '语义',
  '识别',
  '文本',
  '输入',
  '命令',
  '对齐',
  '解析',
]

const PHRASE_CORRECTIONS: Array<[observed: string, intended: string]> = [
  ['雨衣对洗', '语义对齐'],
  ['雨衣对齐', '语义对齐'],
  ['语音对洗', '语义对齐'],
  ['鱼解释', '语义解析'],
  ['雨衣解析', '语义解析'],
  ['对洗', '对齐'],
  ['雨衣', '语义'],
]

const SELF_CORRECTION_MARKERS = [
  '我想说的是',
  '我说的是',
  '我是说',
  '重新说',
  '重说一遍',
  '说错了',
  '准确说',
  '更准确地说',
  '口误',
  'sorry i mean',
  'no wait',
  'i mean',
]

function hasSemanticContext(input: SpeechSemanticResolverInput) {
  const normalizedContext = normalizeSpeechText([
    input.text,
    ...(input.contextTerms || []),
  ].join(' '))

  return SEMANTIC_CONTEXT_TERMS.some(term => normalizedContext.includes(normalizeSpeechText(term)))
}

function isExplicitCorrectionExplanation(text: string) {
  return ['不要把', '识别成', '纠错', '替换', '->', '=>']
    .some(marker => text.toLocaleLowerCase().includes(marker.toLocaleLowerCase()))
}

function trimSelfCorrectionTail(text: string) {
  return text.replace(/^[\s,，.。:：;；、!！?？\-—]+/u, '').trim()
}

function isAsciiWordChar(character: string) {
  return /\w/.test(character)
}

// 标记词必须落在真实边界上，否则"他骂我是说我懒"会被"我是说"腰斩、
// "i meant" 会被 "i mean" 误命中。宁可漏判保留原文，也不误判删掉用户的话。
function hasMarkerBoundaries(lower: string, index: number, needle: string) {
  const before = index > 0 ? lower[index - 1] : ''
  if (before && !/[\s\p{P}\p{S}]/u.test(before))
    return false
  if (isAsciiWordChar(needle[needle.length - 1])) {
    const after = lower[index + needle.length] ?? ''
    if (after && isAsciiWordChar(after))
      return false
  }
  return true
}

function findBoundedMarkerEnd(lower: string) {
  let markerEnd = -1
  for (const marker of SELF_CORRECTION_MARKERS) {
    const needle = marker.toLocaleLowerCase()
    let index = lower.lastIndexOf(needle)
    while (index >= 0) {
      if (hasMarkerBoundaries(lower, index, needle)) {
        if (index + needle.length > markerEnd)
          markerEnd = index + needle.length
        break
      }
      index = index === 0 ? -1 : lower.lastIndexOf(needle, index - 1)
    }
  }
  return markerEnd
}

export function refineSpeechSelfCorrectionText(text: string) {
  const trimmed = text.trim()
  if (!trimmed || isExplicitCorrectionExplanation(trimmed))
    return trimmed

  const markerEnd = findBoundedMarkerEnd(trimmed.toLocaleLowerCase())
  if (markerEnd < 0)
    return trimmed

  const tail = trimSelfCorrectionTail(trimmed.slice(markerEnd))
  return tail.length >= 2 ? tail : trimmed
}

export function refineSpeechSemanticText(input: SpeechSemanticResolverInput) {
  const trimmed = input.text.trim()
  if (!trimmed || !hasSemanticContext({ ...input, text: trimmed }))
    return trimmed

  return PHRASE_CORRECTIONS.reduce(
    (resolved, [observed, intended]) => resolved.replaceAll(observed, intended),
    trimmed,
  )
}
