const CLOUDFLARE_TOKEN_SERVICE: &str = "xin.tobooks.iterate.cloudflare";
const CLOUDFLARE_TOKEN_ACCOUNT: &str = "cloudflare_tunnel_token";

fn cloudflare_token_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(CLOUDFLARE_TOKEN_SERVICE, CLOUDFLARE_TOKEN_ACCOUNT)
        .map_err(|error| format!("cloudflare_secret_entry_failed:{error}"))
}

pub fn save_cloudflare_tunnel_token(token: &str) -> Result<(), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err("token_empty".to_string());
    }

    cloudflare_token_entry()?
        .set_password(trimmed)
        .map_err(|error| format!("cloudflare_token_save_failed:{error}"))
}

pub fn read_cloudflare_tunnel_token() -> Result<String, String> {
    cloudflare_token_entry()?
        .get_password()
        .map_err(|error| format!("cloudflare_token_read_failed:{error}"))
}

pub fn delete_cloudflare_tunnel_token() -> Result<(), String> {
    match cloudflare_token_entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("cloudflare_token_delete_failed:{error}")),
    }
}

pub fn cloudflare_tunnel_token_exists() -> bool {
    read_cloudflare_tunnel_token()
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false)
}
