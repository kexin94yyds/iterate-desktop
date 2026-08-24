use anyhow::Result;
use rmcp::{model::*, Error as McpError};
use std::fs;
use std::path::Path;

use crate::mcp::types::CiRequest;

/// 提示词库搜索工具
///
/// 在 .cunzhi-knowledge/prompts/ 中搜索相关模板
#[derive(Clone)]
pub struct CiTool;

impl CiTool {
    /// 搜索提示词库
    pub async fn search_prompts(request: CiRequest) -> Result<CallToolResult, McpError> {
        let project_path = Path::new(&request.project_path);

        // 验证项目路径
        if !project_path.exists() {
            return Err(McpError::invalid_params(
                format!("项目路径不存在: {}", request.project_path),
                None,
            ));
        }

        // 查找 .cunzhi-knowledge 目录
        let knowledge_dir = project_path.join(".cunzhi-knowledge");
        if !knowledge_dir.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                "📭 项目未接入全局知识库，无法搜索提示词库",
            )]));
        }

        // 查找 prompts 目录
        let prompts_dir = knowledge_dir.join("prompts");
        if !prompts_dir.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                "📭 提示词库目录不存在",
            )]));
        }

        let dir_name = request.directory.to_lowercase();
        let target_dir = prompts_dir.join(&dir_name);

        // 检查目录是否存在
        if !target_dir.exists() || !target_dir.is_dir() {
            // 列出可用目录
            let available_dirs = Self::list_available_dirs(&prompts_dir);
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "📭 目录 `{}` 不存在\n\n**可用目录**：\n{}",
                dir_name, available_dirs
            ))]));
        }

        // 搜索目录中的模板
        let query = request.query.as_deref().unwrap_or("");
        let results = Self::search_in_directory(&target_dir, query)?;

        if results.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "📭 在 `prompts/{}/` 中未找到匹配的模板",
                dir_name
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "# 📚 提示词库搜索结果\n\n目录：`prompts/{}/`\n\n{}",
                dir_name, results
            ))]))
        }
    }

    /// 列出可用目录
    fn list_available_dirs(prompts_dir: &Path) -> String {
        let mut dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(prompts_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if !name.starts_with('.') {
                            dirs.push(format!("- `{}`", name));
                        }
                    }
                }
            }
        }
        dirs.sort();
        dirs.join("\n")
    }

    /// 在目录中搜索模板
    fn search_in_directory(dir: &Path, query: &str) -> Result<String, McpError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" || ext == "txt" {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                // 如果没有查询或文件名/内容匹配
                                if query.is_empty() {
                                    // 列出所有文件
                                    if let Ok(content) = fs::read_to_string(&path) {
                                        let summary = Self::get_file_summary(&content);
                                        results.push(format!("## {}\n\n{}", filename, summary));
                                    }
                                } else {
                                    // 搜索匹配的文件
                                    let filename_lower = filename.to_lowercase();
                                    let content = fs::read_to_string(&path).unwrap_or_default();
                                    let content_lower = content.to_lowercase();

                                    if filename_lower.contains(&query_lower)
                                        || content_lower.contains(&query_lower)
                                    {
                                        let summary = Self::get_file_summary(&content);
                                        results.push(format!("## {}\n\n{}", filename, summary));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 限制返回数量
        results.truncate(5);
        Ok(results.join("\n\n---\n\n"))
    }

    /// 获取文件摘要（前 20 行）
    fn get_file_summary(content: &str) -> String {
        let lines: Vec<&str> = content.lines().take(20).collect();
        let truncated = if content.lines().count() > 20 {
            "\n\n..."
        } else {
            ""
        };
        format!("{}{}", lines.join("\n"), truncated)
    }
}
