//! PTY 命令执行工具模块
//!
//! 通过 MCP Client 代理 Node.js Terminal MCP Server 的 PTY 功能

pub mod mcp;

pub use mcp::PtyExecTool;
