/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { refineSpeechSelfCorrectionText, refineSpeechSemanticText } from './speechSemanticResolver.ts'

describe('speech semantic resolver', () => {
  it('corrects semantic phrases in voice-related context', () => {
    assert.equal(
      refineSpeechSemanticText({
        text: '我们做雨衣对洗和鱼解释',
        contextTerms: ['语音', '识别'],
      }),
      '我们做语义对齐和语义解析',
    )
  })

  it('does not rewrite matching phrases outside semantic context', () => {
    assert.equal(
      refineSpeechSemanticText({
        text: '雨衣还没买',
        contextTerms: ['购物'],
      }),
      '雨衣还没买',
    )
  })

  it('honors bounded self-correction markers', () => {
    assert.equal(
      refineSpeechSelfCorrectionText('刚才那个方案，我是说第二个方案'),
      '第二个方案',
    )
    assert.equal(
      refineSpeechSelfCorrectionText('use the red one sorry i mean the blue one'),
      'the blue one',
    )
  })

  it('does not truncate when a marker is embedded in surrounding words', () => {
    assert.equal(
      refineSpeechSelfCorrectionText('他骂我是说我懒'),
      '他骂我是说我懒',
    )
    assert.equal(
      refineSpeechSelfCorrectionText('i meant to say hello'),
      'i meant to say hello',
    )
  })
})
