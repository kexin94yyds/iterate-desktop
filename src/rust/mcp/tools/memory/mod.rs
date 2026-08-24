//! 记忆管理工具模块
//!
//! 提供全局记忆管理功能，用于存储和管理重要的开发规范、用户偏好和最佳实践

pub mod commands;
pub mod manager;
pub mod mcp;
pub mod types;

// 重新导出主要类型和功能
pub use manager::MemoryManager;
pub use mcp::MemoryTool;
pub use types::{MemoryCategory, MemoryEntry, MemoryMetadata};
