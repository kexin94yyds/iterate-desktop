import type { UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import {
  availableMonitors,
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
} from '@tauri-apps/api/window'
import {
  centerFromPosition,
  containsPoint,
  DESKTOP_CODEX_LIVE_COLLAPSED_SIZE,
  DESKTOP_CODEX_LIVE_COMPACT_SIZE,
  DESKTOP_CODEX_LIVE_EXPANDED_SIZE,
  parseStoredHudCenter,
  positionForRightEdgeCenter,
} from '../utils/desktopCodexLiveHudGeometry'
import { createLatestValueTaskQueue } from '../utils/latestValueTaskQueue'

const POSITION_STORAGE_KEY = 'iterate.desktop-codex-live.hud-center.v1'
const MOVE_PERSIST_DEBOUNCE_MS = 160
const NATIVE_FRAME_SETTLE_MS = 180
const NATIVE_FRAME_ANIMATION_COMMAND = 'animate_speech_overlay_frame'

export type DesktopCodexLiveHudLayoutMode = 'compact' | 'collapsed' | 'expanded'

function sizeForMode(mode: DesktopCodexLiveHudLayoutMode) {
  if (mode === 'expanded')
    return DESKTOP_CODEX_LIVE_EXPANDED_SIZE
  if (mode === 'collapsed')
    return DESKTOP_CODEX_LIVE_COLLAPSED_SIZE
  return DESKTOP_CODEX_LIVE_COMPACT_SIZE
}

function readStoredCenter(positionStorageKey: string) {
  try {
    return parseStoredHudCenter(localStorage.getItem(positionStorageKey))
  }
  catch {
    return null
  }
}

function writeStoredCenter(positionStorageKey: string, center: { x: number, y: number }) {
  try {
    localStorage.setItem(positionStorageKey, JSON.stringify(center))
  }
  catch {
    // Window placement is a best-effort local preference.
  }
}

export function useDesktopCodexLiveHudLayout(positionStorageKey = POSITION_STORAGE_KEY) {
  const hudWindow = getCurrentWindow()
  let currentSize = DESKTOP_CODEX_LIVE_COMPACT_SIZE
  let center = readStoredCenter(positionStorageKey)
  let unlistenMoved: UnlistenFn | null = null
  let layoutOperations = 0
  let moveRevision = 0
  let moveTimer: number | null = null
  let nativeSettleTimer: number | null = null
  let lastRequestedMode: DesktopCodexLiveHudLayoutMode | null = null
  let hasAppliedLayout = false
  let nativeFrameAnimationAvailable: boolean | null = null
  let disposed = false

  function prefersReducedMotion() {
    return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
  }

  function scheduleNativeCenterPersistence() {
    if (nativeSettleTimer !== null)
      window.clearTimeout(nativeSettleTimer)
    nativeSettleTimer = window.setTimeout(() => {
      nativeSettleTimer = null
      void Promise.all([
        hudWindow.scaleFactor(),
        hudWindow.outerPosition(),
      ]).then(([scaleFactor, position]) => {
        if (disposed)
          return
        center = centerFromPosition(position, currentSize, scaleFactor)
        writeStoredCenter(positionStorageKey, center)
      }).catch((error) => {
        console.warn('[GPT-Live HUD] native frame settle persistence failed', error)
      })
    }, NATIVE_FRAME_SETTLE_MS)
  }

  async function applyNativeFrame(nextSize: { width: number, height: number }) {
    if (!hasAppliedLayout || nativeFrameAnimationAvailable === false)
      return false
    try {
      await invoke(NATIVE_FRAME_ANIMATION_COMMAND, {
        request: {
          targetContentWidthPoints: nextSize.width,
          targetContentHeightPoints: nextSize.height,
          reducedMotion: prefersReducedMotion(),
        },
      })
      nativeFrameAnimationAvailable = true
      currentSize = nextSize
      scheduleNativeCenterPersistence()
      return true
    }
    catch (error) {
      nativeFrameAnimationAvailable = false
      console.warn('[GPT-Live HUD] native frame animation unavailable; using Tauri fallback', error)
      return false
    }
  }

  async function monitorGeometry() {
    return (await availableMonitors()).map(monitor => ({
      bounds: {
        x: monitor.workArea.position.x,
        y: monitor.workArea.position.y,
        width: monitor.workArea.size.width,
        height: monitor.workArea.size.height,
      },
      scaleFactor: monitor.scaleFactor,
    }))
  }

  async function applyMode(mode: DesktopCodexLiveHudLayoutMode) {
    if (disposed)
      return
    const nextSize = sizeForMode(mode)
    layoutOperations += 1
    try {
      if (await applyNativeFrame(nextSize))
        return

      const monitors = await monitorGeometry()
      const storedCenterIsVisible = center
        && monitors.some(monitor => containsPoint(monitor.bounds, center!))
      let liveCenter = center
      if (!storedCenterIsVisible) {
        const [scaleFactor, currentPosition] = await Promise.all([
          hudWindow.scaleFactor(),
          hudWindow.outerPosition(),
        ])
        liveCenter = centerFromPosition(currentPosition, currentSize, scaleFactor)
      }
      if (!liveCenter)
        return
      const requestedCenter = storedCenterIsVisible ? center! : liveCenter
      const monitor = monitors.find(candidate => containsPoint(candidate.bounds, requestedCenter))
        ?? monitors.find(candidate => containsPoint(candidate.bounds, liveCenter))
        ?? monitors[0]

      if (!monitor)
        return

      const currentPhysicalWidth = currentSize.width * monitor.scaleFactor
      const nextPosition = positionForRightEdgeCenter(
        requestedCenter.x + currentPhysicalWidth / 2,
        requestedCenter.y,
        nextSize,
        monitor.scaleFactor,
        monitor.bounds,
      )
      const nextCenter = centerFromPosition(nextPosition, nextSize, monitor.scaleFactor)
      // Keep the fold affordance under the pointer: the right edge stays fixed
      // while the transcript grows leftward. Dispatch both mutations together
      // so the Tauri fallback does not expose an intermediate top-left resize.
      await Promise.all([
        hudWindow.setSize(new LogicalSize(nextSize.width, nextSize.height)),
        hudWindow.setPosition(new PhysicalPosition(
          Math.round(nextPosition.x),
          Math.round(nextPosition.y),
        )),
      ])
      currentSize = nextSize
      center = nextCenter
      hasAppliedLayout = true
      writeStoredCenter(positionStorageKey, nextCenter)
    }
    finally {
      layoutOperations = Math.max(0, layoutOperations - 1)
    }
  }

  const layoutQueue = createLatestValueTaskQueue(applyMode, (error) => {
    lastRequestedMode = null
    hasAppliedLayout = false
    nativeFrameAnimationAvailable = null
    console.warn('[GPT-Live HUD] layout update failed', error)
  })

  function requestLayout(mode: DesktopCodexLiveHudLayoutMode) {
    if (lastRequestedMode === mode)
      return Promise.resolve()
    lastRequestedMode = mode
    return layoutQueue.request(mode)
  }

  function persistLatestMove(revision: number, position: { x: number, y: number }) {
    void hudWindow.scaleFactor().then((scaleFactor) => {
      if (disposed || revision !== moveRevision || layoutOperations > 0)
        return
      center = centerFromPosition(position, currentSize, scaleFactor)
      writeStoredCenter(positionStorageKey, center)
    }).catch((error) => {
      console.warn('[GPT-Live HUD] move persistence failed', error)
    })
  }

  function initialize(mode: DesktopCodexLiveHudLayoutMode) {
    disposed = false
    void requestLayout(mode)
    void hudWindow.onMoved(({ payload }) => {
      moveRevision += 1
      const revision = moveRevision
      if (moveTimer !== null)
        window.clearTimeout(moveTimer)
      moveTimer = window.setTimeout(() => {
        moveTimer = null
        persistLatestMove(revision, payload)
      }, MOVE_PERSIST_DEBOUNCE_MS)
    }).then((unlisten) => {
      if (disposed)
        unlisten()
      else
        unlistenMoved = unlisten
    }).catch((error) => {
      console.warn('[GPT-Live HUD] move listener failed', error)
    })
  }

  function setMode(mode: DesktopCodexLiveHudLayoutMode) {
    return requestLayout(mode)
  }

  function dispose() {
    disposed = true
    moveRevision += 1
    lastRequestedMode = null
    if (moveTimer !== null)
      window.clearTimeout(moveTimer)
    if (nativeSettleTimer !== null)
      window.clearTimeout(nativeSettleTimer)
    moveTimer = null
    nativeSettleTimer = null
    hasAppliedLayout = false
    nativeFrameAnimationAvailable = null
    unlistenMoved?.()
    unlistenMoved = null
  }

  return {
    initialize,
    setMode,
    dispose,
  }
}
