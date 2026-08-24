use jsonwebtoken::{Algorithm, EncodingKey, Header};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

const APNS_BEARER_REFRESH_SECS: i64 = 50 * 60;

#[derive(Debug, Clone)]
struct CachedApnsBearer {
    config_fingerprint: u64,
    issued_at: i64,
    token: String,
}

static APNS_BEARER_CACHE: Lazy<Mutex<Option<CachedApnsBearer>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ApnsEnvironment {
    Sandbox,
    Production,
}

impl ApnsEnvironment {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Production => "production",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ApnsConfig {
    pub(super) key_id: String,
    pub(super) team_id: String,
    pub(super) topic: String,
    pub(super) key_pem: String,
    pub(super) default_environment: ApnsEnvironment,
    pub(super) endpoint: String,
}

#[derive(Debug, Serialize)]
struct ApnsClaims {
    iss: String,
    iat: usize,
}

pub(super) fn load_apns_config() -> Option<ApnsConfig> {
    // macOS .app 通过 `open` 启动时不继承 shell 环境变量，
    // 所以先尝试环境变量，缺少必需项时让文件整体覆盖；必需项齐全时仍从文件补 APNS_ENV/APNS_TOPIC 等可选项。
    if apns_required_env_missing() {
        load_apns_env_from_file(true);
    } else {
        load_apns_env_from_file(false);
    }

    load_apns_config_from_env()
}

fn apns_required_env_missing() -> bool {
    std::env::var("APNS_KEY_ID").is_err()
        || std::env::var("APNS_TEAM_ID").is_err()
        || (std::env::var("APNS_AUTH_KEY_PATH").is_err()
            && std::env::var("APNS_AUTH_KEY_P8").is_err())
}

fn load_apns_config_from_env() -> Option<ApnsConfig> {
    let key_id = std::env::var("APNS_KEY_ID").ok()?;
    let team_id = std::env::var("APNS_TEAM_ID").ok()?;
    let topic = std::env::var("APNS_TOPIC").unwrap_or_else(|_| "com.iterate.notify".to_string());
    let key_pem = load_apns_key_pem_from_env()?;
    let default_environment = configured_apns_environment();
    let endpoint = apns_endpoint(default_environment).to_string();

    log::info!(
        "[APNs] 配置加载成功: key_id={}, team_id={}, topic={}, endpoint={}",
        key_id,
        team_id,
        topic,
        endpoint
    );

    Some(ApnsConfig {
        key_id,
        team_id,
        topic,
        key_pem,
        default_environment,
        endpoint,
    })
}

pub(super) fn parse_apns_environment(value: &str) -> Result<ApnsEnvironment, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "sandbox" | "development" => Ok(ApnsEnvironment::Sandbox),
        "production" => Ok(ApnsEnvironment::Production),
        _ => Err("environment must be sandbox or production".to_string()),
    }
}

pub(super) fn resolve_apns_environment(
    value: Option<&str>,
    default_environment: ApnsEnvironment,
) -> Result<ApnsEnvironment, String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => parse_apns_environment(value),
        None => Ok(default_environment),
    }
}

pub(super) fn configured_apns_environment() -> ApnsEnvironment {
    match std::env::var("APNS_ENV") {
        Ok(value) => match parse_apns_environment(&value) {
            Ok(environment) => environment,
            Err(_) => {
                log::warn!("[APNs] APNS_ENV={} 无效，沿用历史默认 production", value);
                ApnsEnvironment::Production
            }
        },
        Err(_) => ApnsEnvironment::Production,
    }
}

pub(super) fn apns_endpoint(environment: ApnsEnvironment) -> &'static str {
    match environment {
        ApnsEnvironment::Sandbox => "https://api.sandbox.push.apple.com",
        ApnsEnvironment::Production => "https://api.push.apple.com",
    }
}

fn load_apns_key_pem_from_env() -> Option<String> {
    if let Ok(path) = std::env::var("APNS_AUTH_KEY_PATH") {
        match std::fs::read_to_string(&path) {
            Ok(key_pem) => return Some(key_pem),
            Err(error) => {
                log::warn!("[APNs] 无法读取 APNS_AUTH_KEY_PATH={}: {}", path, error);
            }
        }
    }

    std::env::var("APNS_AUTH_KEY_P8").ok()
}

