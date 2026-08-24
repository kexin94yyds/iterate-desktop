//! 智能代码审查交互工具模块
//!
//! 提供智能代码审查交互功能，支持预定义选项、自由文本输入和图片上传

pub mod logger;
pub mod mcp;

// 重新导出主要类型和功能
pub use logger::{append_conversation_log, ConversationEntry};
pub use mcp::InteractionTool;
