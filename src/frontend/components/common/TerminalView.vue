<script setup lang="ts">
import type { ITheme } from 'xterm'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { useTheme } from '../../composables/useTheme'
import 'xterm/css/xterm.css'

const props = defineProps<{
  active: boolean
  projectPath?: string
}>()

const { currentTheme } = useTheme()

// 终端配色方案 - Tokyo Night 风格
const terminalThemes: Record<string, ITheme> = {
  // 浅色主题: 白色背景 (macOS Basic 风格)
  light: {
    background: '#ffffff',
    foreground: '#000000',
    cursor: '#000000',
    cursorAccent: '#ffffff',
    selectionBackground: '#b5d5ff',
    selectionForeground: '#000000',
    black: '#000000',
    red: '#c41a16',
    green: '#007400',
    yellow: '#826b28',
    blue: '#0000ff',
    magenta: '#aa0d91',
    cyan: '#318495',
    white: '#686868',
    brightBlack: '#686868',
    brightRed: '#c41a16',
    brightGreen: '#007400',
    brightYellow: '#826b28',
    brightBlue: '#0000ff',
    brightMagenta: '#aa0d91',
    brightCyan: '#318495',
    brightWhite: '#000000',
  },
  // 深色主题: Tokyo Night 风格
  dark: {
    background: '#1a1b26',
    foreground: '#c0caf5',
    cursor: '#c0caf5',
    cursorAccent: '#1a1b26',
    selectionBackground: '#33467c',
    selectionForeground: '#c0caf5',
    black: '#15161e',
    red: '#f7768e',
    green: '#9ece6a',
    yellow: '#e0af68',
    blue: '#7aa2f7',
    magenta: '#bb9af7',
    cyan: '#7dcfff',
    white: '#a9b1d6',
    brightBlack: '#414868',
    brightRed: '#f7768e',
    brightGreen: '#9ece6a',
    brightYellow: '#e0af68',
    brightBlue: '#7aa2f7',
    brightMagenta: '#bb9af7',
    brightCyan: '#7dcfff',
    brightWhite: '#c0caf5',
  },
}

// 当前终端主题
const currentTerminalTheme = computed(() => {
  const theme = currentTheme.value || 'dark'
  return terminalThemes[theme] || terminalThemes.dark
})

// 当前背景色（用于容器样式）
const currentBgColor = computed(() => currentTerminalTheme.value.background)

const terminalElement = ref<HTMLElement | null>(null)
let term: Terminal | null = null
let fitAddon: FitAddon | null = null
let unlisten: (() => void) | null = null

onMounted(async () => {
  if (!terminalElement.value) {
    return
  }

  term = new Terminal({
    cursorBlink: true,
    fontSize: 12,
    lineHeight: 1.2,
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    theme: currentTerminalTheme.value,
    minimumContrastRatio: 4.5,
    allowProposedApi: true,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalElement.value)
  fitAddon.fit()

  // 监听来自 PTY 的数据
  unlisten = await listen<{ data: string }>('terminal-data', (event) => {
    term?.write(event.payload.data)
  })

  // 发送输入到 PTY
  term.onData((data) => {
    invoke('write_to_pty', { data })
  })

  // 开启 PTY
  await invoke('open_pty', { cwd: props.projectPath })

  window.addEventListener('resize', handleResize)
})

function handleResize() {
  fitAddon?.fit()
  if (term) {
    invoke('resize_pty', { rows: term.rows, cols: term.cols })
  }
}

onUnmounted(() => {
  if (unlisten) {
    unlisten()
  }
  window.removeEventListener('resize', handleResize)
  term?.dispose()
})

watch(() => props.active, (isActive) => {
  if (isActive) {
    setTimeout(() => {
      fitAddon?.fit()
      term?.focus()
    }, 100)
  }
})

// 监听主题变化，更新终端配色
watch(currentTerminalTheme, (newTheme) => {
  if (term) {
    term.options.theme = newTheme
    // 强制刷新终端视口以应用新主题
    fitAddon?.fit()
  }
})
</script>

<template>
  <div
    class="terminal-container w-full h-full p-2 rounded-lg border"
    :class="currentTheme === 'light' ? 'border-gray-300' : 'border-gray-700'"
    :style="{ backgroundColor: currentBgColor }"
  >
    <div ref="terminalElement" class="w-full h-full" />
  </div>
</template>

<style scoped>
.terminal-container {
  min-height: 200px;
}
</style>