/// 从 ~/.config/iterate/apns-env.sh 解析 export KEY=VALUE 并设置到环境变量。
/// 当必需项已经来自 launchd 环境时，只补缺失项，避免文件覆盖显式运行时配置。
fn load_apns_env_from_file(overwrite_existing: bool) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::Path::new(&home).join(".config/iterate/apns-env.sh");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            log::debug!("[APNs] 配置文件不存在: {:?}", path);
            return;
        }
    };
    for line in content.lines() {
        let line = line.trim();
        if let Some(kv) = line.strip_prefix("export ") {
            if let Some((key, value)) = kv.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if overwrite_existing || std::env::var_os(key).is_none() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
    log::info!("[APNs] 从文件加载环境变量: {:?}", path);
}

pub(super) fn build_apns_bearer_token(config: &ApnsConfig) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp();
    let config_fingerprint = apns_bearer_config_fingerprint(config);
    let mut cache = APNS_BEARER_CACHE
        .lock()
        .map_err(|_| "APNs JWT 缓存锁不可用".to_string())?;
    if let Some(cached) = cache.as_ref() {
        if cached_apns_bearer_is_fresh(cached, config_fingerprint, now) {
            return Ok(cached.token.clone());
        }
    }

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(config.key_id.clone());

    let claims = ApnsClaims {
        iss: config.team_id.clone(),
        iat: now as usize,
    };

    let encoding_key = EncodingKey::from_ec_pem(config.key_pem.as_bytes())
        .map_err(|err| format!("APNs 私钥加载失败: {}", err))?;

    let token = jsonwebtoken::encode(&header, &claims, &encoding_key)
        .map_err(|err| format!("APNs JWT 生成失败: {}", err))?;

    *cache = Some(CachedApnsBearer {
        config_fingerprint,
        issued_at: now,
        token: token.clone(),
    });

    Ok(token)
}

fn apns_bearer_config_fingerprint(config: &ApnsConfig) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    config.key_id.hash(&mut hasher);
    config.team_id.hash(&mut hasher);
    config.key_pem.hash(&mut hasher);
    hasher.finish()
}

fn cached_apns_bearer_is_fresh(
    cached: &CachedApnsBearer,
    config_fingerprint: u64,
    now: i64,
) -> bool {
    cached.config_fingerprint == config_fingerprint
        && now >= cached.issued_at
        && now - cached.issued_at < APNS_BEARER_REFRESH_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached(issued_at: i64) -> CachedApnsBearer {
        CachedApnsBearer {
            config_fingerprint: 7,
            issued_at,
            token: "cached-token".to_string(),
        }
    }

    #[test]
    fn reuses_apns_bearer_until_safe_refresh_window() {
        assert!(cached_apns_bearer_is_fresh(
            &cached(1_000),
            7,
            1_000 + 49 * 60
        ));
        assert!(!cached_apns_bearer_is_fresh(
            &cached(1_000),
            7,
            1_000 + 50 * 60
        ));
    }

    #[test]
    fn rejects_cached_apns_bearer_for_changed_credentials_or_clock_rollback() {
        assert!(!cached_apns_bearer_is_fresh(&cached(1_000), 8, 1_100));
        assert!(!cached_apns_bearer_is_fresh(&cached(1_000), 7, 999));
    }

    #[test]
    fn normalizes_only_known_apns_environments() {
        assert_eq!(
            parse_apns_environment("sandbox"),
            Ok(ApnsEnvironment::Sandbox)
        );
        assert_eq!(
            parse_apns_environment("development"),
            Ok(ApnsEnvironment::Sandbox)
        );
        assert_eq!(
            parse_apns_environment("production"),
            Ok(ApnsEnvironment::Production)
        );
        assert!(parse_apns_environment("https://example.invalid").is_err());
    }

    #[test]
    fn maps_apns_environments_to_fixed_apple_endpoints() {
        assert_eq!(
            apns_endpoint(ApnsEnvironment::Sandbox),
            "https://api.sandbox.push.apple.com"
        );
        assert_eq!(
            apns_endpoint(ApnsEnvironment::Production),
            "https://api.push.apple.com"
        );
    }

    #[test]
    fn missing_registration_environment_uses_server_default() {
        assert_eq!(
            resolve_apns_environment(None, ApnsEnvironment::Sandbox),
            Ok(ApnsEnvironment::Sandbox)
        );
        assert_eq!(
            resolve_apns_environment(Some(""), ApnsEnvironment::Production),
            Ok(ApnsEnvironment::Production)
        );
        assert!(
            resolve_apns_environment(Some("custom-endpoint"), ApnsEnvironment::Production).is_err()
        );
    }
}
