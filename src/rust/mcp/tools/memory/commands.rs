use super::MemoryTool;
use crate::mcp::types::JiyiRequest;

fn default_category() -> String {
    "context".to_string()
}

#[derive(Debug, serde::Deserialize)]
pub struct ExecuteJiArgs {
    pub action: String,
    #[serde(alias = "projectPath", alias = "project_path")]
    pub project_path: String,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_category")]
    pub category: String,
}

fn call_tool_result_to_text(result: &rmcp::model::CallToolResult) -> Result<String, String> {
    let val = serde_json::to_value(result).map_err(|e| format!("结果序列化失败: {}", e))?;
    let mut out = String::new();
    if let Some(arr) = val.get("content").and_then(|v| v.as_array()) {
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(txt) = item.get("text").and_then(|t| t.as_str()) {
                    out.push_str(txt);
                }
            }
        }
    }

    if out.is_empty() {
        Ok(val.to_string())
    } else {
        Ok(out)
    }
}

#[tauri::command]
pub async fn execute_ji_tool(args: ExecuteJiArgs) -> Result<String, String> {
    let req = JiyiRequest {
        action: args.action,
        project_path: args.project_path,
        content: args.content,
        category: args.category,
    };

    let result = MemoryTool::jiyi(req).await.map_err(|e| e.to_string())?;

    call_tool_result_to_text(&result)
}
