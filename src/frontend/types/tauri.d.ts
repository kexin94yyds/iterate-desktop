declare module '@tauri-apps/plugin-shell' {
  export function open(url: string): Promise<void>
}

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke?: (command: string, args?: Record<string, unknown>) => Promise<unknown>
      }
    }
    __TAURI_INTERNALS__?: {
      invoke?: (command: string, args?: Record<string, unknown>, options?: unknown) => Promise<unknown>
      transformCallback?: (callback?: unknown, once?: boolean) => number
      unregisterCallback?: (id: number) => void
      metadata?: {
        currentWindow?: { label: string }
        currentWebview?: { label: string }
      }
    }
  }
}

export {}
