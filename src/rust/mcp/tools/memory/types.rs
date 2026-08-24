use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 记忆条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 记忆分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MemoryCategory {
    Rule,       // 开发规范和规则
    Preference, // 用户偏好设置
    Note,       // 临时笔记（原 Pattern，避免与 knowledge 的 patterns.md 混淆）
    Context,    // 项目上下文信息
    Session,    // 会话摘要（L3 近期对话摘要）
}

/// 记忆元数据
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub project_path: String,
    pub last_organized: DateTime<Utc>,
    pub total_entries: usize,
    pub version: String,
}
