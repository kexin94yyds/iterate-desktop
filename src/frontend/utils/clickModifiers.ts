export interface ClickModifierState {
  metaKey?: boolean
  ctrlKey?: boolean
}

export function hasOpenModifier(event: ClickModifierState): boolean {
  return Boolean(event.metaKey || event.ctrlKey)
}
