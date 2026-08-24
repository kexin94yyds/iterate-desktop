use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

const ITERATE_CONFIG_DIR_ENV: &str = "ITERATE_CONFIG_DIR";
const ANDROID_PACKAGE_CONFIG_DIR: &str = "/data/data/com.kexin94yyds.iterate/files/cunzhi";

/// Resolve Bridge-owned state (paired devices and the local auth broker).
///
/// Production keeps the historical `.../iterate` directory. Tests and other
/// explicitly isolated processes can reuse `ITERATE_CONFIG_DIR` so every
/// Bridge write stays inside the same disposable root as `config.json`.
pub fn iterate_bridge_state_dir() -> PathBuf {
    resolve_iterate_bridge_state_dir(
        || std::env::var_os(ITERATE_CONFIG_DIR_ENV),
        dirs::config_dir(),
        dirs::home_dir(),
    )
}

fn resolve_iterate_bridge_state_dir(
    explicit_config_dir: impl FnOnce() -> Option<OsString>,
    system_config_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = explicit_config_dir()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return path;
    }

    system_config_dir
        .map(|path| path.join("iterate"))
        .or_else(|| home_dir.map(|path| path.join(".config/iterate")))
        .unwrap_or_else(|| PathBuf::from("iterate"))
}

pub fn cunzhi_config_dir() -> Result<PathBuf> {
    let config_dir = resolve_cunzhi_config_dir(
        || std::env::var_os(ITERATE_CONFIG_DIR_ENV),
        dirs::config_dir(),
        cfg!(target_os = "android"),
    )?;
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

fn resolve_cunzhi_config_dir(
    explicit_config_dir: impl FnOnce() -> Option<OsString>,
    system_config_dir: Option<PathBuf>,
    is_android: bool,
) -> Result<PathBuf> {
    if let Some(path) = explicit_config_dir()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }

    if let Some(config_dir) = system_config_dir {
        return Ok(config_dir.join("cunzhi"));
    }

    if is_android {
        return Ok(PathBuf::from(ANDROID_PACKAGE_CONFIG_DIR));
    }

    Err(anyhow::anyhow!("无法获取配置目录"))
}

#[cfg(test)]
mod tests {
    use super::{resolve_cunzhi_config_dir, resolve_iterate_bridge_state_dir};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn resolver_prefers_explicit_iterate_config_dir() {
        let path = resolve_cunzhi_config_dir(
            || Some(OsString::from("/tmp/iterate-custom")),
            Some(PathBuf::from("/tmp/system-config")),
            true,
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/iterate-custom"));
    }

    #[test]
    fn resolver_uses_system_config_dir() {
        let path =
            resolve_cunzhi_config_dir(|| None, Some(PathBuf::from("/tmp/system-config")), false)
                .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/system-config/cunzhi"));
    }

    #[test]
    fn resolver_falls_back_to_android_private_files_dir() {
        let path = resolve_cunzhi_config_dir(|| None, None, true).unwrap();

        assert_eq!(
            path,
            PathBuf::from("/data/data/com.kexin94yyds.iterate/files/cunzhi")
        );
    }

    #[test]
    fn resolver_errors_without_any_non_android_config_dir() {
        let error = resolve_cunzhi_config_dir(|| None, None, false).unwrap_err();

        assert!(error.to_string().contains("无法获取配置目录"));
    }

    #[test]
    fn bridge_state_resolver_honors_explicit_config_dir() {
        let path = resolve_iterate_bridge_state_dir(
            || Some(OsString::from("/tmp/iterate-isolated")),
            Some(PathBuf::from("/tmp/system-config")),
            Some(PathBuf::from("/tmp/home")),
        );

        assert_eq!(path, PathBuf::from("/tmp/iterate-isolated"));
    }

    #[test]
    fn bridge_state_resolver_preserves_historical_default() {
        let path = resolve_iterate_bridge_state_dir(
            || None,
            Some(PathBuf::from("/tmp/system-config")),
            Some(PathBuf::from("/tmp/home")),
        );

        assert_eq!(path, PathBuf::from("/tmp/system-config/iterate"));
    }
}
