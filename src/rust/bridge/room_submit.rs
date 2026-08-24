use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub(super) struct RoomSubmitRequest {
    pub(super) action: String,
    pub(super) project_path: String,
    pub(super) request_id: Option<String>,
    pub(super) room_id: String,
    pub(super) room_token: String,
    pub(super) room_storage: Option<String>,
    pub(super) target_agent: Option<String>,
    pub(super) correlation_id: String,
    pub(super) run_id: Option<String>,
    pub(super) dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RoomSubmitOutcome {
    pub(super) ok: bool,
    pub(super) status: String,
    pub(super) reason: Option<String>,
    pub(super) action: String,
    pub(super) project_path: String,
    pub(super) request_id: Option<String>,
    pub(super) room_id: Option<String>,
    pub(super) target_agent: Option<String>,
    pub(super) correlation_id: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) dedupe_key: Option<String>,
    pub(super) delivered: bool,
    pub(super) delivery_attempts: Vec<RoomDeliveryAttempt>,
}

const ROOM_SUBMIT_OUTCOME_CACHE_MAX_ENTRIES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RoomDeliveryAttempt {
    pub(super) method: String,
    pub(super) delivered: bool,
    pub(super) reason: Option<String>,
    pub(super) request_id: Option<String>,
    pub(super) project_path: String,
    pub(super) lookup_key: Option<String>,
    pub(super) route_file: Option<String>,
    pub(super) response_file: Option<String>,
    pub(super) response_file_exists: Option<bool>,
    pub(super) route_age_secs: Option<i64>,
    pub(super) route_ttl_secs: Option<i64>,
    pub(super) available_response_channel_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct RoomDeliveryResult {
    pub(super) delivered: bool,
    pub(super) attempts: Vec<RoomDeliveryAttempt>,
}

impl RoomDeliveryAttempt {
    pub(super) fn response_channel(
        project_path: &str,
        request_id: Option<&str>,
        lookup_key: &str,
        delivered: bool,
        reason: Option<&str>,
        available_count: usize,
    ) -> Self {
        Self::response_channel_with_method(
            "response_channel",
            project_path,
            request_id,
            lookup_key,
            delivered,
            reason,
            available_count,
        )
    }

    pub(super) fn response_channel_preflight(
        project_path: &str,
        request_id: Option<&str>,
        lookup_key: &str,
        delivered: bool,
        reason: Option<&str>,
        available_count: usize,
    ) -> Self {
        Self::response_channel_with_method(
            "response_channel_preflight",
            project_path,
            request_id,
            lookup_key,
            delivered,
            reason,
            available_count,
        )
    }

    fn response_channel_with_method(
        method: &str,
        project_path: &str,
        request_id: Option<&str>,
        lookup_key: &str,
        delivered: bool,
        reason: Option<&str>,
        available_count: usize,
    ) -> Self {
        Self {
            method: method.to_string(),
            delivered,
            reason: reason.map(ToOwned::to_owned),
            request_id: request_id.map(ToOwned::to_owned),
            project_path: project_path.to_string(),
            lookup_key: Some(lookup_key.to_string()),
            route_file: None,
            response_file: None,
            response_file_exists: None,
            route_age_secs: None,
            route_ttl_secs: None,
            available_response_channel_count: Some(available_count),
        }
    }

    pub(super) fn serve_response_file(
        project_path: &str,
        request_id: Option<&str>,
        delivered: bool,
        reason: Option<&str>,
        route_file: Option<&PathBuf>,
        response_file: Option<&PathBuf>,
        response_file_exists: Option<bool>,
        route_age_secs: Option<i64>,
        route_ttl_secs: i64,
    ) -> Self {
        Self::serve_response_file_with_method(
            "serve_response_file",
            project_path,
            request_id,
            delivered,
            reason,
            route_file,
            response_file,
            response_file_exists,
            route_age_secs,
            route_ttl_secs,
        )
    }

    pub(super) fn serve_response_file_preflight(
        project_path: &str,
        request_id: Option<&str>,
        delivered: bool,
        reason: Option<&str>,
        route_file: Option<&PathBuf>,
        response_file: Option<&PathBuf>,
        response_file_exists: Option<bool>,
        route_age_secs: Option<i64>,
        route_ttl_secs: i64,
    ) -> Self {
        Self::serve_response_file_with_method(
            "serve_response_file_preflight",
            project_path,
            request_id,
            delivered,
            reason,
            route_file,
            response_file,
            response_file_exists,
            route_age_secs,
            route_ttl_secs,
        )
    }

    fn serve_response_file_with_method(
        method: &str,
        project_path: &str,
        request_id: Option<&str>,
        delivered: bool,
        reason: Option<&str>,
        route_file: Option<&PathBuf>,
        response_file: Option<&PathBuf>,
        response_file_exists: Option<bool>,
        route_age_secs: Option<i64>,
        route_ttl_secs: i64,
    ) -> Self {
        Self {
            method: method.to_string(),
            delivered,
            reason: reason.map(ToOwned::to_owned),
            request_id: request_id.map(ToOwned::to_owned),
            project_path: project_path.to_string(),
            lookup_key: None,
            route_file: route_file.map(|path| path.display().to_string()),
            response_file: response_file.map(|path| path.display().to_string()),
            response_file_exists,
            route_age_secs,
            route_ttl_secs: Some(route_ttl_secs),
            available_response_channel_count: None,
        }
    }

    pub(super) fn internal(
        method: &str,
        project_path: &str,
        request_id: Option<&str>,
        delivered: bool,
        reason: Option<&str>,
    ) -> Self {
        Self {
            method: method.to_string(),
            delivered,
            reason: reason.map(ToOwned::to_owned),
            request_id: request_id.map(ToOwned::to_owned),
            project_path: project_path.to_string(),
            lookup_key: None,
            route_file: None,
            response_file: None,
            response_file_exists: None,
            route_age_secs: None,
            route_ttl_secs: None,
            available_response_channel_count: None,
        }
    }
}

impl RoomDeliveryResult {
    pub(super) fn delivered(attempt: RoomDeliveryAttempt) -> Self {
        Self {
            delivered: true,
            attempts: vec![attempt],
        }
    }

    pub(super) fn rejected(attempt: RoomDeliveryAttempt) -> Self {
        Self {
            delivered: false,
            attempts: vec![attempt],
        }
    }

    pub(super) fn rejected_with_attempts(attempts: Vec<RoomDeliveryAttempt>) -> Self {
        Self {
            delivered: false,
            attempts,
        }
    }

    pub(super) fn from_attempts(attempts: Vec<RoomDeliveryAttempt>) -> Self {
        let delivered = attempts.iter().any(|attempt| attempt.delivered);
        Self {
            delivered,
            attempts,
        }
    }

    pub(super) fn rejection_reason(&self) -> Option<&str> {
        self.attempts
            .iter()
            .rev()
            .find_map(|attempt| attempt.reason.as_deref())
    }
}

static ROOM_SUBMIT_OUTCOME_CACHE: Lazy<Mutex<HashMap<String, RoomSubmitOutcome>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(super) fn payload_string_field(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub(super) fn has_room_submit_metadata(payload: &serde_json::Value) -> bool {
    payload_string_field(payload, &["room_id", "roomId", "room_token", "roomToken"]).is_some()
}

fn room_id_is_safe(room_id: &str) -> bool {
    !room_id.is_empty()
        && !room_id.contains('/')
        && !room_id.contains('\\')
        && room_id != "."
        && room_id != ".."
}

fn room_state_path(
    project_path: &str,
    room_storage: Option<&str>,
    room_id: &str,
) -> Result<PathBuf, String> {
    if !room_id_is_safe(room_id) {
        return Err("invalid_room_id".to_string());
    }

    let base = room_storage
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(project_path).join(".cunzhi-memory/codex-room"));
    let base = if base.is_relative() {
        PathBuf::from(project_path).join(base)
    } else {
        base
    };
    if room_storage.is_some() {
        let project_root =
            std::fs::canonicalize(project_path).unwrap_or_else(|_| PathBuf::from(project_path));
        let base_for_check = std::fs::canonicalize(&base).unwrap_or_else(|_| base.clone());
        if !base_for_check.starts_with(project_root) {
            return Err("room_storage_outside_project".to_string());
        }
    }
    Ok(base.join("rooms").join(format!("{room_id}.json")))
}

fn load_room_state(
    project_path: &str,
    room_storage: Option<&str>,
    room_id: &str,
) -> Result<serde_json::Value, String> {
    let state_path = room_state_path(project_path, room_storage, room_id)?;
    let raw =
        std::fs::read_to_string(&state_path).map_err(|_| "room_state_not_found".to_string())?;
    serde_json::from_str(&raw).map_err(|_| "room_state_invalid_json".to_string())
}

pub(super) fn room_token_matches(
    project_path: &str,
    room_storage: Option<&str>,
    room_id: &str,
    room_token: &str,
) -> Result<serde_json::Value, String> {
    let room_state = load_room_state(project_path, room_storage, room_id)?;
    let expected = room_state
        .get("room_token")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "room_token_not_registered".to_string())?;
    if expected == room_token.trim() {
        Ok(room_state)
    } else {
        Err("room_token_invalid".to_string())
    }
}

