//! Pai Room 编排工具模块
//!
//! 生成 codex-room 调度草案，供主会话按 room 协议执行批量任务

pub mod mcp;

// 重新导出主要类型和功能
pub use mcp::DispatchTool;
