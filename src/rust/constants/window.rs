// 窗口相关常量

/// 窗口最小宽度 (px)
pub const MIN_WIDTH: f64 = 600.0;

/// 窗口最小高度 (px)
pub const MIN_HEIGHT: f64 = 400.0;

/// 窗口最大宽度 (px)
pub const MAX_WIDTH: f64 = 1500.0;

/// 窗口最大高度 (px)
pub const MAX_HEIGHT: f64 = 1000.0;

/// 窗口默认宽度 (px)
pub const DEFAULT_WIDTH: f64 = 600.0;

/// 窗口默认高度 (px)
pub const DEFAULT_HEIGHT: f64 = 900.0;

/// 默认置顶启用状态
pub const DEFAULT_ALWAYS_ON_TOP: bool = true;

/// 默认窗口固定模式状态
pub const DEFAULT_FIXED_MODE: bool = false;

/// 默认自动调整大小状态
pub const DEFAULT_AUTO_RESIZE: bool = true;

// 窗口约束结构体
#[derive(Debug, Clone)]
pub struct WindowConstraints {
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
    pub default_width: f64,
    pub default_height: f64,
}

impl Default for WindowConstraints {
    fn default() -> Self {
        Self {
            min_width: MIN_WIDTH,
            min_height: MIN_HEIGHT,
            max_width: MAX_WIDTH,
            max_height: MAX_HEIGHT,
            default_width: DEFAULT_WIDTH,
            default_height: DEFAULT_HEIGHT,
        }
    }
}

impl WindowConstraints {
    /// 验证窗口尺寸是否在约束范围内
    pub fn is_valid_size(&self, width: f64, height: f64) -> bool {
        width >= self.min_width
            && width <= self.max_width
            && height >= self.min_height
            && height <= self.max_height
    }

    /// 将尺寸限制在约束范围内
    pub fn clamp_size(&self, width: f64, height: f64) -> (f64, f64) {
        let clamped_width = width.clamp(self.min_width, self.max_width);
        let clamped_height = height.clamp(self.min_height, self.max_height);
        (clamped_width, clamped_height)
    }

    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "min_width": self.min_width,
            "min_height": self.min_height,
            "max_width": self.max_width,
            "max_height": self.max_height,
            "default_width": self.default_width,
            "default_height": self.default_height
        })
    }
}

// 便捷函数
/// 获取默认窗口约束
pub fn get_default_constraints() -> WindowConstraints {
    WindowConstraints::default()
}

/// 验证窗口尺寸是否有效
pub fn is_valid_window_size(width: f64, height: f64) -> bool {
    get_default_constraints().is_valid_size(width, height)
}

/// 将窗口尺寸限制在有效范围内
pub fn clamp_window_size(width: f64, height: f64) -> (f64, f64) {
    get_default_constraints().clamp_size(width, height)
}
