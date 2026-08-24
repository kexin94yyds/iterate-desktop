/// 字体配置常量

// 默认字体系列
pub const DEFAULT_FONT_FAMILY: &str = "inter";

// 默认字体大小
pub const DEFAULT_FONT_SIZE: &str = "medium";

// 默认自定义字体系列
pub const DEFAULT_CUSTOM_FONT_FAMILY: &str =
    "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

// 支持的字体系列选项
pub const FONT_FAMILIES: &[(&str, &str, &str)] = &[
    ("inter", "Inter", "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"),
    ("jetbrains-mono", "JetBrains Mono", "JetBrains Mono, 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace"),
    ("system", "系统默认", "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"),
    ("arial", "Arial", "Arial, 'Helvetica Neue', Helvetica, sans-serif"),
    ("helvetica", "Helvetica", "'Helvetica Neue', Helvetica, Arial, sans-serif"),
    ("times", "Times New Roman", "'Times New Roman', Times, serif"),
    ("georgia", "Georgia", "Georgia, 'Times New Roman', Times, serif"),
    ("courier", "Courier New", "'Courier New', Courier, monospace"),
    ("verdana", "Verdana", "Verdana, Geneva, sans-serif"),
    ("tahoma", "Tahoma", "Tahoma, Geneva, sans-serif"),
    ("microsoft-yahei", "微软雅黑", "'Microsoft YaHei', 'PingFang SC', 'Hiragino Sans GB', sans-serif"),
    ("pingfang", "苹方", "'PingFang SC', 'Microsoft YaHei', 'Hiragino Sans GB', sans-serif"),
    ("noto-sans-cjk", "思源黑体", "'Noto Sans CJK SC', 'Source Han Sans SC', 'Microsoft YaHei', sans-serif"),
    ("source-han-sans", "Source Han Sans", "'Source Han Sans SC', 'Noto Sans CJK SC', 'Microsoft YaHei', sans-serif"),
    ("custom", "自定义字体", ""),
];

// 字体大小配置
pub const FONT_SIZES: &[(&str, &str, f32)] = &[
    ("small", "小", 0.875),   // 14px base
    ("medium", "中", 1.0),    // 16px base
    ("large", "大", 1.125),   // 18px base
    ("xlarge", "特大", 1.25), // 20px base
];

// 字体大小比例映射
pub const FONT_SIZE_SCALES: &[(&str, &str, &str, &str, &str)] = &[
    ("xs", "0.75rem", "0.65625rem", "0.84375rem", "0.9375rem"), // 12px -> 10.5px -> 13.5px -> 15px
    ("sm", "0.875rem", "0.765625rem", "0.984375rem", "1.09375rem"), // 14px -> 12.25px -> 15.75px -> 17.5px
    (
        "base",
        "0.875rem",
        "0.765625rem",
        "0.984375rem",
        "1.09375rem",
    ), // 14px -> 12.25px -> 15.75px -> 17.5px
    ("lg", "1rem", "0.875rem", "1.125rem", "1.25rem"),              // 16px -> 14px -> 18px -> 20px
    ("xl", "1.125rem", "0.984375rem", "1.265625rem", "1.40625rem"), // 18px -> 15.75px -> 20.25px -> 22.5px
    ("2xl", "1.25rem", "1.09375rem", "1.40625rem", "1.5625rem"), // 20px -> 17.5px -> 22.5px -> 25px
];

// CSS 变量名
pub const CSS_VAR_FONT_FAMILY: &str = "--font-family";
pub const CSS_VAR_FONT_SIZE_SCALE: &str = "--font-size-scale";
