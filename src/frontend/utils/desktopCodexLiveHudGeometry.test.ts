/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import test from 'node:test'
import {
  centerFromPosition,
  containsPoint,
  DESKTOP_CODEX_LIVE_COLLAPSED_SIZE,
  DESKTOP_CODEX_LIVE_COMPACT_SIZE,
  DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
  parseStoredHudCenter,
  positionForCenter,
  positionForRightEdgeCenter,
} from './desktopCodexLiveHudGeometry.ts'

test('keeps the same physical center when compact HUD expands', () => {
  const center = centerFromPosition({ x: 500, y: 300 }, DESKTOP_CODEX_LIVE_COMPACT_SIZE, 2)
  const position = positionForCenter(
    center,
    DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
    2,
    { x: 0, y: 0, width: 2880, height: 1800 },
  )

  assert.deepEqual(position, { x: 176, y: 192 })
})

test('keeps the same physical center when the active HUD is manually collapsed', () => {
  const center = centerFromPosition({ x: 500, y: 300 }, DESKTOP_CODEX_LIVE_EXPANDED_SIZE, 2)
  const position = positionForCenter(
    center,
    DESKTOP_CODEX_LIVE_COLLAPSED_SIZE,
    2,
    { x: 0, y: 0, width: 2880, height: 1800 },
  )

  assert.deepEqual(
    centerFromPosition(position, DESKTOP_CODEX_LIVE_COLLAPSED_SIZE, 2),
    center,
  )
})

test('keeps the fold-side right edge fixed while the HUD expands leftward', () => {
  const collapsedCenter = centerFromPosition(
    { x: 1000, y: 300 },
    DESKTOP_CODEX_LIVE_COLLAPSED_SIZE,
    2,
  )
  const collapsedRightEdge = collapsedCenter.x + DESKTOP_CODEX_LIVE_COLLAPSED_SIZE.width
  const position = positionForRightEdgeCenter(
    collapsedRightEdge,
    collapsedCenter.y,
    DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
    2,
    { x: 0, y: 0, width: 2880, height: 1800 },
  )

  assert.deepEqual(position, { x: 412, y: 192 })
  assert.equal(position.x + DESKTOP_CODEX_LIVE_EXPANDED_SIZE.width * 2, collapsedRightEdge)
})

test('clamps a right-anchored expansion inside the monitor work area', () => {
  const position = positionForRightEdgeCenter(
    -5,
    20,
    DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
    1,
    { x: -1920, y: 0, width: 1920, height: 1080 },
  )

  assert.deepEqual(position, { x: -428, y: 8 })
})

test('clamps an expanded HUD inside a negative-coordinate monitor', () => {
  const position = positionForCenter(
    { x: -5, y: 20 },
    DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
    1,
    { x: -1920, y: 0, width: 1920, height: 1080 },
  )

  assert.deepEqual(position, { x: -428, y: 8 })
  assert.equal(containsPoint({ x: -1920, y: 0, width: 1920, height: 1080 }, { x: -5, y: 20 }), true)
})

test('clamps against a monitor work area with menu bar and Dock insets', () => {
  const position = positionForCenter(
    { x: 720, y: 10 },
    DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
    2,
    { x: 0, y: 48, width: 2880, height: 1650 },
  )

  assert.deepEqual(position, { x: 300, y: 56 })
})

test('rejects damaged persisted centers', () => {
  assert.deepEqual(parseStoredHudCenter('{"x":120,"y":240}'), { x: 120, y: 240 })
  assert.equal(parseStoredHudCenter('{"x":"bad","y":240}'), null)
  assert.equal(parseStoredHudCenter('broken'), null)
})
