// Telegram 相关常量

/// 默认 Telegram Bot 启用状态
pub const DEFAULT_ENABLED: bool = false;

/// 默认 Telegram Bot Token
pub const DEFAULT_BOT_TOKEN: &str = "";

/// 默认 Telegram Chat ID
pub const DEFAULT_CHAT_ID: &str = "";

/// 默认隐藏前端弹窗状态
pub const DEFAULT_HIDE_FRONTEND_POPUP: bool = false;

/// Telegram API 基础 URL
pub const API_BASE_URL: &str = "https://api.telegram.org/bot";

/// 消息最大长度
pub const MAX_MESSAGE_LENGTH: usize = 4096;

/// 请求超时时间 (ms)
pub const REQUEST_TIMEOUT_MS: u64 = 30000;

/// 重试次数
pub const MAX_RETRY_COUNT: u32 = 3;

/// 轮询间隔 (ms)
pub const POLLING_INTERVAL_MS: u64 = 1000;

// Telegram 配置结构体
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub hide_frontend_popup: bool,
    pub api_base_url: String,
    pub max_message_length: usize,
    pub request_timeout_ms: u64,
    pub max_retry_count: u32,
    pub polling_interval_ms: u64,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_ENABLED,
            bot_token: DEFAULT_BOT_TOKEN.to_string(),
            chat_id: DEFAULT_CHAT_ID.to_string(),
            hide_frontend_popup: DEFAULT_HIDE_FRONTEND_POPUP,
            api_base_url: API_BASE_URL.to_string(),
            max_message_length: MAX_MESSAGE_LENGTH,
            request_timeout_ms: REQUEST_TIMEOUT_MS,
            max_retry_count: MAX_RETRY_COUNT,
            polling_interval_ms: POLLING_INTERVAL_MS,
        }
    }
}

impl TelegramConfig {
    /// 验证配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.bot_token.is_empty() && !self.chat_id.is_empty()
    }

    /// 验证消息长度是否有效
    pub fn is_valid_message_length(&self, message: &str) -> bool {
        message.len() <= self.max_message_length
    }

    /// 获取 API URL
    pub fn get_api_url(&self, method: &str) -> String {
        format!("{}{}/{}", self.api_base_url, self.bot_token, method)
    }

    /// 分割长消息
    pub fn split_long_message(&self, message: &str) -> Vec<String> {
        if message.len() <= self.max_message_length {
            return vec![message.to_string()];
        }

        let mut parts = Vec::new();
        let mut current = String::new();

        for line in message.lines() {
            if current.len() + line.len() + 1 > self.max_message_length && !current.is_empty() {
                parts.push(current);
                current = String::new();
            }

            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }

        if !current.is_empty() {
            parts.push(current);
        }

        parts
    }

    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.enabled,
            "bot_token": self.bot_token,
            "chat_id": self.chat_id,
            "hide_frontend_popup": self.hide_frontend_popup,
            "api_base_url": self.api_base_url,
            "max_message_length": self.max_message_length,
            "request_timeout_ms": self.request_timeout_ms,
            "max_retry_count": self.max_retry_count,
            "polling_interval_ms": self.polling_interval_ms
        })
    }
}

// 便捷函数
/// 获取默认 Telegram 配置
pub fn get_default_telegram_config() -> TelegramConfig {
    TelegramConfig::default()
}

/// 验证 Bot Token 格式是否有效
pub fn is_valid_bot_token(token: &str) -> bool {
    !token.is_empty() && token.contains(':')
}

/// 验证 Chat ID 格式是否有效
pub fn is_valid_chat_id(chat_id: &str) -> bool {
    !chat_id.is_empty() && (chat_id.starts_with('-') || chat_id.chars().all(|c| c.is_ascii_digit()))
}
