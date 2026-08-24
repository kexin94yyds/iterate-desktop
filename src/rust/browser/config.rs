use serde::{Deserialize, Serialize};

/// 支持的 AI 聊天网站配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSiteConfig {
    pub name: String,
    pub url_pattern: String,
    pub stop_button_selector: String,
    pub message_container_selector: String,
    pub typing_indicator_selector: Option<String>,
}

impl Default for AiSiteConfig {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            url_pattern: "".to_string(),
            stop_button_selector: "".to_string(),
            message_container_selector: "".to_string(),
            typing_indicator_selector: None,
        }
    }
}

/// 预定义的 AI 网站配置
pub fn get_ai_site_configs() -> Vec<AiSiteConfig> {
    vec![
        AiSiteConfig {
            name: "ChatGPT".to_string(),
            url_pattern: "chat.openai.com".to_string(),
            stop_button_selector: r#"button[aria-label="Stop generating"]"#.to_string(),
            message_container_selector: r#"div[data-message-author-role="assistant"]"#.to_string(),
            typing_indicator_selector: Some(r#"div.result-streaming"#.to_string()),
        },
        AiSiteConfig {
            name: "ChatGPT (chatgpt.com)".to_string(),
            url_pattern: "chatgpt.com".to_string(),
            stop_button_selector: r#"button[aria-label="Stop generating"]"#.to_string(),
            message_container_selector: r#"div[data-message-author-role="assistant"]"#.to_string(),
            typing_indicator_selector: Some(r#"div.result-streaming"#.to_string()),
        },
        AiSiteConfig {
            name: "Google Gemini".to_string(),
            url_pattern: "gemini.google.com".to_string(),
            stop_button_selector: r#"button[aria-label="Stop"]"#.to_string(),
            message_container_selector: r#"model-response"#.to_string(),
            typing_indicator_selector: Some(r#"loading-indicator"#.to_string()),
        },
        AiSiteConfig {
            name: "Google AI Studio".to_string(),
            url_pattern: "aistudio.google.com".to_string(),
            stop_button_selector: r#"button[aria-label="Stop"]"#.to_string(),
            message_container_selector: r#".model-response"#.to_string(),
            typing_indicator_selector: None,
        },
        AiSiteConfig {
            name: "Claude".to_string(),
            url_pattern: "claude.ai".to_string(),
            stop_button_selector: r#"button[aria-label="Stop Response"]"#.to_string(),
            message_container_selector: r#"div.font-claude-message"#.to_string(),
            typing_indicator_selector: Some(r#"div[data-is-streaming="true"]"#.to_string()),
        },
        AiSiteConfig {
            name: "Poe".to_string(),
            url_pattern: "poe.com".to_string(),
            stop_button_selector: r#"button[class*="StopButton"]"#.to_string(),
            message_container_selector: r#"div[class*="Message_botMessageBubble"]"#.to_string(),
            typing_indicator_selector: None,
        },
    ]
}

/// 根据 URL 匹配 AI 网站配置
pub fn match_ai_site(url: &str) -> Option<AiSiteConfig> {
    let configs = get_ai_site_configs();
    for config in configs {
        if url.contains(&config.url_pattern) {
            return Some(config);
        }
    }
    None
}

/// 浏览器监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMonitorConfig {
    pub enabled: bool,
    pub chrome_debug_port: u16,
    pub poll_interval_ms: u64,
    pub notification_sound: bool,
}

impl Default for BrowserMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            chrome_debug_port: 9222,
            poll_interval_ms: 1000,
            notification_sound: true,
        }
    }
}
