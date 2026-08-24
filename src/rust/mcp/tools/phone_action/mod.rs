use rmcp::{model::*, Error as McpError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BRIDGE_BASE_URL: &str = "http://127.0.0.1:8080";
const DEFAULT_WAIT_FOR_RESULT_MS: u64 = 3_000;
const MAX_WAIT_FOR_RESULT_MS: u64 = 10_000;
const POLL_INTERVAL_MS: u64 = 250;

#[derive(Debug, Deserialize)]
pub struct PhoneActionToolRequest {
    #[serde(default)]
    id: Option<String>,
    action: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default, alias = "shortcutName")]
    shortcut_name: Option<String>,
    #[serde(default, alias = "targetDeviceId")]
    target_device_id: Option<String>,
    #[serde(default)]
    wait_for_result_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct BridgePhoneActionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    browser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcut_name: Option<String>,
    requires_confirmation: bool,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_device_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PhoneActionPublishResponse {
    ok: bool,
    id: String,
    sent: usize,
    subscribers: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct PhoneActionResultEntry {
    id: String,
    status: String,
    message: Option<String>,
    received_at: String,
    source_client_id: Option<String>,
    source_device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PhoneActionResultResponse {
    ok: bool,
    result: Option<PhoneActionResultEntry>,
}

pub struct PhoneActionTool;

impl PhoneActionTool {
    pub async fn send(request: PhoneActionToolRequest) -> Result<CallToolResult, McpError> {
        let wait_for_result_ms = request
            .wait_for_result_ms
            .unwrap_or(DEFAULT_WAIT_FOR_RESULT_MS)
            .min(MAX_WAIT_FOR_RESULT_MS);
        let bridge_request = build_bridge_request(request).map_err(|error| {
            McpError::invalid_params(format!("phone_action 参数无效: {}", error), None)
        })?;

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|error| {
                McpError::internal_error(format!("创建 HTTP 客户端失败: {}", error), None)
            })?;

        let publish = publish_phone_action(&client, &bridge_request).await?;
        let result = if publish.ok && wait_for_result_ms > 0 {
            poll_phone_action_result(&client, &publish.id, wait_for_result_ms).await?
        } else {
            None
        };

        let output = serde_json::json!({
            "ok": publish.ok,
            "id": publish.id,
            "sent": publish.sent,
            "subscribers": publish.subscribers,
            "result": result,
        });

        Ok(CallToolResult::success(vec![Content::text(
            output.to_string(),
        )]))
    }
}

fn build_bridge_request(
    request: PhoneActionToolRequest,
) -> Result<BridgePhoneActionRequest, String> {
    let action = normalized_non_empty(request.action, "action")?.to_ascii_lowercase();
    let id = trimmed_optional(request.id);
    let title = trimmed_optional(request.title);
    let text = trimmed_optional(request.text);
    let url = trimmed_optional(request.url);
    let browser = trimmed_optional(request.browser).map(|value| value.to_ascii_lowercase());
    let shortcut_name = trimmed_optional(request.shortcut_name);
    let target_device_id = trimmed_optional(request.target_device_id);

    match action.as_str() {
        "set_input" => {}
        "append_input" | "set_clipboard" | "show_message" => {
            if text.is_none() {
                return Err(format!("{} 需要 text", action));
            }
        }
        "start_voice" => {}
        "open_url" => {
            let Some(raw_url) = url.as_deref() else {
                return Err("open_url 需要 url".to_string());
            };
            validate_open_url(raw_url)?;
        }
        "open_browser" => {
            let Some(raw_url) = url.as_deref() else {
                return Err("open_browser 需要 url".to_string());
            };
            validate_http_url(raw_url)?;
            validate_browser(browser.as_deref())?;
        }
        "share_text" => {
            if text.is_none() && url.is_none() {
                return Err("share_text 需要 text 或 url".to_string());
            }
            if let Some(raw_url) = url.as_deref() {
                validate_http_url(raw_url)?;
            }
        }
        "run_shortcut" => {
            let Some(name) = shortcut_name.as_deref() else {
                return Err("run_shortcut 需要 shortcut_name".to_string());
            };
            validate_shortcut_name(name)?;
        }
        _ => {
            return Err(format!(
                "不支持的 action: {}。支持 set_input/append_input/set_clipboard/show_message/start_voice/open_url/open_browser/share_text/run_shortcut",
                action
            ));
        }
    }

    Ok(BridgePhoneActionRequest {
        id,
        action,
        title,
        text,
        url,
        browser,
        shortcut_name,
        requires_confirmation: false,
        source: "mcp_phone_action".to_string(),
        target_device_id,
    })
}

fn normalized_non_empty(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(format!("{} 不能为空", field))
    } else {
        Ok(trimmed)
    }
}

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_open_url(raw_url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(raw_url).map_err(|_| "url 格式无效".to_string())?;
    match parsed.scheme() {
        "http" | "https" | "iterate" => Ok(()),
        scheme => Err(format!("open_url 不允许 {} scheme", scheme)),
    }
}

fn validate_http_url(raw_url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(raw_url).map_err(|_| "url 格式无效".to_string())?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("url 不允许 {} scheme", scheme)),
    }
}

fn validate_browser(browser: Option<&str>) -> Result<(), String> {
    match browser.unwrap_or("default") {
        "default" | "safari" | "chrome" | "google" => Ok(()),
        value => Err(format!(
            "不支持的 browser: {}。支持 default/safari/chrome/google",
            value
        )),
    }
}

