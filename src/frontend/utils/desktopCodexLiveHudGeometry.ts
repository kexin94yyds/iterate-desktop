export interface HudPoint {
  x: number
  y: number
}

export interface HudSize {
  width: number
  height: number
}

export interface HudBounds extends HudPoint, HudSize {}

export const DESKTOP_CODEX_LIVE_COMPACT_SIZE: HudSize = { width: 96, height: 48 }
export const DESKTOP_CODEX_LIVE_COLLAPSED_SIZE: HudSize = { width: 126, height: 48 }
export const DESKTOP_CODEX_LIVE_EXPANDED_SIZE: HudSize = { width: 420, height: 156 }

const EDGE_MARGIN = 8

export function containsPoint(bounds: HudBounds, point: HudPoint): boolean {
  return point.x >= bounds.x
    && point.x <= bounds.x + bounds.width
    && point.y >= bounds.y
    && point.y <= bounds.y + bounds.height
}

export function centerFromPosition(position: HudPoint, size: HudSize, scaleFactor: number): HudPoint {
  return {
    x: position.x + (size.width * scaleFactor) / 2,
    y: position.y + (size.height * scaleFactor) / 2,
  }
}

export function positionForCenter(
  center: HudPoint,
  size: HudSize,
  scaleFactor: number,
  bounds: HudBounds,
): HudPoint {
  const physicalWidth = size.width * scaleFactor
  const physicalHeight = size.height * scaleFactor
  const minX = bounds.x + EDGE_MARGIN
  const minY = bounds.y + EDGE_MARGIN
  const maxX = Math.max(minX, bounds.x + bounds.width - physicalWidth - EDGE_MARGIN)
  const maxY = Math.max(minY, bounds.y + bounds.height - physicalHeight - EDGE_MARGIN)
  return {
    x: Math.min(maxX, Math.max(minX, center.x - physicalWidth / 2)),
    y: Math.min(maxY, Math.max(minY, center.y - physicalHeight / 2)),
  }
}

export function positionForRightEdgeCenter(
  rightEdge: number,
  centerY: number,
  size: HudSize,
  scaleFactor: number,
  bounds: HudBounds,
): HudPoint {
  const physicalWidth = size.width * scaleFactor
  const physicalHeight = size.height * scaleFactor
  const minX = bounds.x + EDGE_MARGIN
  const minY = bounds.y + EDGE_MARGIN
  const maxX = Math.max(minX, bounds.x + bounds.width - physicalWidth - EDGE_MARGIN)
  const maxY = Math.max(minY, bounds.y + bounds.height - physicalHeight - EDGE_MARGIN)
  return {
    x: Math.min(maxX, Math.max(minX, rightEdge - physicalWidth)),
    y: Math.min(maxY, Math.max(minY, centerY - physicalHeight / 2)),
  }
}

export function parseStoredHudCenter(raw: string | null): HudPoint | null {
  if (!raw)
    return null
  try {
    const parsed = JSON.parse(raw) as Partial<HudPoint>
    if (!Number.isFinite(parsed.x) || !Number.isFinite(parsed.y))
      return null
    return { x: parsed.x as number, y: parsed.y as number }
  }
  catch {
    return null
  }
}
