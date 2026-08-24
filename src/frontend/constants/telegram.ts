// Telegram 前端常量定义

/** Telegram 官方 API 基础 URL */
export const API_BASE_URL = 'https://api.telegram.org/bot'

/** 常用 API 服务器示例 */
export const API_EXAMPLES = {
  official: 'https://api.telegram.org/bot',
  proxy_example: 'https://your-proxy.com/bot',
} as const

/** 默认 Telegram 配置 */
export const DEFAULT_CONFIG = {
  enabled: false,
  bot_token: '',
  chat_id: '',
  hide_frontend_popup: false,
  api_base_url: API_BASE_URL,
} as const
