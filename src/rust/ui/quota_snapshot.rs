use super::quota::{get_usage_quota_providers_from_config, UsageProvider};
use crate::config::{default_usage_config, AppState};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const QUOTA_SNAPSHOT_STALE_AFTER_MS: i64 = 6 * 60 * 1000;
const QUOTA_LOW_THRESHOLD_PERCENT: u8 = 10;

static BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE: Lazy<QuotaSnapshotState> =
    Lazy::new(QuotaSnapshotState::default);

#[derive(Default)]
pub struct QuotaSnapshotState {
    current: Mutex<Option<QuotaSnapshot>>,
    last_live_activity_fingerprint: Mutex<Option<String>>,
    pending_live_activity_fingerprint: Mutex<Option<String>>,
}

impl QuotaSnapshotState {
    fn set(&self, snapshot: QuotaSnapshot) {
        if let Ok(mut current) = self.current.lock() {
            *current = Some(snapshot);
        }
    }

    fn get(&self) -> Option<QuotaSnapshot> {
        self.current.lock().ok().and_then(|current| current.clone())
    }

    fn claim_live_activity_fingerprint_to_send(&self, snapshot: &QuotaSnapshot) -> Option<String> {
        if !should_publish_quota_activity_snapshot(snapshot) {
            return None;
        }

        let fingerprint = quota_activity_fingerprint(snapshot);
        let Ok(last_fingerprint) = self.last_live_activity_fingerprint.lock() else {
            return Some(fingerprint);
        };
        if last_fingerprint.as_deref() == Some(fingerprint.as_str()) {
            return None;
        }
        if let Ok(mut pending_fingerprint) = self.pending_live_activity_fingerprint.lock() {
            if pending_fingerprint.as_deref() == Some(fingerprint.as_str()) {
                return None;
            }
            *pending_fingerprint = Some(fingerprint.clone());
        }
        Some(fingerprint)
    }

    fn complete_live_activity_fingerprint_send(&self, fingerprint: String, sent: bool) {
        if let Ok(mut pending_fingerprint) = self.pending_live_activity_fingerprint.lock() {
            if pending_fingerprint.as_deref() == Some(fingerprint.as_str()) {
                *pending_fingerprint = None;
            }
        }
        if !sent {
            return;
        }
        let is_current_fingerprint = self
            .current
            .lock()
            .ok()
            .and_then(|current| current.as_ref().map(quota_activity_fingerprint))
            .as_deref()
            == Some(fingerprint.as_str());
        if !is_current_fingerprint {
            return;
        }
        let Ok(mut last_fingerprint) = self.last_live_activity_fingerprint.lock() else {
            return;
        };
        *last_fingerprint = Some(fingerprint);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub status: String,
    pub status_label: String,
    pub providers: Vec<UsageProvider>,
    pub primary: Option<QuotaSnapshotMetric>,
    pub secondary: Option<QuotaSnapshotMetric>,
    pub updated_at_ms: i64,
    pub stale_after_ms: i64,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshotMetric {
    pub provider_id: String,
    pub provider_name: String,
    pub provider_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_label: Option<String>,
    pub label: String,
    pub remaining: u8,
    pub reset_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at_ms: Option<i64>,
}

pub async fn refresh_quota_snapshot_for_app(
    app: Option<&AppHandle>,
    codex_home: Option<&str>,
) -> Option<QuotaSnapshot> {
    let usage_config = app
        .and_then(|app| {
            app.try_state::<AppState>().and_then(|state| {
                state
                    .config
                    .lock()
                    .ok()
                    .map(|config| config.usage_config.clone())
            })
        })
        .unwrap_or_else(|| {
            log::warn!("[QuotaSnapshot] AppState unavailable; using default usage config");
            default_usage_config()
        });

    let snapshot_state = app.and_then(|app| app.try_state::<QuotaSnapshotState>());
    let previous_snapshot = snapshot_state
        .as_ref()
        .and_then(|state| state.get())
        .or_else(|| BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE.get());
    let snapshot = match get_usage_quota_providers_from_config(usage_config, codex_home).await {
        Ok(providers) => build_quota_snapshot(providers, "quota_snapshot_refresh"),
        Err(error) => {
            build_quota_error_snapshot(previous_snapshot, error, "quota_snapshot_refresh_error")
        }
    };

    if let Some(state) = snapshot_state {
        state.set(snapshot.clone());
        if let Some(fingerprint) = state.claim_live_activity_fingerprint_to_send(&snapshot) {
            schedule_quota_snapshot_live_activity_apns(app.cloned(), snapshot.clone(), fingerprint);
        }
    } else {
        BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE.set(snapshot.clone());
        if let Some(fingerprint) =
            BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE.claim_live_activity_fingerprint_to_send(&snapshot)
        {
            schedule_quota_snapshot_live_activity_apns(None, snapshot.clone(), fingerprint);
        }
    }
    Some(snapshot)
}

pub fn current_quota_snapshot_for_app(app: Option<&AppHandle>) -> Option<QuotaSnapshot> {
    app.and_then(|app| app.try_state::<QuotaSnapshotState>())
        .and_then(|state| state.get())
        .or_else(|| BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE.get())
}

pub async fn refresh_and_inject_quota_snapshot_in_mcp_state(
    app: Option<&AppHandle>,
    payload: &mut Value,
    codex_home: Option<&str>,
) {
    let snapshot = refresh_quota_snapshot_for_app(app, codex_home)
        .await
        .or_else(|| current_quota_snapshot_for_app(app));
    if let Some(snapshot) = snapshot.as_ref() {
        inject_quota_snapshot_in_mcp_state(payload, snapshot);
    }
}

pub fn inject_current_quota_snapshot_in_mcp_state(app: Option<&AppHandle>, payload: &mut Value) {
    if let Some(snapshot) = current_quota_snapshot_for_app(app) {
        inject_quota_snapshot_in_mcp_state(payload, &snapshot);
    }
}

pub fn inject_quota_snapshot_in_mcp_state(payload: &mut Value, snapshot: &QuotaSnapshot) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };

