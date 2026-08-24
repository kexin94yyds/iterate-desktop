// UI 交互相关常量

/// 窗口大小调整步长
pub const RESIZE_STEP: u32 = 50;

/// 窗口大小变化节流时间 (ms)
pub const RESIZE_THROTTLE_MS: u64 = 1000;

/// 窗口大小更新延迟时间 (ms)
pub const SIZE_UPDATE_DELAY_MS: u64 = 500;

/// 窗口大小检查延迟时间 (ms)
pub const SIZE_CHECK_DELAY_MS: u64 = 100;

/// 默认动画持续时间 (ms)
pub const DEFAULT_ANIMATION_DURATION: u64 = 300;

/// 快速动画持续时间 (ms)
pub const QUICK_ANIMATION_DURATION: u64 = 200;

/// 慢速动画持续时间 (ms)
pub const SLOW_ANIMATION_DURATION: u64 = 500;

/// 默认过渡缓动函数
pub const DEFAULT_EASING: &str = "cubic-bezier(0.25, 0.8, 0.25, 1)";

/// 弹性缓动函数
pub const BOUNCE_EASING: &str = "cubic-bezier(0.34, 1.56, 0.64, 1)";

/// 平滑缓动函数
pub const SMOOTH_EASING: &str = "cubic-bezier(0.4, 0, 0.2, 1)";

// UI 时间配置结构体
#[derive(Debug, Clone)]
pub struct UiTimings {
    pub resize_step: u32,
    pub resize_throttle_ms: u64,
    pub size_update_delay_ms: u64,
    pub size_check_delay_ms: u64,
    pub animation_duration_ms: u64,
}

impl Default for UiTimings {
    fn default() -> Self {
        Self {
            resize_step: RESIZE_STEP,
            resize_throttle_ms: RESIZE_THROTTLE_MS,
            size_update_delay_ms: SIZE_UPDATE_DELAY_MS,
            size_check_delay_ms: SIZE_CHECK_DELAY_MS,
            animation_duration_ms: DEFAULT_ANIMATION_DURATION,
        }
    }
}

impl UiTimings {
    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "resize_step": self.resize_step,
            "resize_throttle_ms": self.resize_throttle_ms,
            "size_update_delay_ms": self.size_update_delay_ms,
            "size_check_delay_ms": self.size_check_delay_ms,
            "animation_duration_ms": self.animation_duration_ms
        })
    }
}

// 便捷函数
/// 获取默认 UI 时间配置
pub fn get_default_ui_timings() -> UiTimings {
    UiTimings::default()
}
