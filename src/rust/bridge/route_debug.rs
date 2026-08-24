use crate::conversation::resolve_tree_route_key;
use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::sync::RwLock;

static LAST_ACTIVE_ROUTE: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));
static ACTIVE_DESKTOP_POPUP_ROUTE: Lazy<RwLock<Option<RouteDebugSnapshot>>> =
    Lazy::new(|| RwLock::new(None));
static LAST_NOTIFICATION_ROUTE: Lazy<RwLock<Option<RouteDebugSnapshot>>> =
    Lazy::new(|| RwLock::new(None));
static LAST_COMPLETED_ROUTE: Lazy<RwLock<Option<RouteDebugSnapshot>>> =
    Lazy::new(|| RwLock::new(None));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct RouteDebugSnapshot {
    pub(super) route_key: String,
    pub(super) request_id: Option<String>,
    pub(super) project_path: Option<String>,
    pub(super) source: String,
    pub(super) updated_at: String,
}

pub(super) async fn last_active_route() -> Option<String> {
    LAST_ACTIVE_ROUTE.read().await.clone()
}

pub(super) async fn record_last_active_route(request_id: Option<&str>, project_path: Option<&str>) {
    if let Some(route_key) = resolve_tree_route_key(request_id, project_path) {
        *LAST_ACTIVE_ROUTE.write().await = Some(route_key);
    }
}

pub(super) fn build_route_debug_snapshot(
    request_id: Option<&str>,
    project_path: Option<&str>,
    source: &str,
) -> Option<RouteDebugSnapshot> {
    let route_key = resolve_tree_route_key(request_id, project_path)?;
    Some(RouteDebugSnapshot {
        route_key,
        request_id: request_id.map(ToOwned::to_owned),
        project_path: project_path.map(ToOwned::to_owned),
        source: source.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub(super) async fn record_last_notification_route(
    request_id: Option<&str>,
    project_path: Option<&str>,
    source: &str,
) {
    if let Some(snapshot) = build_route_debug_snapshot(request_id, project_path, source) {
        *LAST_NOTIFICATION_ROUTE.write().await = Some(snapshot);
    }
}

pub(super) async fn record_active_desktop_popup_route(
    request_id: Option<&str>,
    project_path: Option<&str>,
    source: &str,
) {
    if let Some(snapshot) = build_route_debug_snapshot(request_id, project_path, source) {
        *ACTIVE_DESKTOP_POPUP_ROUTE.write().await = Some(snapshot);
    }
}

pub(super) async fn clear_active_desktop_popup_route(
    request_id: Option<&str>,
    project_path: Option<&str>,
    source: &str,
) {
    let Some(route_key) = resolve_tree_route_key(request_id, project_path) else {
        return;
    };

    let mut active = ACTIVE_DESKTOP_POPUP_ROUTE.write().await;
    let should_clear = active
        .as_ref()
        .map(|snapshot| snapshot.route_key == route_key)
        .unwrap_or(false);

    if should_clear {
        log::debug!(
            "[Bridge Route] clear active desktop popup route: route_key={}, source={}",
            route_key,
            source
        );
        *active = None;
    }
}

pub(super) async fn record_last_completed_route(
    request_id: Option<&str>,
    project_path: Option<&str>,
    source: &str,
) {
    if let Some(snapshot) = build_route_debug_snapshot(request_id, project_path, source) {
        *LAST_COMPLETED_ROUTE.write().await = Some(snapshot);
    }
}

pub(super) async fn route_debug_status_value() -> serde_json::Value {
    let last_active_route = LAST_ACTIVE_ROUTE.read().await.clone();
    let active_desktop_popup_route = ACTIVE_DESKTOP_POPUP_ROUTE.read().await.clone();
    let last_notification_route = LAST_NOTIFICATION_ROUTE.read().await.clone();
    let last_completed_route = LAST_COMPLETED_ROUTE.read().await.clone();

    serde_json::json!({
        "last_active_route": last_active_route,
        "active_desktop_popup_route": active_desktop_popup_route,
        "last_notification_route": last_notification_route,
        "last_completed_route": last_completed_route,
    })
}

#[cfg(test)]
pub(super) async fn reset_active_desktop_popup_route_for_tests() {
    *ACTIVE_DESKTOP_POPUP_ROUTE.write().await = None;
}
