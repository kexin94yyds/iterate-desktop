/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import {
  advanceCodexLiveExplicitExecutionRequestPending,
  isCodexLiveAffirmativeDecision,
  isCodexLiveExecutionQuestion,
  isCodexLiveNegativeDecision,
  isDirectCodexLiveExecutionCommand,
  isExplicitCodexLiveExecutionRequest,
  resolveCodexLiveVoiceGateDecision,
} from './codexLiveVoiceDecision.ts'

test('recognizes only the canonical confirmation question', () => {
  assert.equal(isCodexLiveExecutionQuestion('需求已经确认，是否现在开始执行？'), true)
  assert.equal(isCodexLiveExecutionQuestion('那要不要现在开始执行？'), false)
  assert.equal(isCodexLiveExecutionQuestion('需求还不清楚，现在不能确认是否开始执行。'), false)
  assert.equal(isCodexLiveExecutionQuestion('我无法看到你的屏幕。'), false)
})

test('accepts natural spoken confirmation without treating a refusal as approval', () => {
  assert.equal(isCodexLiveAffirmativeDecision('那就开始吧'), true)
  assert.equal(isCodexLiveAffirmativeDecision('你直接做吧！'), true)
  assert.equal(isCodexLiveNegativeDecision('先不要执行'), true)
  assert.equal(isCodexLiveAffirmativeDecision('不可以'), false)
  assert.equal(isDirectCodexLiveExecutionCommand('先不执行'), false)
})

test('keeps ambiguous natural phrases behind the explicit assistant confirmation gate', () => {
  assert.equal(isDirectCodexLiveExecutionCommand('动手吧'), false)
  assert.equal(isDirectCodexLiveExecutionCommand('那就开始吧'), false)
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    priorUserUtteranceCount: 1,
    text: '你直接做吧',
  }), 'none')
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: true,
    priorUserUtteranceCount: 1,
    text: '你直接做吧',
  }), 'confirm')
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: true,
    priorUserUtteranceCount: 1,
    text: '先不要执行',
  }), 'decline')
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: false,
    priorUserUtteranceCount: 1,
    text: '开始执行',
  }), 'none')
})

test('allows a direct confirmation only after an explicit tool request', () => {
  assert.equal(isExplicitCodexLiveExecutionRequest('帮我搜索一下当前项目的最近提交'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('按照 xi 查一下以前怎么修的'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('你这样回溯这个技能，你回溯一下'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('我们昨天进行到哪了'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('hui1'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('xi'), true)
  assert.equal(isExplicitCodexLiveExecutionRequest('你能不能读写文件'), false)
  assert.equal(isExplicitCodexLiveExecutionRequest('不要修改任何文件'), false)
  assert.equal(isExplicitCodexLiveExecutionRequest('这个执行为什么这么慢'), false)
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: true,
    priorUserUtteranceCount: 1,
    text: '确认执行',
  }), 'confirm')

  const recallPending = advanceCodexLiveExplicitExecutionRequestPending(
    false,
    '你这样回溯这个技能，你回溯一下',
  )
  assert.equal(recallPending, true)
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: recallPending,
    priorUserUtteranceCount: 1,
    text: '开始执行',
  }), 'confirm')
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: true,
    priorUserUtteranceCount: 1,
    text: '可以',
  }), 'none')
})

test('keeps a natural research task armed while the user clarifies subagent delegation', () => {
  let pending = false
  pending = advanceCodexLiveExplicitExecutionRequestPending(
    pending,
    '你继续刚刚的调研吧，做完了汇报给我',
  )
  assert.equal(pending, true)

  pending = advanceCodexLiveExplicitExecutionRequestPending(pending, '子代理')
  assert.equal(pending, true)
  pending = advanceCodexLiveExplicitExecutionRequestPending(pending, '我说用子代理')
  assert.equal(pending, true)

  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: pending,
    priorUserUtteranceCount: 3,
    text: '开始执行',
  }), 'confirm')

  assert.equal(advanceCodexLiveExplicitExecutionRequestPending(false, '子代理'), false)
  assert.equal(advanceCodexLiveExplicitExecutionRequestPending(true, '今天天气怎么样'), false)
})

test('a fresh transport cannot reuse an old confirmation phrase', () => {
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    priorUserUtteranceCount: 0,
    text: '可以',
  }), 'none')
  assert.equal(resolveCodexLiveVoiceGateDecision({
    awaitingConfirmation: false,
    explicitRequestPending: false,
    priorUserUtteranceCount: 1,
    text: '直接执行',
  }), 'none')
})

test('recognizes direct voice tasks that should start without a second confirmation turn', () => {
  for (const request of [
    'hui1',
    'xi',
    '继续刚刚的任务',
    '检查当前代码状态',
    '把按钮改成蓝色',
    '重启当前 iterate',
  ]) {
    assert.equal(isExplicitCodexLiveExecutionRequest(request), true, request)
  }
  for (const discussion of [
    '为什么执行这么慢',
    '这个功能是什么意思',
    '你能不能修改文件',
  ]) {
    assert.equal(isExplicitCodexLiveExecutionRequest(discussion), false, discussion)
  }
})