fn validate_shortcut_name(name: &str) -> Result<(), String> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.starts_with("iterate") {
        Ok(())
    } else {
        Err("run_shortcut 只允许名称以 iterate 开头的快捷指令".to_string())
    }
}

async fn publish_phone_action(
    client: &reqwest::Client,
    request: &BridgePhoneActionRequest,
) -> Result<PhoneActionPublishResponse, McpError> {
    let url = format!("{}/api/phone-action", BRIDGE_BASE_URL);
    let request_builder = crate::bridge::auth::authorize_internal_bridge_request(
        client.post(&url).json(request),
        "POST",
        &url,
    )
    .map_err(|error| {
        McpError::internal_error(
            format!("签发 phone_action_request 凭据失败: {}", error),
            None,
        )
    })?;
    let response = request_builder.send().await.map_err(|error| {
        McpError::internal_error(format!("发送 phone_action_request 失败: {}", error), None)
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::internal_error(
            format!(
                "Bridge 拒绝 phone_action_request: status={} body={}",
                status, body
            ),
            None,
        ));
    }

    response
        .json::<PhoneActionPublishResponse>()
        .await
        .map_err(|error| {
            McpError::internal_error(
                format!("解析 phone_action_request 响应失败: {}", error),
                None,
            )
        })
}

async fn poll_phone_action_result(
    client: &reqwest::Client,
    id: &str,
    wait_for_result_ms: u64,
) -> Result<Option<PhoneActionResultEntry>, McpError> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_for_result_ms);
    loop {
        let url = format!("{}/api/phone-action-result", BRIDGE_BASE_URL);
        let request_builder = crate::bridge::auth::authorize_internal_bridge_request(
            client.get(&url).query(&[("id", id)]),
            "GET",
            &url,
        )
        .map_err(|error| {
            McpError::internal_error(
                format!("签发 phone_action_result 凭据失败: {}", error),
                None,
            )
        })?;
        let response = request_builder.send().await.map_err(|error| {
            McpError::internal_error(format!("读取 phone_action_result 失败: {}", error), None)
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!(
                    "Bridge 拒绝 phone_action_result 查询: status={} body={}",
                    status, body
                ),
                None,
            ));
        }

        let body = response
            .json::<PhoneActionResultResponse>()
            .await
            .map_err(|error| {
                McpError::internal_error(
                    format!("解析 phone_action_result 响应失败: {}", error),
                    None,
                )
            })?;
        if body.ok {
            if let Some(result) = body.result {
                return Ok(Some(result));
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Ok(None);
        }
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{build_bridge_request, PhoneActionToolRequest};

    #[test]
    fn builds_safe_phone_action_payload() {
        let payload = build_bridge_request(PhoneActionToolRequest {
            id: Some(" action-1 ".to_string()),
            action: " SET_CLIPBOARD ".to_string(),
            title: None,
            text: Some(" hello ".to_string()),
            url: None,
            browser: None,
            shortcut_name: None,
            target_device_id: Some(" iphone-1 ".to_string()),
            wait_for_result_ms: None,
        })
        .expect("valid phone action");

        assert_eq!(payload.id.as_deref(), Some("action-1"));
        assert_eq!(payload.action, "set_clipboard");
        assert_eq!(payload.text.as_deref(), Some("hello"));
        assert_eq!(payload.target_device_id.as_deref(), Some("iphone-1"));
        assert!(!payload.requires_confirmation);
    }

    #[test]
    fn rejects_unsupported_action() {
        let error = build_bridge_request(PhoneActionToolRequest {
            id: None,
            action: "tap_other_app".to_string(),
            title: None,
            text: None,
            url: None,
            browser: None,
            shortcut_name: None,
            target_device_id: None,
            wait_for_result_ms: None,
        })
        .expect_err("unsupported action");

        assert!(error.contains("不支持"));
    }

    #[test]
    fn rejects_unsafe_open_url_scheme() {
        let error = build_bridge_request(PhoneActionToolRequest {
            id: None,
            action: "open_url".to_string(),
            title: None,
            text: None,
            url: Some("javascript:alert(1)".to_string()),
            browser: None,
            shortcut_name: None,
            target_device_id: None,
            wait_for_result_ms: None,
        })
        .expect_err("unsafe url");

        assert!(error.contains("不允许"));
    }

    #[test]
    fn builds_open_browser_payload() {
        let payload = build_bridge_request(PhoneActionToolRequest {
            id: None,
            action: "open_browser".to_string(),
            title: None,
            text: None,
            url: Some("https://iterate.xin".to_string()),
            browser: Some("CHROME".to_string()),
            shortcut_name: None,
            target_device_id: None,
            wait_for_result_ms: None,
        })
        .expect("valid open browser action");

        assert_eq!(payload.action, "open_browser");
        assert_eq!(payload.browser.as_deref(), Some("chrome"));
    }

    #[test]
    fn rejects_non_iterate_shortcut_name() {
        let error = build_bridge_request(PhoneActionToolRequest {
            id: None,
            action: "run_shortcut".to_string(),
            title: None,
            text: None,
            url: None,
            browser: None,
            shortcut_name: Some("Send Message".to_string()),
            target_device_id: None,
            wait_for_result_ms: None,
        })
        .expect_err("unsafe shortcut name");

        assert!(error.contains("iterate"));
    }
}
