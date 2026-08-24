pub(super) fn format_http_status(status: axum::http::StatusCode) -> String {
    match status.canonical_reason() {
        Some(reason) => format!("{} {}", status.as_u16(), reason),
        None => status.as_u16().to_string(),
    }
}

pub(super) fn http_url_to_ws_url(url: &str) -> Option<String> {
    if let Some(rest) = url.strip_prefix("https://") {
        Some(format!("wss://{rest}"))
    } else {
        url.strip_prefix("http://")
            .map(|rest| format!("ws://{rest}"))
    }
}

pub(super) fn probe_error_summary(probe: &serde_json::Value) -> Option<String> {
    probe
        .get("error_code")
        .and_then(|value| value.as_str())
        .or_else(|| probe.get("error").and_then(|value| value.as_str()))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn websocket_probe_auth_required(probe: &serde_json::Value) -> bool {
    matches!(
        probe.get("status_code").and_then(|value| value.as_u64()),
        Some(401 | 403)
    )
}

pub(super) fn websocket_probe_ok_or_auth_required(
    upgrade_ok: bool,
    probe: &serde_json::Value,
    require_mobile_auth: bool,
) -> bool {
    upgrade_ok || (require_mobile_auth && websocket_probe_auth_required(probe))
}
