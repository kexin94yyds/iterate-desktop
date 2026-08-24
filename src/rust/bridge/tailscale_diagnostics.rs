use super::public_control::public_bridge_base_url;

pub(super) const TAILSCALE_FUNNEL_PORT: &str = "443";

pub(super) fn trim_dns_dot(value: &str) -> String {
    value.trim().trim_end_matches('.').to_string()
}

pub(super) fn https_host_from_base_url(value: &str) -> Option<String> {
    let rest = value.trim().strip_prefix("https://")?;
    let host = rest.split('/').next()?.trim().trim_end_matches('.');
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

pub(super) fn tailscale_host_from_public_bridge_base_url() -> Option<String> {
    https_host_from_base_url(&public_bridge_base_url()).filter(|host| host.ends_with(".ts.net"))
}

pub(super) fn tailscale_dns_name(status_json: &serde_json::Value) -> Option<String> {
    status_json
        .pointer("/Self/DNSName")
        .and_then(|value| value.as_str())
        .map(trim_dns_dot)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            status_json
                .get("CertDomains")
                .and_then(|value| value.as_array())
                .and_then(|domains| domains.first())
                .and_then(|value| value.as_str())
                .map(trim_dns_dot)
                .filter(|value| !value.is_empty())
        })
}

pub(super) fn tailscale_status_summary(status_json: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "backend_state": status_json.get("BackendState").cloned().unwrap_or(serde_json::Value::Null),
        "self_online": status_json.pointer("/Self/Online").cloned().unwrap_or(serde_json::Value::Null),
        "dns_name": tailscale_dns_name(status_json),
        "tailscale_ips": status_json.pointer("/Self/TailscaleIPs").cloned().unwrap_or(serde_json::Value::Null),
        "health": status_json.get("Health").cloned().unwrap_or(serde_json::Value::Null),
    })
}

pub(super) fn normalize_tailscale_proxy_target(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

pub(super) fn tailscale_funnel_config_matches(
    funnel_status_json: &serde_json::Value,
    host: &str,
    target: &str,
) -> bool {
    let key = format!("{host}:{TAILSCALE_FUNNEL_PORT}");
    let allow_funnel = funnel_status_json
        .get("AllowFunnel")
        .and_then(|value| value.get(&key))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let proxy = funnel_status_json
        .get("Web")
        .and_then(|value| value.get(&key))
        .and_then(|value| value.get("Handlers"))
        .and_then(|value| value.get("/"))
        .and_then(|value| value.get("Proxy"))
        .and_then(|value| value.as_str())
        .map(normalize_tailscale_proxy_target);
    let target = normalize_tailscale_proxy_target(target);
    allow_funnel
        && matches!(
            proxy.as_deref(),
            Some(proxy) if proxy == target || proxy == "http://localhost:8080"
        )
}
