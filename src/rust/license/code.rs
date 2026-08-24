use anyhow::Result;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use chrono::Utc;
use ring::rand::SystemRandom;
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const LICENSE_CODE_PREFIX: &str = "ITL1";
pub const LICENSE_PRODUCT: &str = "iterate";
pub const LICENSE_PUBLIC_KEY_B64: &str = "NR9khjNx6spHXylqMhSugj7+ftIR4M+JfQyaLu1TQLU=";

#[cfg(test)]
pub(crate) static LICENSE_TEST_ENV_LOCK: once_cell::sync::Lazy<std::sync::Mutex<()>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(()));

fn configured_public_key_b64() -> String {
    std::env::var("ITERATE_LICENSE_PUBLIC_KEY_B64")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| LICENSE_PUBLIC_KEY_B64.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedLicensePayload {
    pub version: u8,
    pub product: String,
    pub license_type: String,
    pub issued_at: String,
    pub nonce: String,
}

pub fn normalize_license_type(input: &str) -> Result<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "day1" | "1d" | "1-day" | "t1" => Ok("day1"),
        "day7" | "7d" | "7-day" | "t7" => Ok("day7"),
        "permanent" | "lifetime" | "forever" | "p" => Ok("permanent"),
        other => Err(anyhow::anyhow!("不支持的激活码类型: {}", other)),
    }
}

pub fn license_days(license_type: &str) -> Option<u64> {
    match normalize_license_type(license_type).ok() {
        Some("day1") => Some(1),
        Some("day7") => Some(7),
        Some("permanent") => None,
        _ => Some(7),
    }
}

pub fn license_type_label(license_type: &str) -> &'static str {
    match normalize_license_type(license_type).ok() {
        Some("day1") => "1天体验",
        Some("day7") => "7天体验",
        Some("permanent") => "永久版",
        _ => "未知",
    }
}

pub fn generate_signing_keypair() -> Result<(String, String)> {
    let rng = SystemRandom::new();
    let pkcs8 =
        Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| anyhow::anyhow!("生成签名密钥失败"))?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| anyhow::anyhow!("解析生成的签名密钥失败"))?;

    Ok((
        STANDARD.encode(pkcs8.as_ref()),
        STANDARD.encode(key_pair.public_key().as_ref()),
    ))
}

pub fn generate_license_code(license_type: &str, private_key_b64: &str) -> Result<String> {
    let license_type = normalize_license_type(license_type)?;
    let payload = SignedLicensePayload {
        version: 1,
        product: LICENSE_PRODUCT.to_string(),
        license_type: license_type.to_string(),
        issued_at: Utc::now().to_rfc3339(),
        nonce: Uuid::new_v4().to_string(),
    };

    let payload_bytes = serde_json::to_vec(&payload)?;
    let private_key_bytes = STANDARD
        .decode(private_key_b64.trim())
        .map_err(|e| anyhow::anyhow!("私钥 base64 解码失败: {}", e))?;
    let key_pair = Ed25519KeyPair::from_pkcs8(&private_key_bytes)
        .map_err(|_| anyhow::anyhow!("私钥格式无效，需要 PKCS8 base64"))?;
    let signature = key_pair.sign(&payload_bytes);

    Ok(format!(
        "{}.{}.{}",
        LICENSE_CODE_PREFIX,
        URL_SAFE_NO_PAD.encode(payload_bytes),
        URL_SAFE_NO_PAD.encode(signature.as_ref())
    ))
}

pub fn generate_license_codes(license_type: &str, count: usize) -> Result<Vec<String>> {
    let private_key_b64 = std::env::var("ITERATE_LICENSE_PRIVATE_KEY_B64").map_err(|_| {
        anyhow::anyhow!("缺少 ITERATE_LICENSE_PRIVATE_KEY_B64，无法从 CLI 批量生成激活码")
    })?;

    let mut codes = Vec::with_capacity(count);
    for _ in 0..count {
        codes.push(generate_license_code(license_type, &private_key_b64)?);
    }
    Ok(codes)
}

pub fn parse_and_verify_license_code(code: &str) -> Result<SignedLicensePayload> {
    let mut parts = code.trim().split('.');
    let prefix = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("激活码格式无效"))?;
    let payload_b64 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("激活码格式无效"))?;
    let signature_b64 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("激活码格式无效"))?;
    if parts.next().is_some() {
        return Err(anyhow::anyhow!("激活码格式无效"));
    }
    if prefix != LICENSE_CODE_PREFIX {
        return Err(anyhow::anyhow!("激活码前缀无效"));
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| anyhow::anyhow!("激活码内容解码失败: {}", e))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|e| anyhow::anyhow!("激活码签名解码失败: {}", e))?;
    let public_key = STANDARD
        .decode(configured_public_key_b64())
        .map_err(|e| anyhow::anyhow!("公钥解码失败: {}", e))?;

    let verifier = UnparsedPublicKey::new(&signature::ED25519, public_key);
    verifier
        .verify(&payload_bytes, &signature_bytes)
        .map_err(|_| anyhow::anyhow!("激活码签名无效"))?;

    let payload: SignedLicensePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| anyhow::anyhow!("激活码内容格式无效: {}", e))?;
    if payload.version != 1 {
        return Err(anyhow::anyhow!("激活码版本不支持"));
    }
    if payload.product != LICENSE_PRODUCT {
        return Err(anyhow::anyhow!("激活码不属于当前产品"));
    }
    let _ = normalize_license_type(&payload.license_type)?;

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::{generate_license_code, generate_signing_keypair, parse_and_verify_license_code};
    use crate::license::code::LICENSE_TEST_ENV_LOCK;

    struct PublicKeyGuard {
        previous: Option<String>,
    }

    impl PublicKeyGuard {
        fn set(value: &str) -> Self {
            let previous = std::env::var("ITERATE_LICENSE_PUBLIC_KEY_B64").ok();
            std::env::set_var("ITERATE_LICENSE_PUBLIC_KEY_B64", value);
            Self { previous }
        }
    }

    impl Drop for PublicKeyGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.as_ref() {
                std::env::set_var("ITERATE_LICENSE_PUBLIC_KEY_B64", previous);
            } else {
                std::env::remove_var("ITERATE_LICENSE_PUBLIC_KEY_B64");
            }
        }
    }

    #[test]
    fn generate_and_verify_day7_license_code() {
        let _lock = LICENSE_TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (private_key_b64, public_key_b64) = generate_signing_keypair().unwrap();
        let _guard = PublicKeyGuard::set(&public_key_b64);
        let code = generate_license_code("day7", &private_key_b64).unwrap();
        let payload = parse_and_verify_license_code(&code).unwrap();

        assert_eq!(payload.license_type, "day7");
        assert_eq!(payload.product, "iterate");
    }

    #[test]
    fn reject_tampered_license_code() {
        let _lock = LICENSE_TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (private_key_b64, public_key_b64) = generate_signing_keypair().unwrap();
        let _guard = PublicKeyGuard::set(&public_key_b64);
        let code = generate_license_code("day1", &private_key_b64).unwrap();
        let mut tampered = code.clone();
        tampered.push('x');

        assert!(parse_and_verify_license_code(&tampered).is_err());
    }
}
