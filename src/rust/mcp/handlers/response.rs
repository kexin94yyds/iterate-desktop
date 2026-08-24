use anyhow::Result;
use rmcp::{model::Content, Error as McpError};

use crate::mcp::types::{McpResponse, McpResponseContent};

/// 解析 MCP 响应内容
///
/// 支持新的结构化格式和旧格式的兼容性，并生成适当的 Content 对象
pub fn parse_mcp_response(response: &str) -> Result<Vec<Content>, McpError> {
    if response.trim() == "CANCELLED" || response.trim() == "用户取消了操作" {
        return Ok(vec![Content::text("用户取消了操作".to_string())]);
    }

    // 首先尝试解析为新的结构化格式
    if let Ok(structured_response) = serde_json::from_str::<McpResponse>(response) {
        return parse_structured_response(structured_response);
    }

    // 回退到旧格式兼容性解析
    match serde_json::from_str::<Vec<McpResponseContent>>(response) {
        Ok(content_array) => {
            let mut result = Vec::new();
            let mut image_count = 0;
            let mut user_text_parts = Vec::new();

            for content in content_array {
                match content.content_type.as_str() {
                    "text" => {
                        if let Some(text) = content.text {
                            user_text_parts.push(text);
                        }
                    }
                    "image" => {
                        if let Some(source) = content.source {
                            if source.source_type == "base64" {
                                image_count += 1;
                                result.push(Content::image(
                                    source.data.clone(),
                                    source.media_type.clone(),
                                ));
                            }
                        }
                    }
                    _ => {
                        if let Some(text) = content.text {
                            user_text_parts.push(text);
                        }
                    }
                }
            }

            if image_count > 0 {
                user_text_parts.push(format!("附件: {} 张图片", image_count));
            }

            if !user_text_parts.is_empty() {
                let combined_text = user_text_parts.join("\n\n");
                result.push(Content::text(combined_text));
            }

            if result.is_empty() {
                result.push(Content::text("用户未提供任何内容".to_string()));
            }

            Ok(result)
        }
        Err(_) => {
            // 如果不是JSON格式，作为纯文本处理
            Ok(vec![Content::text(response.to_string())])
        }
    }
}

/// 解析新的结构化响应格式
fn parse_structured_response(response: McpResponse) -> Result<Vec<Content>, McpError> {
    let mut result = Vec::new();
    let mut text_parts = Vec::new();

    // 0. 暴露响应来源，避免 MCP 调用链吞掉 loop_auto_continue 语义
    if let Some(source) = response.metadata.source.as_ref() {
        let trimmed = source.trim();
        if !trimmed.is_empty() {
            text_parts.push(format!("响应来源: {}", trimmed));
        }
    }

    if let Some(snapshot) = response.metadata.hui_snapshot.as_ref() {
        let trimmed = snapshot.trim();
        if !trimmed.is_empty() {
            text_parts.push(trimmed.to_string());
        }
    }

    // 1. 处理用户输入文本。选项属于用户选择，应并入输入正文前面，而不是成为游离元数据。
    if let Some(user_input) =
        prepend_selected_options_to_user_input(response.user_input, &response.selected_options)
    {
        text_parts.push(user_input);
    }

    // 3. 处理图片附件（只传图片数据，不生成冗余文本描述）
    for image in response.images.iter() {
        result.push(Content::image(image.data.clone(), image.media_type.clone()));
    }

    // 4. 附件路径提示。能拿到真实路径时优先给路径，避免后续回溯只能靠数量猜。
    if !response.file_paths.is_empty() {
        text_parts.push(format_path_block("附加文件路径", &response.file_paths));
    }
    if !response.image_paths.is_empty() {
        text_parts.push(format_path_block("附加图片路径", &response.image_paths));
    }

    // 5. 附件计数提示
    if !response.images.is_empty() && response.image_paths.is_empty() {
        text_parts.push(format!("附件: {} 张图片", response.images.len()));
    }

    // 6. 将文本内容添加到结果中（图片后面）
    if !text_parts.is_empty() {
        let combined_text = text_parts.join("\n\n");
        result.push(Content::text(combined_text));
    }

    // 7. 如果没有任何内容，添加默认响应
    if result.is_empty() {
        result.push(Content::text("用户未提供任何内容".to_string()));
    }

    Ok(result)
}

fn format_path_block(label: &str, paths: &[String]) -> String {
    let lines = paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .map(|path| format!("- {}", path))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}：\n{}", label, lines)
}

fn prepend_selected_options_to_user_input(
    user_input: Option<String>,
    selected_options: &[String],
) -> Option<String> {
    let user_input = user_input
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let missing_options = selected_options
        .iter()
        .map(|option| option.trim())
        .filter(|option| {
            !option.is_empty()
                && user_input
                    .as_deref()
                    .map(|input| !input.contains(*option))
                    .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if missing_options.is_empty() {
        return user_input;
    }

    let prefix = format!("选中的选项: {}", missing_options.join(" / "));
    Some(match user_input {
        Some(input) => format!("{}\n\n{}", prefix, input),
        None => prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_text_content(response: &str) -> String {
        parse_mcp_response(response)
            .expect("parse response")
            .into_iter()
            .find_map(|content| content.as_text().map(|text| text.text.clone()))
            .expect("text content")
    }

    #[test]
    fn parse_structured_response_prepends_selected_options_to_user_input() {
        let response = serde_json::json!({
            "user_input": "✔️不明白的地方反问我，先不着急编码",
            "selected_options": ["按这个边界修 MCP 输入"],
            "images": [],
            "file_paths": [],
            "image_paths": [],
            "metadata": {
                "timestamp": "2026-06-17T00:00:00Z",
                "request_id": "req-1",
                "source": "popup"
            }
        })
        .to_string();

        let text = first_text_content(&response);
        assert!(text.contains("选中的选项: 按这个边界修 MCP 输入\n\n✔️不明白的地方反问我"));
    }

    #[test]
    fn parse_structured_response_keeps_option_only_submission_as_input() {
        let response = serde_json::json!({
            "user_input": "",
            "selected_options": ["a"],
            "images": [],
            "file_paths": [],
            "image_paths": [],
            "metadata": {
                "timestamp": "2026-06-17T00:00:00Z",
                "request_id": "req-1",
                "source": "popup"
            }
        })
        .to_string();

        let text = first_text_content(&response);
        assert!(text.contains("选中的选项: a"));
    }

    #[test]
    fn parse_structured_response_does_not_duplicate_existing_selected_option() {
        let response = serde_json::json!({
            "user_input": "选中的选项: a\n\n继续",
            "selected_options": ["a"],
            "images": [],
            "file_paths": [],
            "image_paths": [],
            "metadata": {
                "timestamp": "2026-06-17T00:00:00Z",
                "request_id": "req-1",
                "source": "popup"
            }
        })
        .to_string();

        let text = first_text_content(&response);
        assert_eq!(text.matches("选中的选项: a").count(), 1);
    }
}
