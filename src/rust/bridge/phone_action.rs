use super::json_fields::json_string_field;
use super::ws::BridgeMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) const PHONE_ACTION_RESULT_MAX_ENTRIES: usize = 200;
pub(super) const PHONE_ACTION_RESULT_TTL_SECS: i64 = 10 * 60;
pub(super) const PHONE_ACTION_INLINE_PAYLOAD_MAX_BYTES: usize = 32 * 1024;
pub(super) const PHONE_ACTION_JOB_PAYLOAD_MAX_BYTES: usize = 1024 * 1024;
pub(super) const PHONE_ACTION_JOB_TTL_SECS: i64 = 10 * 60;
pub(super) const PHONE_ACTION_JOB_MAX_ENTRIES: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneActionRequest {
    #[serde(default)]
    pub(super) id: Option<String>,
    pub(super) action: String,
    #[serde(default)]
    pub(super) title: Option<String>,
    #[serde(default)]
    pub(super) text: Option<String>,
    #[serde(default)]
    pub(super) url: Option<String>,
    #[serde(default)]
    pub(super) browser: Option<String>,
    #[serde(default, alias = "shortcutName")]
    pub(super) shortcut_name: Option<String>,
    #[serde(default, alias = "requiresConfirmation")]
    pub(super) requires_confirmation: bool,
    #[serde(default)]
    pub(super) source: Option<String>,
    #[serde(default, alias = "targetDeviceId")]
    pub(super) target_device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneActionPublishResponse {
    pub(super) ok: bool,
    pub(super) id: String,
    pub(super) sent: usize,
    pub(super) subscribers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneActionResultEntry {
    pub(super) id: String,
    pub(super) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) message: Option<String>,
    pub(super) received_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source_device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneActionResultResponse {
    pub(super) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<PhoneActionResultEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PhoneActionResultQuery {
    pub(super) id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(super) struct PhoneActionJobPayload {
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
}

impl PhoneActionJobPayload {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.text.is_none()
            && self.url.is_none()
            && self.browser.is_none()
            && self.shortcut_name.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PhoneActionJobEntry {
    pub(super) id: String,
    pub(super) action_id: String,
    pub(super) action: String,
    pub(super) payload: PhoneActionJobPayload,
    pub(super) payload_size_bytes: usize,
    pub(super) created_at: String,
    pub(super) expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PhoneActionJobResponse {
    pub(super) ok: bool,
    pub(super) job: PhoneActionJobEntry,
}

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&chrono::Utc))
}

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn phone_action_target_device_id(message: &BridgeMessage) -> Option<String> {
    if message.message_type != "phone_action_request" {
        return None;
    }

    json_string_field(&message.payload, &["target_device_id", "targetDeviceId"])
}

pub(super) fn phone_action_result_entry_from_message(
    message: &BridgeMessage,
    source_client_id: Option<String>,
    source_device_id: Option<String>,
) -> Option<PhoneActionResultEntry> {
    if message.message_type != "phone_action_result" {
        return None;
    }

    let id = json_string_field(&message.payload, &["id", "action_id", "actionId"])?;
    let status = json_string_field(&message.payload, &["status"])?;
    Some(PhoneActionResultEntry {
        id,
        status,
        message: json_string_field(&message.payload, &["message"]),
        received_at: chrono::Utc::now().to_rfc3339(),
        source_client_id,
        source_device_id,
    })
}

fn phone_action_result_is_expired(
    entry: &PhoneActionResultEntry,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    parse_rfc3339_utc(&entry.received_at)
        .map(|received_at| {
            received_at + chrono::Duration::seconds(PHONE_ACTION_RESULT_TTL_SECS) <= now
        })
        .unwrap_or(true)
}

pub(super) fn prune_phone_action_results(
    results: &mut HashMap<String, PhoneActionResultEntry>,
    now: chrono::DateTime<chrono::Utc>,
) {
    results.retain(|_, entry| !phone_action_result_is_expired(entry, now));
    if results.len() < PHONE_ACTION_RESULT_MAX_ENTRIES {
        return;
    }

    let mut entries = results
        .iter()
        .map(|(id, entry)| (id.clone(), entry.received_at.clone()))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    let remove_count = results
        .len()
        .saturating_sub(PHONE_ACTION_RESULT_MAX_ENTRIES - 1);
    for (id, _) in entries.into_iter().take(remove_count) {
        results.remove(&id);
    }
}

fn validate_phone_action_open_url(raw_url: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(raw_url).map_err(|_| "phone action url is invalid".to_string())?;
    match parsed.scheme() {
        "http" | "https" | "iterate" => Ok(()),
        scheme => Err(format!("open_url does not allow {} scheme", scheme)),
    }
}

fn validate_phone_action_http_url(raw_url: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(raw_url).map_err(|_| "phone action url is invalid".to_string())?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("phone action url does not allow {} scheme", scheme)),
    }
}

fn validate_phone_action_browser(browser: Option<&str>) -> Result<(), String> {
    match browser.unwrap_or("default") {
        "default" | "safari" | "chrome" | "google" => Ok(()),
        value => Err(format!(
            "unsupported phone action browser: {}. expected default/safari/chrome/google",
            value
        )),
    }
}

fn validate_phone_action_shortcut_name(shortcut_name: &str) -> Result<(), String> {
    if shortcut_name
        .trim()
        .to_ascii_lowercase()
        .starts_with("iterate")
    {
        Ok(())
    } else {
        Err("run_shortcut only allows shortcut names starting with iterate".to_string())
    }
}

fn validate_phone_action_payload(
    action: &str,
    text: Option<&str>,
    url: Option<&str>,
    browser: Option<&str>,
    shortcut_name: Option<&str>,
) -> Result<(), String> {
    match action {
        "set_input" => Ok(()),
        "append_input" | "set_clipboard" | "show_message" => {
            if text.is_some() {
                Ok(())
            } else {
                Err(format!("{} requires text", action))
            }
        }
        "start_voice" => Ok(()),
        "open_url" => {
            let Some(raw_url) = url else {
                return Err("open_url requires url".to_string());
            };
            validate_phone_action_open_url(raw_url)
        }
        "open_browser" => {
            let Some(raw_url) = url else {
                return Err("open_browser requires url".to_string());
            };
            validate_phone_action_http_url(raw_url)?;
            validate_phone_action_browser(browser)
        }
        "share_text" => {
            if text.is_none() && url.is_none() {
                return Err("share_text requires text or url".to_string());
            }
            if let Some(raw_url) = url {
                validate_phone_action_http_url(raw_url)?;
            }
            Ok(())
        }
        "run_shortcut" => {
            let Some(name) = shortcut_name else {
                return Err("run_shortcut requires shortcut_name".to_string());
            };
            validate_phone_action_shortcut_name(name)
        }
        _ => Err(format!("unsupported phone action: {}", action)),
    }
}

pub(super) fn phone_action_job_payload_from_message(
    message: &BridgeMessage,
) -> Option<PhoneActionJobPayload> {
    if message.message_type != "phone_action_request" {
        return None;
    }
    let object = message.payload.as_object()?;
    let payload = PhoneActionJobPayload {
        title: object
            .get("title")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        text: object
            .get("text")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        url: object
            .get("url")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        browser: object
            .get("browser")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        shortcut_name: object
            .get("shortcut_name")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
    };
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub(super) fn phone_action_job_payload_size(
    payload: &PhoneActionJobPayload,
) -> Result<usize, String> {
    serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .map_err(|error| format!("phone action job payload encoding failed: {}", error))
}

pub(super) fn phone_action_job_is_expired(
    job: &PhoneActionJobEntry,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    parse_rfc3339_utc(&job.expires_at)
        .map(|expires_at| expires_at <= now)
        .unwrap_or(true)
}

pub(super) fn prune_phone_action_jobs(
    jobs: &mut HashMap<String, PhoneActionJobEntry>,
    now: chrono::DateTime<chrono::Utc>,
) {
    jobs.retain(|_, job| !phone_action_job_is_expired(job, now));
    while jobs.len() > PHONE_ACTION_JOB_MAX_ENTRIES {
        let oldest_key = jobs
            .iter()
            .min_by(|(_, left), (_, right)| left.created_at.cmp(&right.created_at))
            .map(|(key, _)| key.clone());
        let Some(key) = oldest_key else { break };
        jobs.remove(&key);
    }
}

pub(super) fn attach_phone_action_job_metadata(
    message: &mut BridgeMessage,
    job: &PhoneActionJobEntry,
) {
    let Some(object) = message.payload.as_object_mut() else {
        return;
    };
    object.remove("title");
    object.remove("text");
    object.remove("url");
    object.remove("browser");
    object.remove("shortcut_name");
    object.insert("job_id".to_string(), serde_json::json!(job.id));
    object.insert(
        "job_expires_at".to_string(),
        serde_json::json!(job.expires_at),
    );
    object.insert(
        "job_payload_size_bytes".to_string(),
        serde_json::json!(job.payload_size_bytes),
    );
}

pub(super) fn build_phone_action_bridge_message(
    request: PhoneActionRequest,
    default_source: &str,
) -> Result<(String, BridgeMessage), String> {
    let action = request.action.trim().to_ascii_lowercase();
    if action.is_empty() {
        return Err("phone action is required".to_string());
    }

    let title = trimmed_optional(request.title);
    let text = trimmed_optional(request.text);
    let url = trimmed_optional(request.url);
    let browser = trimmed_optional(request.browser).map(|value| value.to_ascii_lowercase());
    let shortcut_name = trimmed_optional(request.shortcut_name);
    let source = trimmed_optional(request.source).unwrap_or_else(|| default_source.to_string());
    let target_device_id = trimmed_optional(request.target_device_id);

    validate_phone_action_payload(
        &action,
        text.as_deref(),
        url.as_deref(),
        browser.as_deref(),
        shortcut_name.as_deref(),
    )?;

    let id = trimmed_optional(request.id).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut payload = serde_json::json!({
        "id": id,
        "action": action,
        "requires_confirmation": request.requires_confirmation,
        "source": source,
    });

    if let Some(object) = payload.as_object_mut() {
        if let Some(title) = title {
            object.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(text) = text {
            object.insert("text".to_string(), serde_json::json!(text));
        }
        if let Some(url) = url {
            object.insert("url".to_string(), serde_json::json!(url));
        }
        if let Some(browser) = browser {
            object.insert("browser".to_string(), serde_json::json!(browser));
        }
        if let Some(shortcut_name) = shortcut_name {
            object.insert(
                "shortcut_name".to_string(),
                serde_json::json!(shortcut_name),
            );
        }
        if let Some(target_device_id) = target_device_id {
            object.insert(
                "target_device_id".to_string(),
                serde_json::json!(target_device_id),
            );
        }
    }

    Ok((
        id,
        BridgeMessage {
            message_type: "phone_action_request".to_string(),
            payload,
        },
    ))
}
