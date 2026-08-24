import { invoke } from '@tauri-apps/api/core'

const LOOPBACK_HOSTS = new Set(['127.0.0.1', 'localhost', '[::1]'])

export function normalizeBridgeUrlForFetch(input: string | URL): URL {
  const url = input instanceof URL ? new URL(input.href) : new URL(input)
  if (
    url.protocol !== 'http:'
    || !LOOPBACK_HOSTS.has(url.hostname)
    || url.port !== '8080'
    || url.username !== ''
    || url.password !== ''
  ) {
    throw new Error('bridge_fetch_requires_loopback_8080')
  }
  return url
}

/**
 * Authenticated fetch for the native desktop renderer.
 *
 * The bearer is minted through Tauri IPC, lasts only 20 seconds, and is bound
 * to this exact method and URL path. It is never reused for another request or
 * forwarded to a non-loopback origin.
 */
export async function bridgeFetch(input: string | URL, init: RequestInit = {}): Promise<Response> {
  const url = normalizeBridgeUrlForFetch(input)
  const method = (init.method || 'GET').trim().toUpperCase()
  const token = await invoke<string>('get_bridge_desktop_token', {
    method,
    path: url.pathname,
  })
  if (!token)
    throw new Error('bridge_desktop_token_unavailable')

  const headers = new Headers(init.headers)
  headers.set('Authorization', `Bearer ${token}`)
  return await fetch(url, {
    ...init,
    method,
    headers,
    credentials: 'omit',
  })
}
