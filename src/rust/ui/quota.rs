use super::codex_quota::{
    get_native_codex_usage_provider, get_native_codex_usage_provider_with_options,
    CodexUsageProviderOptions,
};
use crate::config::{AppState, UsageConfig, UsageProviderAccountConfig, UsageProviderConfig};
use chrono::{DateTime, Local};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::State;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaMetric {
    pub label: String,
    pub remaining: u8,
    pub reset_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageProvider {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_label: Option<String>,
    pub color: String,
    pub icon_url: Option<String>,
    pub summary: String,
    pub updated_at: Option<String>,
    pub metrics: Vec<QuotaMetric>,
}

#[derive(Clone)]
struct AntigravityEndpoint {
    scheme: &'static str,
    port: u16,
    csrf_token: String,
}

struct AntigravityProcessInfo {
    pid: u32,
    csrf_token: String,
    extension_port: Option<u16>,
    extension_csrf_token: Option<String>,
}

struct AntigravityModelQuota {
    label: String,
    model_id: String,
    remaining_fraction: Option<f64>,
    reset_time: Option<DateTime<Local>>,
}

struct AntigravitySnapshot {
    model_quotas: Vec<AntigravityModelQuota>,
    account_plan: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityUserStatusResponse {
    code: Option<AntigravityCodeValue>,
    user_status: Option<AntigravityUserStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityCommandModelResponse {
    code: Option<AntigravityCodeValue>,
    client_model_configs: Option<Vec<AntigravityModelConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityUserStatus {
    plan_status: Option<AntigravityPlanStatus>,
    cascade_model_config_data: Option<AntigravityModelConfigData>,
    user_tier: Option<AntigravityUserTier>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityUserTier {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityPlanStatus {
    plan_info: Option<AntigravityPlanInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityPlanInfo {
    plan_name: Option<String>,
    plan_display_name: Option<String>,
    display_name: Option<String>,
    product_name: Option<String>,
    plan_short_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityModelConfigData {
    client_model_configs: Option<Vec<AntigravityModelConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityModelConfig {
    label: Option<String>,
    model_or_alias: Option<AntigravityModelAlias>,
    quota_info: Option<AntigravityQuotaInfo>,
}

#[derive(Debug, Deserialize)]
struct AntigravityModelAlias {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AntigravityQuotaInfo {
    remaining_fraction: Option<f64>,
    reset_time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AntigravityCodeValue {
    Int(i64),
    String(String),
}

fn usage_provider_config<'a>(
    usage_config: &'a UsageConfig,
    provider_id: &str,
) -> Option<&'a UsageProviderConfig> {
    if !usage_config.enabled {
        return None;
    }
    usage_config
        .providers
        .get(provider_id)
        .filter(|config| config.enabled)
}

async fn get_native_codex_usage_providers() -> Result<Vec<UsageProvider>, String> {
    get_native_codex_usage_provider()
        .await
        .map(|provider| vec![provider])
}

async fn get_native_codex_usage_provider_for_account(
    account: &UsageProviderAccountConfig,
    index: usize,
) -> Result<UsageProvider, String> {
    get_native_codex_usage_provider_with_options(CodexUsageProviderOptions {
        provider_id: Some(codex_account_provider_id(account, index)),
        summary_label: account
            .label
            .as_ref()
            .and_then(|value| non_empty_string(value)),
        codex_home: account.codex_home.as_deref().and_then(expand_codex_home),
    })
    .await
}

async fn get_auto_codex_usage_providers() -> Result<Vec<UsageProvider>, String> {
    get_native_codex_usage_providers()
        .await
        .map_err(|error| format!("无法获取 Codex 额度：原生 OAuth 失败：{}", error))
}

async fn get_codex_usage_providers_for_source(source: &str) -> Result<Vec<UsageProvider>, String> {
    match source {
        "" | "auto" => get_auto_codex_usage_providers().await,
        "oauth" | "native" | "native-oauth" => get_native_codex_usage_providers().await,
        other => Err(format!("Codex 暂不支持 source={}", other)),
    }
}

async fn get_codex_usage_provider_for_account_source(
    account: &UsageProviderAccountConfig,
    index: usize,
    source: &str,
) -> Result<UsageProvider, String> {
    match source {
        "" | "auto" | "oauth" | "native" | "native-oauth" => {
            get_native_codex_usage_provider_for_account(account, index).await
        }
        other => Err(format!(
            "Codex 账号 {} 暂不支持 source={}",
            codex_account_error_label(account, index),
            other
        )),
    }
}

async fn get_codex_usage_providers_for_config(
    config: &UsageProviderConfig,
) -> Result<Vec<UsageProvider>, String> {
    let accounts = codex_accounts_for_config(config);
    if accounts.is_empty() {
        return get_codex_usage_providers_for_source(config.source.as_str()).await;
    }

    let mut providers = Vec::new();
    let mut errors = Vec::new();
    let mut enabled_accounts = 0usize;

    for (index, account) in accounts.iter().enumerate() {
        if !account.enabled {
            continue;
        }
        enabled_accounts += 1;
        let source = account
            .source
            .as_deref()
            .and_then(non_empty_string)
            .unwrap_or_else(|| config.source.clone());
        match get_codex_usage_provider_for_account_source(account, index, source.as_str()).await {
            Ok(provider) => providers.push(provider),
            Err(error) => errors.push(error),
        }
    }

    if providers.is_empty() {
        if enabled_accounts == 0 {
            return Err("Codex accounts 没有启用项".to_string());
        }
        if !errors.is_empty() {
            return Err(errors.join("；"));
        }
    }

    Ok(providers)
}

async fn get_codex_usage_providers_for_request(
    config: &UsageProviderConfig,
    codex_home: Option<&str>,
) -> Result<Vec<UsageProvider>, String> {
    let Some(codex_home) = codex_home.and_then(non_empty_string) else {
        return get_codex_usage_providers_for_config(config).await;
    };

    let codex_home = allowed_request_codex_home(codex_home.as_str())?;
    let account = UsageProviderAccountConfig {
        id: "current".to_string(),
        enabled: true,
        label: None,
        codex_home: Some(codex_home.to_string_lossy().to_string()),
        source: None,
    };
    let provider =
        get_codex_usage_provider_for_account_source(&account, 0, config.source.as_str()).await?;
    Ok(vec![provider])
}

fn allowed_request_codex_home(value: &str) -> Result<PathBuf, String> {
    let codex_home =
        expand_codex_home(value).ok_or_else(|| "Codex home override 为空".to_string())?;
    let home_dir = dirs::home_dir().ok_or_else(|| "无法解析用户 home 目录".to_string())?;
    let file_name = codex_home
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    let allowed_name = file_name == ".codex" || file_name.starts_with(".codex-");
    if !codex_home.starts_with(&home_dir) || !allowed_name {
        return Err(format!(
            "Codex home override 不允许：{}",
            codex_home.display()
        ));
    }

    Ok(codex_home)
}

fn codex_accounts_for_config(config: &UsageProviderConfig) -> Vec<UsageProviderAccountConfig> {
    let mut accounts = config.accounts.clone();
    if !config.auto_discover_accounts {
        return accounts;
    }

    let mut keys = accounts
        .iter()
        .filter_map(codex_account_home_key)
        .collect::<BTreeSet<_>>();
    for discovered in discover_codex_accounts() {
        if let Some(key) = codex_account_home_key(&discovered) {
            if !keys.insert(key) {
                continue;
            }
        }
        accounts.push(discovered);
    }
    accounts
}

fn discover_codex_accounts() -> Vec<UsageProviderAccountConfig> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let mut accounts = Vec::new();
    let default_home = home.join(".codex");
    if default_home.join("auth.json").is_file() {
        accounts.push(discovered_codex_account(
            "default",
            "Default",
            "~/.codex".to_string(),
        ));
    }

    let Ok(entries) = std::fs::read_dir(home.as_path()) else {
        return accounts;
    };
    let mut discovered = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() || !path.join("auth.json").is_file() {
                return None;
            }
            let name = entry.file_name();
            let name = name.to_str()?;
            let suffix = name.strip_prefix(".codex-")?;
            let id = sanitize_codex_account_id(suffix);
            let label = label_from_codex_home_suffix(suffix);
            Some(discovered_codex_account(
                id.as_str(),
                label.as_str(),
                format!("~/{}", name),
            ))
        })
        .collect::<Vec<_>>();
    discovered.sort_by(|left, right| left.id.cmp(&right.id));
    accounts.append(&mut discovered);
    accounts
}

fn discovered_codex_account(
    id: &str,
    label: &str,
    codex_home: String,
) -> UsageProviderAccountConfig {
    UsageProviderAccountConfig {
        id: id.to_string(),
        enabled: true,
        label: Some(label.to_string()),
        codex_home: Some(codex_home),
        source: None,
    }
}

fn codex_account_home_key(account: &UsageProviderAccountConfig) -> Option<String> {
    account
        .codex_home
        .as_deref()
        .and_then(expand_codex_home)
        .map(|path| path.to_string_lossy().to_string())
}

fn sanitize_codex_account_id(value: &str) -> String {
    let id = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if id.is_empty() {
        "account".to_string()
    } else {
        id
    }
}

fn label_from_codex_home_suffix(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn codex_account_provider_id(account: &UsageProviderAccountConfig, index: usize) -> String {
    match non_empty_string(account.id.as_str()) {
        Some(id) => format!("codex:{}", id),
        None => format!("codex:account-{}", index + 1),
    }
}

fn codex_account_error_label(account: &UsageProviderAccountConfig, index: usize) -> String {
    account
        .label
        .as_deref()
        .and_then(non_empty_string)
        .or_else(|| non_empty_string(account.id.as_str()))
        .unwrap_or_else(|| format!("account-{}", index + 1))
}

fn expand_codex_home(value: &str) -> Option<PathBuf> {
    let value = non_empty_string(value)?;
    if value == "~" {
        return dirs::home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(rest));
    }
    Some(PathBuf::from(value))
}

fn clamp_percent(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 100.0)
}

fn format_local_reset_label(value: Option<DateTime<Local>>) -> Option<String> {
    Some(format!("{} 重置", value?.format("%Y年%-m月%-d日 %-H:%M")))
}

fn parse_local_reset_time(value: Option<&str>) -> Option<DateTime<Local>> {
    let value = value?;
    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Some(date.with_timezone(&Local));
    }
    if let Ok(seconds) = value.parse::<f64>() {
        return DateTime::from_timestamp(seconds as i64, 0).map(|date| date.with_timezone(&Local));
    }
    None
}

fn antigravity_code_is_ok(code: Option<&AntigravityCodeValue>) -> bool {
    match code {
        None => true,
        Some(AntigravityCodeValue::Int(value)) => *value == 0,
        Some(AntigravityCodeValue::String(value)) => {
            let lower = value.to_ascii_lowercase();
            lower == "ok" || lower == "success" || lower == "0"
        }
    }
}

fn preferred_antigravity_plan_name(
    tier: Option<&AntigravityUserTier>,
    plan: Option<&AntigravityPlanInfo>,
) -> Option<String> {
    if let Some(value) = tier
        .and_then(|tier| tier.name.as_deref())
        .and_then(non_empty_string)
    {
        return Some(value);
    }
    let candidates = [
        plan.and_then(|plan| plan.plan_display_name.as_deref()),
        plan.and_then(|plan| plan.display_name.as_deref()),
        plan.and_then(|plan| plan.product_name.as_deref()),
        plan.and_then(|plan| plan.plan_name.as_deref()),
        plan.and_then(|plan| plan.plan_short_name.as_deref()),
    ];
    candidates.into_iter().flatten().find_map(non_empty_string)
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn antigravity_quota_from_config(config: AntigravityModelConfig) -> Option<AntigravityModelQuota> {
    let quota = config.quota_info?;
    Some(AntigravityModelQuota {
        label: config.label?,
        model_id: config.model_or_alias?.model?,
        remaining_fraction: quota.remaining_fraction,
        reset_time: parse_local_reset_time(quota.reset_time.as_deref()),
    })
}

fn parse_antigravity_user_status(data: &[u8]) -> Result<AntigravitySnapshot, String> {
    let response: AntigravityUserStatusResponse = serde_json::from_slice(data)
        .map_err(|e| format!("解析 Antigravity GetUserStatus 失败: {}", e))?;
    if !antigravity_code_is_ok(response.code.as_ref()) {
        return Err(format!("Antigravity API 返回异常 code={:?}", response.code));
    }
    let user_status = response
        .user_status
        .ok_or_else(|| "Antigravity GetUserStatus 缺少 userStatus".to_string())?;
    let model_quotas = user_status
        .cascade_model_config_data
        .and_then(|data| data.client_model_configs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(antigravity_quota_from_config)
        .collect();
    let plan_info = user_status
        .plan_status
        .as_ref()
        .and_then(|status| status.plan_info.as_ref());
    Ok(AntigravitySnapshot {
        model_quotas,
        account_plan: preferred_antigravity_plan_name(user_status.user_tier.as_ref(), plan_info),
    })
}

fn parse_antigravity_command_models(data: &[u8]) -> Result<AntigravitySnapshot, String> {
    let response: AntigravityCommandModelResponse = serde_json::from_slice(data)
        .map_err(|e| format!("解析 Antigravity GetCommandModelConfigs 失败: {}", e))?;
    if !antigravity_code_is_ok(response.code.as_ref()) {
        return Err(format!("Antigravity API 返回异常 code={:?}", response.code));
    }
    let model_quotas = response
        .client_model_configs
        .unwrap_or_default()
        .into_iter()
        .filter_map(antigravity_quota_from_config)
        .collect();
    Ok(AntigravitySnapshot {
        model_quotas,
        account_plan: None,
    })
}

async fn run_command_output(binary: &str, args: &[&str], seconds: u64) -> Result<String, String> {
    let output = timeout(
        Duration::from_secs(seconds),
        Command::new(binary).args(args).output(),
    )
    .await
    .map_err(|_| format!("{} 执行超时", binary))?
    .map_err(|e| format!("无法执行 {}: {}", binary, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{} 退出码: {}", binary, output.status)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn is_antigravity_command_line(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("language_server_macos")
        && ((lower.contains("--app_data_dir") && lower.contains("antigravity"))
            || lower.contains("/antigravity/")
            || lower.contains("\\antigravity\\"))
}

fn extract_antigravity_flag(flag: &str, command: &str) -> Option<String> {
    let pattern = format!(r"{}[=\s]+([^\s]+)", regex::escape(flag));
    let regex = Regex::new(&pattern).ok()?;
    regex
        .captures(command)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

async fn detect_antigravity_process() -> Result<AntigravityProcessInfo, String> {
    let output = run_command_output("/bin/ps", &["-ax", "-o", "pid=,command="], 4).await?;
    let mut saw_antigravity = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((pid, command)) = trimmed.split_once(char::is_whitespace) else {
            continue;
        };
        if !is_antigravity_command_line(command) {
            continue;
        }
        saw_antigravity = true;
        let csrf_token = match extract_antigravity_flag("--csrf_token", command) {
            Some(value) => value,
            None => continue,
        };
        return Ok(AntigravityProcessInfo {
            pid: pid
                .trim()
                .parse()
                .map_err(|_| "Antigravity 进程 PID 解析失败".to_string())?,
            csrf_token,
            extension_port: extract_antigravity_flag("--extension_server_port", command)
                .and_then(|value| value.parse::<u16>().ok()),
            extension_csrf_token: extract_antigravity_flag(
                "--extension_server_csrf_token",
                command,
            ),
        });
    }

    if saw_antigravity {
        Err("Antigravity language server 缺少 CSRF token，请重启 Antigravity 后重试".to_string())
    } else {
        Err("Antigravity language server 未检测到，请先启动 Antigravity".to_string())
    }
}

fn parse_antigravity_ports(output: &str) -> Vec<u16> {
    let regex = match Regex::new(r":(\d+)\s+\(LISTEN\)") {
        Ok(regex) => regex,
        Err(_) => return Vec::new(),
    };
    regex
        .captures_iter(output)
        .filter_map(|captures| captures.get(1))
        .filter_map(|value| value.as_str().parse::<u16>().ok())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

async fn antigravity_listening_ports(pid: u32) -> Result<Vec<u16>, String> {
    let lsof = ["/usr/sbin/lsof", "/usr/bin/lsof"]
        .into_iter()
        .find(|path| Path::new(path).exists())
        .ok_or_else(|| "Antigravity 端口检测需要 lsof".to_string())?;
    let pid_string = pid.to_string();
    let output = run_command_output(
        lsof,
        &[
            "-nP",
            "-iTCP",
            "-sTCP:LISTEN",
            "-a",
            "-p",
            pid_string.as_str(),
        ],
        4,
    )
    .await?;
    let ports = parse_antigravity_ports(output.as_str());
    if ports.is_empty() {
        Err("Antigravity 正在运行但尚未暴露监听端口，请稍后重试".to_string())
    } else {
        Ok(ports)
    }
}

fn antigravity_endpoints(info: &AntigravityProcessInfo, ports: &[u16]) -> Vec<AntigravityEndpoint> {
    let mut endpoints = Vec::new();
    for port in ports {
        endpoints.push(AntigravityEndpoint {
            scheme: "https",
            port: *port,
            csrf_token: info.csrf_token.clone(),
        });
    }
    if let Some(port) = info.extension_port {
        if let Some(token) = info.extension_csrf_token.as_ref() {
            endpoints.push(AntigravityEndpoint {
                scheme: "http",
                port,
                csrf_token: token.clone(),
            });
        }
        endpoints.push(AntigravityEndpoint {
            scheme: "http",
            port,
            csrf_token: info.csrf_token.clone(),
        });
    }
    endpoints
}

async fn antigravity_request(
    endpoint: &AntigravityEndpoint,
    path: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .danger_accept_invalid_certs(endpoint.scheme == "https")
        .build()
        .map_err(|e| format!("创建 Antigravity HTTP client 失败: {}", e))?;
    let body = serde_json::json!({
        "metadata": {
            "ideName": "antigravity",
            "extensionName": "antigravity",
            "ideVersion": "unknown",
            "locale": "en"
        }
    });
    let url = format!("{}://127.0.0.1:{}{}", endpoint.scheme, endpoint.port, path);
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", endpoint.csrf_token.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Antigravity 本地请求失败: {}", e))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取 Antigravity 响应失败: {}", e))?;
    if !status.is_success() {
        let message = String::from_utf8_lossy(&bytes);
        return Err(format!("Antigravity HTTP {}: {}", status, message));
    }
    Ok(bytes.to_vec())
}

async fn fetch_antigravity_snapshot() -> Result<AntigravitySnapshot, String> {
    const USER_STATUS: &str = "/exa.language_server_pb.LanguageServerService/GetUserStatus";
    const COMMAND_MODELS: &str =
        "/exa.language_server_pb.LanguageServerService/GetCommandModelConfigs";

    let process = detect_antigravity_process().await?;
    let ports = antigravity_listening_ports(process.pid).await?;
    let endpoints = antigravity_endpoints(&process, ports.as_slice());
    let mut last_error = None;

    for endpoint in &endpoints {
        match antigravity_request(endpoint, USER_STATUS)
            .await
            .and_then(|data| parse_antigravity_user_status(data.as_slice()))
        {
            Ok(snapshot) if !snapshot.model_quotas.is_empty() => return Ok(snapshot),
            Ok(_) => last_error = Some("Antigravity GetUserStatus 未返回 quota".to_string()),
            Err(error) => last_error = Some(error),
        }
    }

    for endpoint in &endpoints {
        match antigravity_request(endpoint, COMMAND_MODELS)
            .await
            .and_then(|data| parse_antigravity_command_models(data.as_slice()))
        {
            Ok(snapshot) if !snapshot.model_quotas.is_empty() => return Ok(snapshot),
            Ok(_) => {
                last_error = Some("Antigravity GetCommandModelConfigs 未返回 quota".to_string())
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| "Antigravity 未找到可用本地端点".to_string()))
}

fn antigravity_family(quota: &AntigravityModelQuota) -> &'static str {
    let text = format!("{} {}", quota.model_id, quota.label).to_ascii_lowercase();
    if text.contains("claude") {
        "Claude"
    } else if text.contains("gemini") && text.contains("pro") {
        "Gemini Pro"
    } else if text.contains("gemini") && text.contains("flash") {
        "Gemini Flash"
    } else {
        "Other"
    }
}

fn antigravity_representative<'a>(
    quotas: &'a [AntigravityModelQuota],
    family: &str,
) -> Option<&'a AntigravityModelQuota> {
    quotas
        .iter()
        .filter(|quota| antigravity_family(quota) == family)
        .min_by(|left, right| {
            let left_remaining = left.remaining_fraction.unwrap_or(1.0);
            let right_remaining = right.remaining_fraction.unwrap_or(1.0);
            left_remaining
                .partial_cmp(&right_remaining)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn antigravity_metric(label: &str, quota: &AntigravityModelQuota) -> QuotaMetric {
    let remaining = clamp_percent(quota.remaining_fraction.unwrap_or(0.0) * 100.0).round() as u8;
    QuotaMetric {
        label: label.to_string(),
        remaining,
        reset_label: format_local_reset_label(quota.reset_time),
        reset_at_ms: quota
            .reset_time
            .map(|reset_time| reset_time.timestamp_millis()),
    }
}

fn antigravity_snapshot_to_provider(snapshot: AntigravitySnapshot) -> UsageProvider {
    let mut metrics = Vec::new();
    for label in ["Claude", "Gemini Pro", "Gemini Flash"] {
        if let Some(quota) = antigravity_representative(snapshot.model_quotas.as_slice(), label) {
            metrics.push(antigravity_metric(label, quota));
        }
    }
    if metrics.is_empty() {
        if let Some(quota) = snapshot.model_quotas.iter().min_by(|left, right| {
            let left_remaining = left.remaining_fraction.unwrap_or(1.0);
            let right_remaining = right.remaining_fraction.unwrap_or(1.0);
            left_remaining
                .partial_cmp(&right_remaining)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            metrics.push(antigravity_metric(quota.label.as_str(), quota));
        }
    }

    UsageProvider {
        id: "antigravity".to_string(),
        name: "Antigravity".to_string(),
        account_label: None,
        color: "#60ba7e".to_string(),
        icon_url: Some("./icons/ai-providers/antigravity.svg".to_string()),
        summary: snapshot.account_plan.unwrap_or_else(|| "Local".to_string()),
        updated_at: Some(Local::now().format("%H:%M").to_string()),
        metrics,
    }
}

async fn get_antigravity_usage_provider(
    config: &UsageProviderConfig,
) -> Result<UsageProvider, String> {
    if config.source != "local" && config.source != "auto" {
        return Err(format!("Antigravity 暂不支持 source={}", config.source));
    }
    Ok(antigravity_snapshot_to_provider(
        fetch_antigravity_snapshot().await?,
    ))
}

#[tauri::command]
pub async fn get_codex_quota_providers() -> Result<Vec<UsageProvider>, String> {
    get_codex_usage_providers_for_source("auto").await
}

#[tauri::command]
pub async fn get_usage_quota_providers(
    state: State<'_, AppState>,
    codex_home: Option<String>,
) -> Result<Vec<UsageProvider>, String> {
    let usage_config = state
        .config
        .lock()
        .map_err(|e| format!("读取用量配置失败: {}", e))?
        .usage_config
        .clone();

    get_usage_quota_providers_from_config(usage_config, codex_home.as_deref()).await
}

pub async fn get_usage_quota_providers_from_config(
    usage_config: UsageConfig,
    codex_home: Option<&str>,
) -> Result<Vec<UsageProvider>, String> {
    let mut providers = Vec::new();
    let mut errors = Vec::new();

    if usage_provider_config(&usage_config, "codex").is_some() {
        let codex_config = usage_config
            .providers
            .get("codex")
            .expect("codex config checked above");
        match get_codex_usage_providers_for_request(codex_config, codex_home.as_deref()).await {
            Ok(mut codex_providers) => providers.append(&mut codex_providers),
            Err(error) => errors.push(error),
        }
    }

    if let Some(antigravity_config) = usage_provider_config(&usage_config, "antigravity") {
        match get_antigravity_usage_provider(antigravity_config).await {
            Ok(provider) => providers.push(provider),
            Err(error) => errors.push(format!("无法获取 Antigravity 额度：{}", error)),
        }
    }

    if providers.is_empty() && !errors.is_empty() {
        return Err(errors.join("；"));
    }

    Ok(providers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codex_account(id: &str) -> UsageProviderAccountConfig {
        UsageProviderAccountConfig {
            id: id.to_string(),
            enabled: true,
            label: None,
            codex_home: None,
            source: None,
        }
    }

    #[test]
    fn codex_account_provider_ids_are_namespaced() {
        let account = codex_account("plus");

        assert_eq!(codex_account_provider_id(&account, 0), "codex:plus");
    }

    #[test]
    fn codex_account_provider_id_falls_back_to_index() {
        let account = codex_account(" ");

        assert_eq!(codex_account_provider_id(&account, 1), "codex:account-2");
    }

    #[test]
    fn expands_tilde_codex_home() {
        let home = dirs::home_dir().expect("home dir");

        assert_eq!(expand_codex_home("~").as_deref(), Some(home.as_path()));
        assert_eq!(
            expand_codex_home("~/custom-codex").as_deref(),
            Some(home.join("custom-codex").as_path())
        );
    }

    #[test]
    fn allows_request_codex_home_under_codex_dirs() {
        let home = dirs::home_dir().expect("home dir");

        assert_eq!(
            allowed_request_codex_home("~/.codex-clone").as_deref(),
            Ok(home.join(".codex-clone").as_path())
        );
    }

    #[test]
    fn rejects_request_codex_home_outside_codex_dirs() {
        assert!(allowed_request_codex_home("~/Documents").is_err());
        assert!(allowed_request_codex_home("/tmp/.codex-clone").is_err());
    }

    #[test]
    fn formats_discovered_codex_account_labels() {
        assert_eq!(sanitize_codex_account_id("Plus_Main"), "plus_main");
        assert_eq!(sanitize_codex_account_id("plus@2"), "plus-2");
        assert_eq!(label_from_codex_home_suffix("plus-main"), "Plus Main");
    }

    #[test]
    fn codex_accounts_for_config_preserves_manual_list_when_discovery_disabled() {
        let account = UsageProviderAccountConfig {
            id: "plus".to_string(),
            enabled: true,
            label: Some("Plus".to_string()),
            codex_home: Some("~/.codex-plus".to_string()),
            source: None,
        };
        let config = UsageProviderConfig {
            enabled: true,
            source: "auto".to_string(),
            manual_cookie: None,
            accounts: vec![account],
            auto_discover_accounts: false,
        };

        let accounts = codex_accounts_for_config(&config);

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "plus");
    }
}
