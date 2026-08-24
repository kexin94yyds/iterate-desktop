export type DesktopSpeechRecognitionMode = 'quality' | 'privacy'

export const DESKTOP_SPEECH_RECOGNITION_MODE_KEY = 'iterate.desktopSpeechRecognitionMode'
export const DEFAULT_DESKTOP_SPEECH_RECOGNITION_MODE: DesktopSpeechRecognitionMode = 'quality'

export const desktopSpeechRecognitionModeOptions = [
  {
    label: '质量优先',
    value: 'quality',
    description: '不强制本机识别，优先 macOS Speech 的系统质量策略。',
  },
  {
    label: '隐私优先',
    value: 'privacy',
    description: '强制本机识别，保持当前桌面端隐私模式。',
  },
] as const

export function normalizeDesktopSpeechRecognitionMode(value: unknown): DesktopSpeechRecognitionMode {
  if (value === 'quality' || value === 'privacy')
    return value
  return DEFAULT_DESKTOP_SPEECH_RECOGNITION_MODE
}

export function getDesktopSpeechRecognitionMode(): DesktopSpeechRecognitionMode {
  try {
    return normalizeDesktopSpeechRecognitionMode(localStorage.getItem(DESKTOP_SPEECH_RECOGNITION_MODE_KEY))
  }
  catch {
    return DEFAULT_DESKTOP_SPEECH_RECOGNITION_MODE
  }
}

export function setDesktopSpeechRecognitionMode(value: unknown): DesktopSpeechRecognitionMode {
  const mode = normalizeDesktopSpeechRecognitionMode(value)
  try {
    localStorage.setItem(DESKTOP_SPEECH_RECOGNITION_MODE_KEY, mode)
  }
  catch {}
  return mode
}
