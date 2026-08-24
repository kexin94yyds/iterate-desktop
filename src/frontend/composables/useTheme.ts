import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { applyThemeVariables, getTheme } from '../theme'

export function useTheme() {
  const DEFAULT_THEME = 'dark'

  // 先同步应用默认深色变量，避免异步读取配置期间 popup 使用到浅色占位变量而发白
  applyThemeVariables(DEFAULT_THEME)
  const currentTheme = ref(DEFAULT_THEME)

  // 计算 Naive UI 主题
  const naiveTheme = computed(() => {
    const theme = currentTheme.value || DEFAULT_THEME
    return getTheme(theme)
  })

  // 应用主题
  function applyTheme(theme: string) {
    // 应用主题变量和类
    applyThemeVariables(theme)
    currentTheme.value = theme
  }

  // 切换主题
  async function setTheme(theme: string) {
    try {
      // 先保存到后端
      await invoke('set_theme', { theme })
      // 保存成功后应用前端主题
      applyTheme(theme)
    }
    catch (error) {
      console.error('保存主题设置失败:', error)
      // 保存失败时不应用主题，保持一致性
    }
  }

  // 加载主题设置
  async function loadTheme() {
    try {
      const theme = await invoke('get_theme')
      // 确保主题值有效
      const validTheme = (theme === 'light' || theme === 'dark') ? theme : 'dark'

      // 应用后端主题（无论是否与当前主题相同，确保状态同步）
      applyTheme(validTheme as string)
    }
    catch (error) {
      console.error('加载主题失败:', error)
      // 加载失败时使用默认深色主题
      applyTheme(DEFAULT_THEME)
    }
  }

  // 立即尝试加载主题，避免延迟
  loadTheme().catch(() => {
    // 如果加载失败，应用默认主题
    applyTheme(DEFAULT_THEME)
  })

  return {
    currentTheme,
    naiveTheme,
    setTheme,
    loadTheme,
  }
}
