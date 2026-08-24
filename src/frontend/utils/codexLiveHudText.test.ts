/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import { resolveCodexLiveHudText } from './codexLiveHudText.ts'

test('keeps realtime transcript visible while GPT-Live is only listening', () => {
  assert.equal(resolveCodexLiveHudText({
    executionPhase: 'waiting',
    statusText: 'GPT-Live 已连接，可以开始说话',
    latestTranscript: '你：帮我看看这个问题',
  }), '你：帮我看看这个问题')
})

test('shows task progress instead of stale transcript while Codex is executing', () => {
  assert.equal(resolveCodexLiveHudText({
    executionPhase: 'running',
    statusText: 'Codex 正在执行',
    taskProgressText: '正在运行：pnpm test:desktop-codex-live',
    latestTranscript: 'Codex：我现在开始处理',
  }), '正在运行：pnpm test:desktop-codex-live')
})

test('keeps completion and failure reports visible until the next user turn', () => {
  for (const executionPhase of ['submitting', 'completed', 'failed'] as const) {
    assert.equal(resolveCodexLiveHudText({
      executionPhase,
      statusText: 'Codex 已完成',
      taskProgressText: 'Codex 已完成当前任务',
      latestTranscript: '你：开始执行',
    }), 'Codex 已完成当前任务')
  }
})

test('falls back safely when the local UI snapshot has not arrived', () => {
  assert.equal(resolveCodexLiveHudText({
    fallbackStatusText: '正在连接 Codex GPT-Live',
  }), '正在连接 Codex GPT-Live')
})
