use anyhow::Result;
use rmcp::{model::*, Error as McpError};

use super::{MemoryCategory, MemoryManager};
use crate::log_debug;
use crate::mcp::{
    utils::{project_path_error, validate_project_path},
    JiyiRequest,
};

/// 全局记忆管理工具
///
/// 用于存储和管理重要的开发规范、用户偏好和最佳实践
#[derive(Clone)]
pub struct MemoryTool;

impl MemoryTool {
    fn normalize_id_text(input: &str) -> String {
        input.replace(['‑', '–', '—', '−'], "-").replace('：', ":")
    }

    pub async fn jiyi(request: JiyiRequest) -> Result<CallToolResult, McpError> {
        // 使用增强的路径验证功能
        if let Err(e) = validate_project_path(&request.project_path) {
            return Err(project_path_error(format!(
                "路径验证失败: {}\n原始路径: {}\n请检查路径格式是否正确，特别是 Windows 路径应使用正确的盘符格式（如 C:\\path）",
                e,
                request.project_path
            )).into());
        }

        let manager = MemoryManager::new(&request.project_path)
            .map_err(|e| McpError::internal_error(format!("创建记忆管理器失败: {}", e), None))?;

        let result = match request.action.as_str() {
            "记忆" => {
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少记忆内容".to_string(), None));
                }

                let category = match request.category.as_str() {
                    "rule" => MemoryCategory::Rule,
                    "preference" => MemoryCategory::Preference,
                    "note" => MemoryCategory::Note,
                    "context" => MemoryCategory::Context,
                    _ => MemoryCategory::Context,
                };

                let id = manager
                    .add_memory(&request.content, category)
                    .map_err(|e| McpError::internal_error(format!("添加记忆失败: {}", e), None))?;

                format!(
                    "✅ 记忆已添加，ID: {}\n📝 内容: {}\n📂 分类: {:?}",
                    id, request.content, category
                )
            }
            "回忆" => {
                let memory_info = manager.get_project_info().map_err(|e| {
                    McpError::internal_error(format!("获取项目记忆失败: {}", e), None)
                })?;
                let knowledge_info = manager.read_knowledge().map_err(|e| {
                    McpError::internal_error(format!("获取知识库失败: {}", e), None)
                })?;

                format!("{}\n{}", memory_info, knowledge_info)
            }
            "沉淀" => {
                // 沉淀到 .cunzhi-knowledge/
                // - problems: 直接写入 + 自动 push（不询问）
                // - patterns: 返回预览，需要用户确认是否补充
                // - regressions: 直接写入 + 自动 push（不询问）
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少沉淀内容".to_string(), None));
                }

                // 验证 category 是否为 knowledge 专用类型，支持单复数形式
                let category = match request.category.as_str() {
                    "patterns" | "problems" | "regressions" => request.category.as_str(),
                    "pattern" => "patterns",
                    "problem" => "problems",
                    "regression" => "regressions",
                    _ => {
                        return Err(McpError::invalid_params(
                            format!(
                                "沉淀仅支持 patterns/problems/regressions 分类，收到: {}",
                                request.category
                            ),
                            None,
                        ))
                    }
                };

                // 验证格式
                match category {
                    "problems" => {
                        let pattern = regex::Regex::new(r"P[‑–—−-]\d{4}[‑–—−-]\d{3}").unwrap();
                        let content_trimmed = request.content.trim();
                        let normalized = Self::normalize_id_text(content_trimmed);
                        if !pattern.is_match(&normalized) {
                            let first_line = normalized.lines().next().unwrap_or("");
                            log_debug!(
                                "ji(沉淀) problems ID 校验失败: content_len={}, first_line={}",
                                normalized.len(),
                                first_line
                            );
                            return Err(McpError::invalid_params(
                                format!(
                                    "沉淀 problems 必须包含 P-YYYY-NNN 格式的编号（如 P-2024-001）。当前内容长度={}，首行片段={}，完整内容=[{}]",
                                    normalized.len(),
                                    first_line,
                                    normalized
                                ),
                                None,
                            ));
                        }
                    }
                    "patterns" => {
                        let pattern = regex::Regex::new(r"PAT[‑–—−-]\d{4}[‑–—−-]\d{3}").unwrap();
                        let content_trimmed = request.content.trim();
                        let normalized = Self::normalize_id_text(content_trimmed);
                        if !pattern.is_match(&normalized) {
                            let first_line = normalized.lines().next().unwrap_or("");
                            log_debug!(
                                "ji(沉淀) patterns ID 校验失败: content_len={}, first_line={}",
                                normalized.len(),
                                first_line
                            );
                            return Err(McpError::invalid_params(
                                format!(
                                    "沉淀 patterns 必须包含 PAT-YYYY-NNN 格式的编号（如 PAT-2024-001）。当前内容长度={}，首行片段={}",
                                    normalized.len(),
                                    first_line
                                ),
                                None,
                            ));
                        }
                    }
                    "regressions" => {
                        let pattern = regex::Regex::new(r"R[‑–—−-]\d{4}[‑–—−-]\d{3}").unwrap();
                        let content_trimmed = request.content.trim();
                        let normalized = Self::normalize_id_text(content_trimmed);
                        if !pattern.is_match(&normalized) {
                            let first_line = normalized.lines().next().unwrap_or("");
                            log_debug!(
                                "ji(沉淀) regressions ID 校验失败: content_len={}, first_line={}",
                                normalized.len(),
                                first_line
                            );
                            return Err(McpError::invalid_params(
                                format!(
                                    "沉淀 regressions 必须包含 R-YYYY-NNN 格式的编号（如 R-2024-001）。当前内容长度={}，首行片段={}",
                                    normalized.len(),
                                    first_line
                                ),
                                None,
                            ));
                        }
                    }
                    _ => {}
                }

                // problems 和 regressions: 直接写入 + 自动 push
                // patterns: 返回预览，需要用户确认
                if category == "problems" || category == "regressions" {
                    manager
                        .settle_to_knowledge(&request.content, category)
                        .map_err(|e| McpError::internal_error(format!("沉淀失败: {}", e), None))?
                } else {
                    // patterns: 返回预览，不执行写入
                    format!(
                        r#"📋 **沉淀预览**

> 目标文件: `.cunzhi-knowledge/patterns.md`

```
{}
```

⚠️ **请调用 `zhi` 工具让用户确认**，确认后再调用 `ji(action=确认沉淀)` 执行写入。"#,
                        &request.content
                    )
                }
            }
            "确认沉淀" => {
                // 用户确认后执行 patterns 写入（problems/regressions 不走这个分支）
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少沉淀内容".to_string(), None));
                }

