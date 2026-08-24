// 网络相关常量

/// 默认请求超时时间 (ms)
pub const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// 默认重试次数
pub const DEFAULT_RETRY_COUNT: u32 = 3;

/// 默认重试间隔 (ms)
pub const DEFAULT_RETRY_INTERVAL_MS: u64 = 1000;

/// 最大重试次数
pub const MAX_RETRY_COUNT: u32 = 10;

/// 最小重试间隔 (ms)
pub const MIN_RETRY_INTERVAL_MS: u64 = 100;

/// 最大重试间隔 (ms)
pub const MAX_RETRY_INTERVAL_MS: u64 = 60000;

/// 连接超时时间 (ms)
pub const CONNECTION_TIMEOUT_MS: u64 = 10000;

/// 读取超时时间 (ms)
pub const READ_TIMEOUT_MS: u64 = 30000;

/// 写入超时时间 (ms)
pub const WRITE_TIMEOUT_MS: u64 = 10000;

/// 最大并发连接数
pub const MAX_CONCURRENT_CONNECTIONS: usize = 10;

/// 默认用户代理
pub const DEFAULT_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// 网络配置结构体
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub timeout_ms: u64,
    pub retry_count: u32,
    pub retry_interval_ms: u64,
    pub connection_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub write_timeout_ms: u64,
    pub max_concurrent_connections: usize,
    pub user_agent: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            retry_count: DEFAULT_RETRY_COUNT,
            retry_interval_ms: DEFAULT_RETRY_INTERVAL_MS,
            connection_timeout_ms: CONNECTION_TIMEOUT_MS,
            read_timeout_ms: READ_TIMEOUT_MS,
            write_timeout_ms: WRITE_TIMEOUT_MS,
            max_concurrent_connections: MAX_CONCURRENT_CONNECTIONS,
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }
}

impl NetworkConfig {
    /// 验证配置是否有效
    pub fn is_valid(&self) -> bool {
        self.timeout_ms > 0
            && self.retry_count <= MAX_RETRY_COUNT
            && self.retry_interval_ms >= MIN_RETRY_INTERVAL_MS
            && self.retry_interval_ms <= MAX_RETRY_INTERVAL_MS
            && self.connection_timeout_ms > 0
            && self.read_timeout_ms > 0
            && self.write_timeout_ms > 0
            && self.max_concurrent_connections > 0
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// 设置重试次数
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count.min(MAX_RETRY_COUNT);
        self
    }

    /// 设置重试间隔
    pub fn with_retry_interval(mut self, interval_ms: u64) -> Self {
        self.retry_interval_ms = interval_ms.clamp(MIN_RETRY_INTERVAL_MS, MAX_RETRY_INTERVAL_MS);
        self
    }

    /// 转换为 JSON 格式
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "timeout_ms": self.timeout_ms,
            "retry_count": self.retry_count,
            "retry_interval_ms": self.retry_interval_ms,
            "connection_timeout_ms": self.connection_timeout_ms,
            "read_timeout_ms": self.read_timeout_ms,
            "write_timeout_ms": self.write_timeout_ms,
            "max_concurrent_connections": self.max_concurrent_connections,
            "user_agent": self.user_agent
        })
    }
}

// 便捷函数
/// 获取默认网络配置
pub fn get_default_network_config() -> NetworkConfig {
    NetworkConfig::default()
}

/// 获取快速网络配置（较短的超时时间）
pub fn get_fast_network_config() -> NetworkConfig {
    NetworkConfig::default()
        .with_timeout(5000)
        .with_retry_count(1)
        .with_retry_interval(500)
}

/// 获取可靠网络配置（较长的超时时间和更多重试）
pub fn get_reliable_network_config() -> NetworkConfig {
    NetworkConfig::default()
        .with_timeout(60000)
        .with_retry_count(5)
        .with_retry_interval(2000)
}
