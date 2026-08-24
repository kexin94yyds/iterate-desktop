import { invoke } from '@tauri-apps/api/core'
import { computed } from 'vue'

/**
 * 跨平台键盘快捷键处理
 */
export function useKeyboard() {
  // 检测操作系统
  const isMac = computed(() => {
    if (typeof navigator !== 'undefined') {
      return navigator.platform.toUpperCase().includes('MAC')
    }
    return false
  })

  // 获取修饰键名称
  const modifierKey = computed(() => {
    return isMac.value ? 'Cmd' : 'Ctrl'
  })

  // 获取粘贴快捷键描述
  const pasteShortcut = computed(() => {
    return isMac.value ? '⌘+V' : 'Ctrl+V'
  })

  // 检查是否按下了粘贴快捷键
  function isPasteShortcut(event: KeyboardEvent): boolean {
    const isModifierPressed = isMac.value ? event.metaKey : event.ctrlKey
    return isModifierPressed && event.key.toLowerCase() === 'v'
  }

  // 检查是否按下了退出快捷键
  function isExitShortcut(event: KeyboardEvent): boolean {
    // macOS: Cmd+Q 或 Cmd+W
    if (isMac.value) {
      return event.metaKey && (event.key.toLowerCase() === 'q' || event.key.toLowerCase() === 'w')
    }

    // Windows: Alt+F4
    if (navigator.platform.toUpperCase().includes('WIN')) {
      return event.altKey && event.key === 'F4'
    }

    // Linux: Ctrl+Q
    return event.ctrlKey && event.key.toLowerCase() === 'q'
  }

  // 处理退出快捷键
  async function handleExitShortcut(event: KeyboardEvent) {
    if (isExitShortcut(event)) {
      event.preventDefault()
      event.stopPropagation()

      try {
        // 调用后端退出处理逻辑
        await invoke('handle_app_exit_request')
      }
      catch (error) {
        console.error('处理退出快捷键失败:', error)
      }
    }
  }

  // 获取常用快捷键描述
  function getShortcutText(action: 'paste' | 'copy' | 'cut' | 'save' | 'undo' | 'redo' | 'exit'): string {
    const prefix = isMac.value ? '⌘' : 'Ctrl'

    switch (action) {
      case 'paste':
        return `${prefix}+V`
      case 'copy':
        return `${prefix}+C`
      case 'cut':
        return `${prefix}+X`
      case 'save':
        return `${prefix}+S`
      case 'undo':
        return `${prefix}+Z`
      case 'redo':
        return isMac.value ? '⌘+Shift+Z' : 'Ctrl+Y'
      case 'exit':
        if (isMac.value)
          return '⌘+Q'
        if (navigator.platform.toUpperCase().includes('WIN'))
          return 'Alt+F4'
        return 'Ctrl+Q'
      default:
        return ''
    }
  }

  return {
    isMac,
    modifierKey,
    pasteShortcut,
    isPasteShortcut,
    isExitShortcut,
    handleExitShortcut,
    getShortcutText,
  }
}
