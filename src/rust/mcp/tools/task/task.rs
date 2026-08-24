use anyhow::Result;
use rmcp::model::*;
use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 文件持久化任务系统
///
/// 任务存储在 {project_path}/.cunzhi-memory/tasks.json
/// JSON 格式，跨会话持久化
#[derive(Clone)]
pub struct TaskTool;

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    /// 操作类型: list / add / update / done / remove
    pub action: String,
    /// 项目路径（必需）
    pub project_path: String,
    /// 任务 ID（update/done/remove 时必需）
    pub task_id: Option<String>,
    /// 任务主题（add 时必需）
    pub subject: Option<String>,
    /// 任务状态（update 时可选）: pending / in_progress / done / blocked
    pub status: Option<String>,
    /// 优先级（add/update 时可选）: high / medium / low
    pub priority: Option<String>,
    /// 阻塞原因（update 时可选）
    pub blocked_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub subject: String,
    pub status: String,
    pub priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TaskStore {
    tasks: Vec<Task>,
    next_id: u32,
}

impl TaskTool {
    pub async fn handle(request: TaskRequest) -> Result<CallToolResult, McpError> {
        let tasks_path = Self::tasks_path(&request.project_path);

        match request.action.as_str() {
            "list" => Self::list_tasks(&tasks_path),
            "add" => Self::add_task(&tasks_path, &request),
            "update" => Self::update_task(&tasks_path, &request),
            "done" => Self::done_task(&tasks_path, &request),
            "remove" => Self::remove_task(&tasks_path, &request),
            _ => Err(McpError::invalid_params(
                format!(
                    "未知操作: {}。支持: list/add/update/done/remove",
                    request.action
                ),
                None,
            )),
        }
    }

    fn tasks_path(project_path: &str) -> PathBuf {
        PathBuf::from(project_path)
            .join(".cunzhi-memory")
            .join("tasks.json")
    }

    fn load_store(path: &PathBuf) -> TaskStore {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => TaskStore::default(),
            }
        } else {
            TaskStore::default()
        }
    }

    fn save_store(path: &PathBuf, store: &TaskStore) -> Result<(), McpError> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| McpError::internal_error(format!("创建目录失败: {}", e), None))?;
        }
        let json = serde_json::to_string_pretty(store)
            .map_err(|e| McpError::internal_error(format!("序列化失败: {}", e), None))?;
        fs::write(path, json)
            .map_err(|e| McpError::internal_error(format!("写入文件失败: {}", e), None))?;
        Ok(())
    }

    fn list_tasks(path: &PathBuf) -> Result<CallToolResult, McpError> {
        let store = Self::load_store(path);
        if store.tasks.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "📋 任务列表为空。用 action=add 添加任务。".to_string(),
            )]));
        }

        let mut output = String::from("📋 任务列表：\n\n");
        for task in &store.tasks {
            let icon = match task.status.as_str() {
                "done" => "✅",
                "in_progress" => "🔄",
                "blocked" => "🚫",
                _ => "⬜",
            };
            let priority_tag = match task.priority.as_str() {
                "high" => " 🔴",
                "low" => " 🟢",
                _ => "",
            };
            output.push_str(&format!(
                "{} [{}] {}{}\n",
                icon, task.id, task.subject, priority_tag
            ));
            if let Some(ref blocked) = task.blocked_by {
                output.push_str(&format!("   ⚠️ 阻塞: {}\n", blocked));
            }
        }

        let active = store.tasks.iter().filter(|t| t.status != "done").count();
        let done = store.tasks.iter().filter(|t| t.status == "done").count();
        output.push_str(&format!("\n活跃: {} | 完成: {}", active, done));

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    fn add_task(path: &PathBuf, req: &TaskRequest) -> Result<CallToolResult, McpError> {
        let subject = req.subject.as_ref().ok_or_else(|| {
            McpError::invalid_params("add 操作需要 subject 参数".to_string(), None)
        })?;

        let mut store = Self::load_store(path);
        store.next_id += 1;
        let task = Task {
            id: format!("T{}", store.next_id),
            subject: subject.clone(),
            status: "pending".to_string(),
            priority: req.priority.clone().unwrap_or_else(|| "medium".to_string()),
            blocked_by: None,
        };
        let task_id = task.id.clone();
        let task_subject = task.subject.clone();
        store.tasks.push(task);
        Self::save_store(path, &store)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "✅ 已添加任务 [{}]: {}",
            task_id, task_subject
        ))]))
    }

    fn update_task(path: &PathBuf, req: &TaskRequest) -> Result<CallToolResult, McpError> {
        let task_id = req.task_id.as_ref().ok_or_else(|| {
            McpError::invalid_params("update 操作需要 task_id 参数".to_string(), None)
        })?;

        let mut store = Self::load_store(path);
        let task = store
            .tasks
            .iter_mut()
            .find(|t| &t.id == task_id)
            .ok_or_else(|| McpError::invalid_params(format!("任务 {} 不存在", task_id), None))?;

        if let Some(ref status) = req.status {
            task.status = status.clone();
        }
        if let Some(ref priority) = req.priority {
            task.priority = priority.clone();
        }
        if let Some(ref subject) = req.subject {
            task.subject = subject.clone();
        }
        task.blocked_by = req.blocked_by.clone();

        let updated_task = task.clone();
        Self::save_store(path, &store)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "✅ 已更新任务 [{}]: {} ({})",
            updated_task.id, updated_task.subject, updated_task.status
        ))]))
    }

    fn done_task(path: &PathBuf, req: &TaskRequest) -> Result<CallToolResult, McpError> {
        let task_id = req.task_id.as_ref().ok_or_else(|| {
            McpError::invalid_params("done 操作需要 task_id 参数".to_string(), None)
        })?;

        let mut store = Self::load_store(path);
        let task = store
            .tasks
            .iter_mut()
            .find(|t| &t.id == task_id)
            .ok_or_else(|| McpError::invalid_params(format!("任务 {} 不存在", task_id), None))?;

        task.status = "done".to_string();
        task.blocked_by = None;
        let done_task = task.clone();
        Self::save_store(path, &store)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "✅ 任务 [{}] 已完成: {}",
            done_task.id, done_task.subject
        ))]))
    }

    fn remove_task(path: &PathBuf, req: &TaskRequest) -> Result<CallToolResult, McpError> {
        let task_id = req.task_id.as_ref().ok_or_else(|| {
            McpError::invalid_params("remove 操作需要 task_id 参数".to_string(), None)
        })?;

        let mut store = Self::load_store(path);
        let len_before = store.tasks.len();
        store.tasks.retain(|t| &t.id != task_id);
        if store.tasks.len() == len_before {
            return Err(McpError::invalid_params(
                format!("任务 {} 不存在", task_id),
                None,
            ));
        }
        Self::save_store(path, &store)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "🗑️ 已删除任务 [{}]",
            task_id
        ))]))
    }
}
