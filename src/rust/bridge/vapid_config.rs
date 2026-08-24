#[derive(Debug, Clone)]
pub(super) struct VapidConfig {
    pub(super) public_key: Option<String>,
    pub(super) private_key: Option<String>,
    pub(super) subject: Option<String>,
}

pub(super) fn load_vapid_config() -> VapidConfig {
    let file_config = load_vapid_from_file();

    let private_key = std::env::var("ITERATE_VAPID_PRIVATE_KEY")
        .or_else(|_| std::env::var("VAPID_PRIVATE_KEY"))
        .ok()
        .or_else(|| file_config.as_ref().and_then(|c| c.private_key.clone()));
    let public_key = std::env::var("ITERATE_VAPID_PUBLIC_KEY")
        .or_else(|_| std::env::var("VAPID_PUBLIC_KEY"))
        .ok()
        .or_else(|| file_config.as_ref().and_then(|c| c.public_key.clone()));
    let subject = std::env::var("ITERATE_VAPID_SUBJECT")
        .or_else(|_| std::env::var("VAPID_SUBJECT"))
        .ok()
        .or_else(|| file_config.as_ref().and_then(|c| c.subject.clone()));

    log::info!(
        "[Bridge] VAPID 配置: public_key={}, private_key={}, subject={}",
        public_key.is_some(),
        private_key.is_some(),
        subject.is_some()
    );

    VapidConfig {
        public_key,
        private_key,
        subject,
    }
}

fn load_vapid_from_file() -> Option<VapidConfig> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home)
        .join(".cunzhi")
        .join("vapid.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(VapidConfig {
        public_key: json
            .get("public_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        private_key: json
            .get("private_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        subject: json
            .get("subject")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}