                // 确认沉淀只用于 patterns
                if request.category.as_str() != "patterns" {
                    return Err(McpError::invalid_params(
                        "确认沉淀仅用于 patterns 分类（problems/regressions 直接写入）".to_string(),
                        None,
                    ));
                }

                // 验证 patterns 格式
                let pattern = regex::Regex::new(r"PAT-\d{4}-\d{3}").unwrap();
                if !pattern.is_match(&request.content) {
                    return Err(McpError::invalid_params(
                        "沉淀 patterns 必须包含 PAT-YYYY-NNN 格式的编号（如 PAT-2024-001）"
                            .to_string(),
                        None,
                    ));
                }

                manager
                    .settle_to_knowledge(&request.content, "patterns")
                    .map_err(|e| McpError::internal_error(format!("沉淀失败: {}", e), None))?
            }
            "摘要" => {
                if request.content.trim().is_empty() {
                    return Err(McpError::invalid_params("缺少摘要内容".to_string(), None));
                }

                manager
                    .add_session_summary(&request.content)
                    .map_err(|e| McpError::internal_error(format!("添加摘要失败: {}", e), None))?
            }
            "" | "选择" => {
                // 返回选项菜单，让 AI 调用 zhi 展示给用户
                r#"📋 **请选择 ji 操作**

请调用 `zhi` 工具展示以下选项：

**a** = 沉淀 → 写入 `.cunzhi-knowledge/` (problems/patterns/regressions)
**b** = 记忆 → 写入 `.cunzhi-memory/` (context/preference/rule)

用户选择后，再调用 `ji(action=沉淀)` 或 `ji(action=记忆)`"#
                    .to_string()
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!(
                        "未知的操作类型: {}。可选：回忆/记忆/沉淀/摘要，或留空显示选项",
                        request.action
                    ),
                    None,
                ));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}