fn normalize_path_for_room_compare(path: &str) -> String {
    let path = std::path::Path::new(path);
    let path = if path.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    };
    std::fs::canonicalize(&path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn room_agent_string_field(agent: &serde_json::Value, key: &str) -> Option<String> {
    agent
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn room_submit_target_matches_room_state(
    request: &RoomSubmitRequest,
    room_state: &serde_json::Value,
) -> Result<(), String> {
    let target_agent = request
        .target_agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing_target_agent".to_string())?;
    let registry = room_state
        .get("agent_registry")
        .and_then(|value| value.as_object())
        .ok_or_else(|| "target_not_registered".to_string())?;
    let agent = registry
        .get(target_agent)
        .ok_or_else(|| "target_not_registered".to_string())?;

    if let Some(expires_at) = room_agent_string_field(agent, "expires_at") {
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|_| "target_invalid_expires_at".to_string())?;
        if expires_at <= chrono::Utc::now() {
            return Err("target_expired".to_string());
        }
    }

    if room_agent_string_field(agent, "health").as_deref() != Some("healthy") {
        return Err("target_unhealthy".to_string());
    }
    if room_agent_string_field(agent, "status").as_deref() != Some("waiting_user") {
        return Err("target_not_waiting".to_string());
    }

    let workspace = room_agent_string_field(agent, "workspace")
        .ok_or_else(|| "target_workspace_missing".to_string())?;
    if normalize_path_for_room_compare(&workspace)
        != normalize_path_for_room_compare(&request.project_path)
    {
        return Err("target_workspace_mismatch".to_string());
    }

    let registered_request_id = room_agent_string_field(agent, "request_id")
        .ok_or_else(|| "target_request_id_missing".to_string())?;
    let request_id = request
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing_request_id".to_string())?;
    if registered_request_id != request_id {
        return Err("target_request_id_mismatch".to_string());
    }

    Ok(())
}

pub(super) fn room_submit_request_from_payload(
    project_path: &str,
    request_id: Option<&str>,
    payload: &serde_json::Value,
) -> Result<RoomSubmitRequest, String> {
    let action = payload_string_field(payload, &["action"]).unwrap_or_default();
    if action != "submit" {
        return Err("unsupported_room_submit_action".to_string());
    }
    let room_id = payload_string_field(payload, &["room_id", "roomId"])
        .ok_or_else(|| "missing_room_id".to_string())?;
    let room_token = payload_string_field(payload, &["room_token", "roomToken"])
        .ok_or_else(|| "missing_room_token".to_string())?;
    let correlation_id = payload_string_field(payload, &["correlation_id", "correlationId"])
        .ok_or_else(|| "missing_correlation_id".to_string())?;

    Ok(RoomSubmitRequest {
        action,
        project_path: project_path.to_string(),
        request_id: request_id.map(ToOwned::to_owned),
        room_id,
        room_token,
        room_storage: payload_string_field(payload, &["room_storage", "roomStorage"]),
        target_agent: payload_string_field(payload, &["target_agent", "targetAgent"]),
        correlation_id,
        run_id: payload_string_field(payload, &["run_id", "runId"]),
        dedupe_key: payload_string_field(payload, &["dedupe_key", "dedupeKey"]),
    })
}

pub(super) fn room_submit_outcome(
    request: Option<&RoomSubmitRequest>,
    action: &str,
    project_path: &str,
    request_id: Option<&str>,
    status: &str,
    reason: Option<&str>,
    delivered: bool,
) -> RoomSubmitOutcome {
    room_submit_outcome_with_attempts(
        request,
        action,
        project_path,
        request_id,
        status,
        reason,
        delivered,
        Vec::new(),
    )
}

pub(super) fn room_submit_outcome_with_attempts(
    request: Option<&RoomSubmitRequest>,
    action: &str,
    project_path: &str,
    request_id: Option<&str>,
    status: &str,
    reason: Option<&str>,
    delivered: bool,
    delivery_attempts: Vec<RoomDeliveryAttempt>,
) -> RoomSubmitOutcome {
    RoomSubmitOutcome {
        ok: status == "accepted",
        status: status.to_string(),
        reason: reason.map(ToOwned::to_owned),
        action: request
            .map(|request| request.action.clone())
            .unwrap_or_else(|| action.to_string()),
        project_path: request
            .map(|request| request.project_path.clone())
            .unwrap_or_else(|| project_path.to_string()),
        request_id: request
            .and_then(|request| request.request_id.clone())
            .or_else(|| request_id.map(ToOwned::to_owned)),
        room_id: request.map(|request| request.room_id.clone()),
        target_agent: request.and_then(|request| request.target_agent.clone()),
        correlation_id: request.map(|request| request.correlation_id.clone()),
        run_id: request.and_then(|request| request.run_id.clone()),
        dedupe_key: request.and_then(|request| request.dedupe_key.clone()),
        delivered,
        delivery_attempts,
    }
}

fn room_submit_dedupe_cache_key(request: &RoomSubmitRequest) -> Option<String> {
    let dedupe_key = request
        .dedupe_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        request.room_id,
        request.target_agent.as_deref().unwrap_or(""),
        request.project_path,
        request.request_id.as_deref().unwrap_or(""),
        request.run_id.as_deref().unwrap_or(""),
        dedupe_key
    ))
}

