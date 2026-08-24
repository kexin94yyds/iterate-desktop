// 音频相关常量

/// 默认音频 URL（空字符串表示使用内置音效）
pub const DEFAULT_URL: &str = "";

/// 默认音频通知启用状态
pub const DEFAULT_NOTIFICATION_ENABLED: bool = false;

/// 音频文件支持的格式
pub const SUPPORTED_FORMATS: &[&str] = &["mp3", "wav", "ogg", "m4a"];

/// 默认音量 (0.0 - 1.0)
pub const DEFAULT_VOLUME: f32 = 0.8;

/// 最大音频文件大小 (bytes) - 10MB
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// 受管自定义音频在配置中的标识前缀。
pub const MANAGED_AUDIO_PREFIX: &str = "managed-audio:";

/// 受管自定义音频使用固定文件名，避免保留用户原始文件名和路径。
pub const MANAGED_AUDIO_BASENAME: &str = "custom-notification";

// 音频配置结构体
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub default_url: String,
    pub notification_enabled: bool,
    pub supported_formats: Vec<String>,
    pub default_volume: f32,
    pub max_file_size: u64,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            default_url: DEFAULT_URL.to_string(),
            notification_enabled: DEFAULT_NOTIFICATION_ENABLED,
            supported_formats: SUPPORTED_FORMATS.iter().map(|s| s.to_string()).collect(),
            default_volume: DEFAULT_VOLUME,
            max_file_size: MAX_FILE_SIZE,
        }
    }
}

impl AudioConfig {
    /// 验证音频格式是否支持
    pub fn is_supported_format(&self, format: &str) -> bool {
        self.supported_formats.contains(&format.to_lowercase())
    }

    /// 验证音频文件大小是否有效
    pub fn is_valid_file_size(&self, size: u64) -> bool {
        size <= self.max_file_size
    }

    /// 验证音量是否有效
    pub fn is_valid_volume(&self, volume: f32) -> bool {
        (0.0..=1.0).contains(&volume)
    }

    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "default_url": self.default_url,
            "notification_enabled": self.notification_enabled,
            "supported_formats": self.supported_formats,
            "default_volume": self.default_volume,
            "max_file_size": self.max_file_size
        })
    }
}

// 便捷函数
/// 获取默认音频配置
pub fn get_default_audio_config() -> AudioConfig {
    AudioConfig::default()
}

/// 验证音频格式是否支持
pub fn is_supported_audio_format(format: &str) -> bool {
    SUPPORTED_FORMATS.contains(&format.to_lowercase().as_str())
}

/// 验证音频文件大小是否有效
pub fn is_valid_audio_file_size(size: u64) -> bool {
    size <= MAX_FILE_SIZE
}

pub fn managed_audio_reference(extension: &str) -> Option<String> {
    let extension = extension.to_ascii_lowercase();
    is_supported_audio_format(&extension).then(|| {
        format!(
            "{}{}.{}",
            MANAGED_AUDIO_PREFIX, MANAGED_AUDIO_BASENAME, extension
        )
    })
}

pub fn managed_audio_filename(reference: &str) -> Option<&str> {
    let filename = reference.strip_prefix(MANAGED_AUDIO_PREFIX)?;
    let (basename, extension) = filename.rsplit_once('.')?;
    if basename != MANAGED_AUDIO_BASENAME || !is_supported_audio_format(extension) {
        return None;
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return None;
    }
    Some(filename)
}

pub fn is_safe_audio_config_reference(reference: &str) -> bool {
    if reference.is_empty() || managed_audio_filename(reference).is_some() {
        return true;
    }

    reference.len() <= 160
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::{is_safe_audio_config_reference, managed_audio_filename, managed_audio_reference};

    #[test]
    fn managed_audio_references_are_fixed_and_traversal_safe() {
        assert_eq!(
            managed_audio_reference("MP3").as_deref(),
            Some("managed-audio:custom-notification.mp3")
        );
        assert_eq!(
            managed_audio_filename("managed-audio:custom-notification.wav"),
            Some("custom-notification.wav")
        );
        assert!(managed_audio_filename("managed-audio:../secret.mp3").is_none());
        assert!(managed_audio_filename("managed-audio:other.mp3").is_none());
    }

    #[test]
    fn persisted_audio_references_reject_paths_and_urls() {
        assert!(is_safe_audio_config_reference(""));
        assert!(is_safe_audio_config_reference("notification-ping-372479"));
        assert!(is_safe_audio_config_reference(
            "managed-audio:custom-notification.ogg"
        ));
        assert!(!is_safe_audio_config_reference("/Users/example/sound.mp3"));
        assert!(!is_safe_audio_config_reference("C:\\audio\\sound.wav"));
        assert!(!is_safe_audio_config_reference(
            "https://example.com/sound.mp3"
        ));
    }
}
