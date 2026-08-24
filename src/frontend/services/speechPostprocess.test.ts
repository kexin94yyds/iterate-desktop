/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { applySpeechPostprocess } from './speechPostprocess.ts'

describe('speech postprocess', () => {
  it('applies trusted correction memory with matching context', () => {
    const result = applySpeechPostprocess({
      text: '请修一下 sell 文件',
      contextTerms: ['style', 'CSS'],
      correctionMemoryEntries: [
        {
          observedText: 'sell',
          intendedText: 'style',
          contextTerms: ['CSS'],
          confirmCount: 4,
          rejectCount: 0,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, '请修一下 style 文件')
    assert.equal(result.status, 'correction-memory-written')
  })

  it('does not apply style correction in sales context', () => {
    const result = applySpeechPostprocess({
      text: '这个 sales 客户报价不对',
      contextTerms: ['客户', '报价'],
      correctionMemoryEntries: [
        {
          observedText: 'sales',
          intendedText: 'style',
          contextTerms: ['CSS'],
          confirmCount: 5,
          rejectCount: 0,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, '这个 sales 客户报价不对')
    assert.equal(result.status, 'written')
  })

  it('blocks trusted corrections when no real context matches the transcript', () => {
    // 生产接线只允许传真实环境上下文（当前为空，转写词项由内部补充），
    // 识别 hints 不得回流；词条自身的 intendedText 不能再自我满足门禁。
    const result = applySpeechPostprocess({
      text: '把马克当格式整理一下',
      contextTerms: [],
      correctionMemoryEntries: [
        {
          observedText: '马克当',
          intendedText: 'Markdown',
          contextTerms: ['文档'],
          confirmCount: 5,
          rejectCount: 0,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, '把马克当格式整理一下')
    assert.equal(result.status, 'written')
  })

  it('applies trusted corrections when real ambient context matches entry context terms', () => {
    const result = applySpeechPostprocess({
      text: '把马克当格式整理一下',
      contextTerms: ['文档'],
      correctionMemoryEntries: [
        {
          observedText: '马克当',
          intendedText: 'Markdown',
          contextTerms: ['文档'],
          confirmCount: 5,
          rejectCount: 0,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, '把Markdown格式整理一下')
    assert.equal(result.status, 'correction-memory-written')
  })

  it('keeps explicit correction explanations unchanged', () => {
    const result = applySpeechPostprocess({
      text: '不要把 sell 识别成 style',
      contextTerms: ['style', 'CSS'],
      correctionMemoryEntries: [
        {
          observedText: 'sell',
          intendedText: 'style',
          contextTerms: ['CSS'],
          confirmCount: 5,
          rejectCount: 0,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, '不要把 sell 识别成 style')
    assert.equal(result.status, 'written')
  })

  it('preserves existing whole-transcript muscle memory substitutions', () => {
    const result = applySpeechPostprocess({
      text: '之',
      muscleMemoryEntries: [
        {
          spokenPhrase: '之',
          outputText: 'zhi',
          trainingCount: 4,
          isEnabled: true,
        },
      ],
    })

    assert.equal(result.text, 'zhi')
    assert.equal(result.status, 'memory-written')
  })

  it('keeps the final intent when the speaker corrects themselves', () => {
    const result = applySpeechPostprocess({
      text: '先发到 problems 我是说先不要发出去',
    })

    assert.equal(result.text, '先不要发出去')
    assert.equal(result.status, 'self-correction-written')
  })

  it('does not treat descriptive contrast as self-correction', () => {
    const result = applySpeechPostprocess({
      text: '这个不是重点，重点是语义理解',
      contextTerms: ['语音'],
    })

    assert.equal(result.text, '这个不是重点，重点是语义理解')
    assert.equal(result.status, 'written')
  })

  it('runs semantic refinement when no memory substitution matches', () => {
    const result = applySpeechPostprocess({
      text: '继续做雨衣对洗',
      contextTerms: ['语音识别', '解析'],
    })

    assert.equal(result.text, '继续做语义对齐')
    assert.equal(result.status, 'semantic-written')
  })
})