pub(super) fn cached_room_submit_outcome(request: &RoomSubmitRequest) -> Option<RoomSubmitOutcome> {
    let key = room_submit_dedupe_cache_key(request)?;
    let mut outcome = ROOM_SUBMIT_OUTCOME_CACHE.lock().ok()?.get(&key)?.clone();
    outcome.correlation_id = Some(request.correlation_id.clone());
    outcome.run_id = request.run_id.clone();
    outcome.dedupe_key = request.dedupe_key.clone();
    Some(outcome)
}

pub(super) fn remember_room_submit_outcome(
    request: &RoomSubmitRequest,
    outcome: &RoomSubmitOutcome,
) {
    if !outcome.ok {
        return;
    }
    let Some(key) = room_submit_dedupe_cache_key(request) else {
        return;
    };
    let Ok(mut cache) = ROOM_SUBMIT_OUTCOME_CACHE.lock() else {
        return;
    };
    if cache.len() >= ROOM_SUBMIT_OUTCOME_CACHE_MAX_ENTRIES {
        if let Some(oldest_key) = cache.keys().next().cloned() {
            cache.remove(&oldest_key);
        }
    }
    cache.insert(key, outcome.clone());
}

#[cfg(test)]
pub(super) fn clear_room_submit_outcome_cache_for_tests() {
    if let Ok(mut cache) = ROOM_SUBMIT_OUTCOME_CACHE.lock() {
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_submit_request_requires_room_metadata() {
        let payload = serde_json::json!({
            "action": "submit",
            "room_id": "room-a",
            "room_token": "room_secret",
            "room_storage": "/tmp/room-storage",
            "target_agent": "ai-5344",
            "correlation_id": "corr-1",
            "run_id": "run-1",
            "dedupe_key": "dedupe-1",
        });

        assert!(has_room_submit_metadata(&payload));
        let legacy_with_generic_metadata = serde_json::json!({
            "action": "submit",
            "correlation_id": "legacy-corr",
            "target_agent": "ai-legacy",
        });
        assert!(!has_room_submit_metadata(&legacy_with_generic_metadata));

        let request =
            room_submit_request_from_payload("/tmp/project", Some("serve-1"), &payload).unwrap();
        assert_eq!(request.action, "submit");
        assert_eq!(request.project_path, "/tmp/project");
        assert_eq!(request.request_id.as_deref(), Some("serve-1"));
        assert_eq!(request.room_id, "room-a");
        assert_eq!(request.room_token, "room_secret");
        assert_eq!(request.target_agent.as_deref(), Some("ai-5344"));
        assert_eq!(request.correlation_id, "corr-1");
        assert_eq!(request.run_id.as_deref(), Some("run-1"));
        assert_eq!(request.dedupe_key.as_deref(), Some("dedupe-1"));

        let missing_token = serde_json::json!({
            "action": "submit",
            "room_id": "room-a",
            "correlation_id": "corr-1",
        });
        assert_eq!(
            room_submit_request_from_payload("/tmp/project", Some("serve-1"), &missing_token)
                .unwrap_err(),
            "missing_room_token"
        );

        let unsupported = serde_json::json!({
            "action": "cancel",
            "room_id": "room-a",
            "room_token": "room_secret",
            "correlation_id": "corr-1",
        });
        assert_eq!(
            room_submit_request_from_payload("/tmp/project", Some("serve-1"), &unsupported)
                .unwrap_err(),
            "unsupported_room_submit_action"
        );
    }

    #[test]
    fn room_submit_target_must_match_room_registry() {
        let project = std::env::temp_dir().join(format!(
            "iterate-room-submit-target-test-{}",
            uuid::Uuid::new_v4()
        ));
        let storage = project.join(".cunzhi-memory/codex-room");
        let rooms = storage.join("rooms");
        std::fs::create_dir_all(&rooms).unwrap();
        let future = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
        let past = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        std::fs::write(
            rooms.join("room-a.json"),
            serde_json::json!({
                "room_id": "room-a",
                "room_token": "room_secret",
                "agent_registry": {
                    "ai-1": {
                        "agent_id": "ai-1",
                        "workspace": project.to_string_lossy(),
                        "port": "5344",
                        "request_id": "serve-1",
                        "status": "waiting_user",
                        "health": "healthy",
                        "expires_at": future,
                    }
                },
            })
            .to_string(),
        )
        .unwrap();

        let project_str = project.to_string_lossy().to_string();
        let storage_str = storage.to_string_lossy().to_string();
        let payload = serde_json::json!({
            "action": "submit",
            "room_id": "room-a",
            "room_token": "room_secret",
            "room_storage": storage_str,
            "target_agent": "ai-1",
            "correlation_id": "corr-1",
        });
        let request =
            room_submit_request_from_payload(&project_str, Some("serve-1"), &payload).unwrap();
        let room_state = room_token_matches(
            &project_str,
            request.room_storage.as_deref(),
            "room-a",
            "room_secret",
        )
        .unwrap();
        assert!(room_submit_target_matches_room_state(&request, &room_state).is_ok());

        let mut stale_request = request.clone();
        stale_request.request_id = Some("serve-stale".to_string());
        assert_eq!(
            room_submit_target_matches_room_state(&stale_request, &room_state).unwrap_err(),
            "target_request_id_mismatch"
        );

        let mut missing_request_id = request.clone();
        missing_request_id.request_id = None;
        assert_eq!(
            room_submit_target_matches_room_state(&missing_request_id, &room_state).unwrap_err(),
            "missing_request_id"
        );

        let mut missing_target = request.clone();
        missing_target.target_agent = None;
        assert_eq!(
            room_submit_target_matches_room_state(&missing_target, &room_state).unwrap_err(),
            "missing_target_agent"
        );

        let mut unregistered_target = request.clone();
        unregistered_target.target_agent = Some("ai-missing".to_string());
        assert_eq!(
            room_submit_target_matches_room_state(&unregistered_target, &room_state).unwrap_err(),
            "target_not_registered"
        );

        let mut mismatched_workspace = request.clone();
        mismatched_workspace.project_path = project.join("other").to_string_lossy().to_string();
        assert_eq!(
            room_submit_target_matches_room_state(&mismatched_workspace, &room_state).unwrap_err(),
            "target_workspace_mismatch"
        );

        let mut expired_state = room_state.clone();
        expired_state["agent_registry"]["ai-1"]["expires_at"] = serde_json::json!(past);
        assert_eq!(
            room_submit_target_matches_room_state(&request, &expired_state).unwrap_err(),
            "target_expired"
        );

        let _ = std::fs::remove_dir_all(project);
    }

    #[test]
    fn room_submit_token_matches_room_state() {
        let project =
            std::env::temp_dir().join(format!("iterate-room-submit-test-{}", uuid::Uuid::new_v4()));
        let storage = project.join(".cunzhi-memory/codex-room");
        let rooms = storage.join("rooms");
        std::fs::create_dir_all(&rooms).unwrap();
        std::fs::write(
            rooms.join("room-a.json"),
            serde_json::json!({
                "room_id": "room-a",
                "room_token": "room_secret",
            })
            .to_string(),
        )
        .unwrap();

        let project_str = project.to_string_lossy().to_string();
        let storage_str = storage.to_string_lossy().to_string();
        assert!(
            room_token_matches(&project_str, Some(&storage_str), "room-a", "room_secret").is_ok()
        );
        assert_eq!(
            room_token_matches(&project_str, Some(&storage_str), "room-a", "wrong").unwrap_err(),
            "room_token_invalid"
        );
        assert_eq!(
            room_token_matches(&project_str, Some(&storage_str), "../room-a", "room_secret")
                .unwrap_err(),
            "invalid_room_id"
        );

        let outside_storage = std::env::temp_dir().join(format!(
            "iterate-room-submit-outside-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(outside_storage.join("rooms")).unwrap();
        let outside_storage_str = outside_storage.to_string_lossy().to_string();
        assert_eq!(
            room_token_matches(
                &project_str,
                Some(&outside_storage_str),
                "room-a",
                "room_secret"
            )
            .unwrap_err(),
            "room_storage_outside_project"
        );

        let _ = std::fs::remove_dir_all(project);
        let _ = std::fs::remove_dir_all(outside_storage);
    }
}