    object.insert(
        "quotaSnapshot".to_string(),
        serde_json::to_value(snapshot).unwrap_or(Value::Null),
    );
    object.insert(
        "quotaProviders".to_string(),
        serde_json::to_value(&snapshot.providers).unwrap_or(Value::Array(Vec::new())),
    );
    object.insert(
        "quotaStatusLabel".to_string(),
        Value::String(snapshot.status_label.clone()),
    );
}

pub fn codex_home_from_mcp_state(payload: &Value) -> Option<String> {
    payload
        .get("request")
        .and_then(|request| {
            request
                .get("codex_home")
                .or_else(|| request.get("codexHome"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn build_quota_snapshot(providers: Vec<UsageProvider>, source: &str) -> QuotaSnapshot {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let metrics = quota_snapshot_metrics(&providers);
    let primary = metrics.first().cloned();
    let secondary = metrics
        .iter()
        .skip(1)
        .find(|metric| {
            primary.as_ref().map_or(true, |primary| {
                metric.provider_id != primary.provider_id || metric.label != primary.label
            })
        })
        .cloned();
    let status = quota_status(primary.as_ref(), providers.is_empty());
    let status_label = quota_status_label(status.as_str(), primary.as_ref());

    QuotaSnapshot {
        status,
        status_label,
        providers,
        primary,
        secondary,
        updated_at_ms: now_ms,
        stale_after_ms: now_ms.saturating_add(QUOTA_SNAPSHOT_STALE_AFTER_MS),
        source: source.to_string(),
        error: None,
    }
}

fn build_quota_error_snapshot(
    previous: Option<QuotaSnapshot>,
    error: String,
    source: &str,
) -> QuotaSnapshot {
    let now_ms = chrono::Utc::now().timestamp_millis();
    if let Some(mut snapshot) = previous {
        snapshot.status = "stale".to_string();
        snapshot.status_label = "额度离线".to_string();
        snapshot.updated_at_ms = now_ms;
        snapshot.stale_after_ms = now_ms;
        snapshot.source = source.to_string();
        snapshot.error = Some(error);
        return snapshot;
    }

    QuotaSnapshot {
        status: "unknown".to_string(),
        status_label: "额度未知".to_string(),
        providers: Vec::new(),
        primary: None,
        secondary: None,
        updated_at_ms: now_ms,
        stale_after_ms: now_ms,
        source: source.to_string(),
        error: Some(error),
    }
}

fn quota_snapshot_metrics(providers: &[UsageProvider]) -> Vec<QuotaSnapshotMetric> {
    let mut metrics = providers
        .iter()
        .flat_map(|provider| {
            provider
                .metrics
                .iter()
                .map(move |metric| QuotaSnapshotMetric {
                    provider_id: provider.id.clone(),
                    provider_name: provider.name.clone(),
                    provider_summary: provider.summary.clone(),
                    account_label: provider.account_label.clone(),
                    label: metric.label.clone(),
                    remaining: metric.remaining,
                    reset_label: metric.reset_label.clone(),
                    reset_at_ms: metric.reset_at_ms,
                })
        })
        .collect::<Vec<_>>();

    metrics.sort_by(|left, right| {
        left.remaining
            .cmp(&right.remaining)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
            .then_with(|| left.label.cmp(&right.label))
    });
    metrics
}

fn quota_status(primary: Option<&QuotaSnapshotMetric>, no_providers: bool) -> String {
    let Some(primary) = primary else {
        return if no_providers { "unknown" } else { "ok" }.to_string();
    };

    if primary.remaining == 0 {
        "limited".to_string()
    } else if primary.remaining <= QUOTA_LOW_THRESHOLD_PERCENT {
        "low".to_string()
    } else {
        "ok".to_string()
    }
}

fn quota_status_label(status: &str, primary: Option<&QuotaSnapshotMetric>) -> String {
    match (status, primary) {
        ("limited", Some(metric)) => metric
            .reset_label
            .as_ref()
            .map(|reset| format!("受限 · {}", reset))
            .unwrap_or_else(|| "受限".to_string()),
        ("low", Some(metric)) => metric
            .reset_label
            .as_ref()
            .map(|reset| format!("偏低 · {}% · {}", metric.remaining, reset))
            .unwrap_or_else(|| format!("偏低 · {}%", metric.remaining)),
        ("ok", Some(metric)) => format!("实时 · {}%", metric.remaining),
        ("stale", _) => "额度离线".to_string(),
        _ => "额度未知".to_string(),
    }
}

fn should_publish_quota_activity_snapshot(snapshot: &QuotaSnapshot) -> bool {
    snapshot.primary.is_some()
}

fn quota_activity_fingerprint(snapshot: &QuotaSnapshot) -> String {
    let primary = snapshot
        .primary
        .as_ref()
        .map(quota_activity_metric_fingerprint)
        .unwrap_or_else(|| "primary:none".to_string());
    let secondary = snapshot
        .secondary
        .as_ref()
        .map(quota_activity_metric_fingerprint)
        .unwrap_or_else(|| "secondary:none".to_string());

    format!(
        "{}|{}|{}|{}",
        snapshot.status, snapshot.status_label, primary, secondary
    )
}

fn quota_activity_metric_fingerprint(metric: &QuotaSnapshotMetric) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{:?}",
        metric.provider_id,
        metric.provider_summary,
        metric.account_label.as_deref().unwrap_or(""),
        metric.label,
        metric.remaining,
        metric.reset_label.as_deref().unwrap_or(""),
        metric.reset_at_ms
    )
}

fn schedule_quota_snapshot_live_activity_apns(
    app: Option<AppHandle>,
    snapshot: QuotaSnapshot,
    fingerprint: String,
) {
    tokio::spawn(async move {
        let sent = send_quota_snapshot_live_activity_apns(snapshot).await;
        if let Some(state) = app
            .as_ref()
            .and_then(|app| app.try_state::<QuotaSnapshotState>())
        {
            state.complete_live_activity_fingerprint_send(fingerprint, sent);
        } else {
            BRIDGE_ONLY_QUOTA_SNAPSHOT_STATE
                .complete_live_activity_fingerprint_send(fingerprint, sent);
        }
    });
}

async fn send_quota_snapshot_live_activity_apns(snapshot: QuotaSnapshot) -> bool {
    let event = if snapshot.status == "ok" {
        "end"
    } else {
        "update"
    };
    let Ok(snapshot_value) = serde_json::to_value(&snapshot) else {
        return false;
    };
    crate::bridge::ws::send_quota_live_activity_apns(snapshot_value, event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::quota::QuotaMetric;

    fn provider(metrics: Vec<QuotaMetric>) -> UsageProvider {
        UsageProvider {
            id: "codex".to_string(),
            name: "Codex".to_string(),
            account_label: Some("user@example.com".to_string()),
            color: "#111111".to_string(),
            icon_url: None,
            summary: "Pro OAuth".to_string(),
            updated_at: None,
            metrics,
        }
    }

    #[test]
    fn zero_remaining_is_limited_not_infinite() {
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );

        assert_eq!(snapshot.status, "limited");
        assert_eq!(snapshot.status_label, "受限 · 14:01 重置");
        assert_eq!(
            snapshot.primary.as_ref().unwrap().provider_summary,
            "Pro OAuth"
        );
        assert_eq!(
            snapshot.primary.as_ref().unwrap().reset_at_ms,
            Some(1_780_560_060_000)
        );
    }

    #[test]
    fn low_remaining_is_low() {
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 8,
                reset_label: None,
                reset_at_ms: None,
            }])],
            "test",
        );

        assert_eq!(snapshot.status, "low");
        assert_eq!(snapshot.status_label, "偏低 · 8%");
    }

    #[test]
    fn account_label_changes_quota_activity_fingerprint() {
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );
        let mut changed = snapshot.clone();
        changed.primary.as_mut().unwrap().account_label = Some("other@example.com".to_string());

        assert_ne!(
            quota_activity_fingerprint(&snapshot),
            quota_activity_fingerprint(&changed)
        );
    }

    #[test]
    fn provider_summary_changes_quota_activity_fingerprint() {
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );
        let mut changed = snapshot.clone();
        changed.primary.as_mut().unwrap().provider_summary = "Plus OAuth".to_string();

        assert_ne!(
            quota_activity_fingerprint(&snapshot),
            quota_activity_fingerprint(&changed)
        );
    }

    #[test]
    fn rolling_timestamps_do_not_change_quota_activity_fingerprint() {
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );
        let mut changed = snapshot.clone();
        changed.updated_at_ms = changed.updated_at_ms.saturating_add(60_000);
        changed.stale_after_ms = changed.stale_after_ms.saturating_add(60_000);

        assert_eq!(
            quota_activity_fingerprint(&snapshot),
            quota_activity_fingerprint(&changed)
        );
    }

    #[test]
    fn unsent_quota_activity_fingerprint_remains_retryable() {
        let state = QuotaSnapshotState::default();
        let snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );
        state.set(snapshot.clone());

        let fingerprint = state
            .claim_live_activity_fingerprint_to_send(&snapshot)
            .expect("first semantic snapshot should be eligible");
        assert!(
            state
                .claim_live_activity_fingerprint_to_send(&snapshot)
                .is_none(),
            "in-flight APNs sends should not enqueue duplicate fingerprints"
        );

        state.complete_live_activity_fingerprint_send(fingerprint.clone(), false);
        assert!(
            state
                .claim_live_activity_fingerprint_to_send(&snapshot)
                .is_some(),
            "failed or skipped APNs sends must become retryable again"
        );

        state.complete_live_activity_fingerprint_send(fingerprint, true);
        assert!(
            state
                .claim_live_activity_fingerprint_to_send(&snapshot)
                .is_none(),
            "successfully sent fingerprints should be deduped"
        );
    }

    #[test]
    fn stale_quota_activity_send_completion_does_not_mark_current_fingerprint() {
        let state = QuotaSnapshotState::default();
        let old_snapshot = build_quota_snapshot(
            vec![provider(vec![QuotaMetric {
                label: "Session".to_string(),
                remaining: 0,
                reset_label: Some("14:01 重置".to_string()),
                reset_at_ms: Some(1_780_560_060_000),
            }])],
            "test",
        );
        let mut new_snapshot = old_snapshot.clone();
        new_snapshot.primary.as_mut().unwrap().remaining = 8;
        new_snapshot.status = "low".to_string();
        new_snapshot.status_label = "偏低 · 8% · 14:01 重置".to_string();

        state.set(old_snapshot.clone());
        let old_fingerprint = state
            .claim_live_activity_fingerprint_to_send(&old_snapshot)
            .expect("old snapshot should be eligible");
        state.set(new_snapshot.clone());
        let new_fingerprint = state
            .claim_live_activity_fingerprint_to_send(&new_snapshot)
            .expect("new snapshot should be independently eligible");

        state.complete_live_activity_fingerprint_send(old_fingerprint, true);
        state.complete_live_activity_fingerprint_send(new_fingerprint, false);
        assert!(
            state
                .claim_live_activity_fingerprint_to_send(&new_snapshot)
                .is_some(),
            "a stale successful completion must not mark the current fingerprint sent"
        );
    }
}
