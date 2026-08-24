import type { ClickModifierState } from './clickModifiers.ts'
import { hasOpenModifier } from './clickModifiers.ts'

export interface LocalMarkdownLinkTarget {
  path: string
}

export interface OpenLocalPathInvokeArgs {
  path: string
  projectPath: string
  preferEditor: boolean
}

function hasScheme(value: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(value)
}

function decodeRepeatedly(value: string): string {
  let decoded = value

  for (let i = 0; i < 4; i++) {
    try {
      const next = decodeURIComponent(decoded)
      if (next === decoded)
        break
      decoded = next
    }
    catch {
      break
    }
  }

  return decoded
}

function stripQueryAndHash(value: string): string {
  const hashIndex = value.indexOf('#')
  const withoutHash = hashIndex >= 0 ? value.slice(0, hashIndex) : value
  const queryIndex = withoutHash.indexOf('?')
  return queryIndex >= 0 ? withoutHash.slice(0, queryIndex) : withoutHash
}

function stripWrappingQuotes(value: string): string {
  return value.trim().replace(/^['"]|['"]$/g, '')
}

export function isPotentialLocalMarkdownHref(href: string): boolean {
  const trimmed = stripWrappingQuotes(href)
  if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//'))
    return false

  if (hasScheme(trimmed))
    return trimmed.toLowerCase().startsWith('file://')

  return true
}

export function resolveLocalMarkdownHref(
  href: string,
  projectPath: string | null | undefined,
): LocalMarkdownLinkTarget | null {
  const trimmed = stripWrappingQuotes(href)
  if (!isPotentialLocalMarkdownHref(trimmed))
    return null

  if (!projectPath?.trim())
    return null

  let path = stripQueryAndHash(trimmed)

  if (path.toLowerCase().startsWith('file://')) {
    try {
      path = new URL(path).pathname
    }
    catch {
      path = path.replace(/^file:\/\//i, '')
    }
  }

  path = decodeRepeatedly(path)
  if (!path)
    return null

  if (path.startsWith('/'))
    return { path }

  const normalizedProjectPath = projectPath.trim().replace(/\/+$/, '')
  const normalizedRelativePath = path.replace(/^\.\/+/, '')
  return { path: `${normalizedProjectPath}/${normalizedRelativePath}` }
}

function normalizePathForProjectComparison(path: string): string {
  const withoutEditorLocation = path.replace(/:(\d+)(?::\d+)?$/, '')

  try {
    return decodeRepeatedly(new URL(`file://${withoutEditorLocation}`).pathname).replace(/\/+$/, '') || '/'
  }
  catch {
    return withoutEditorLocation.replace(/\/+$/, '') || '/'
  }
}

/**
 * This is only a UI routing decision. The Rust command canonicalizes the path
 * again before opening it, so symlinks cannot bypass the security boundary.
 */
export function isOutsideCurrentProject(
  target: LocalMarkdownLinkTarget,
  projectPath: string,
): boolean {
  const normalizedProjectPath = normalizePathForProjectComparison(projectPath)
  const normalizedTargetPath = normalizePathForProjectComparison(target.path)

  return normalizedTargetPath !== normalizedProjectPath
    && !normalizedTargetPath.startsWith(`${normalizedProjectPath}/`)
}

export function buildOpenLocalPathInvokeArgs(
  target: LocalMarkdownLinkTarget,
  projectPath: string,
  modifiers: ClickModifierState,
): OpenLocalPathInvokeArgs {
  return {
    path: target.path,
    projectPath,
    preferEditor: hasOpenModifier(modifiers),
  }
}
