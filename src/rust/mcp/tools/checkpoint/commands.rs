use super::git_ops;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub id: String,
    #[serde(default)]
    pub checkpoint_id: Option<String>,
    pub name: String,
    pub timestamp: String,
    pub files: Vec<String>,
    pub message: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub deletable: bool,
}

impl From<git_ops::Checkpoint> for CheckpointInfo {
    fn from(cp: git_ops::Checkpoint) -> Self {
        CheckpointInfo {
            id: cp.id,
            checkpoint_id: cp.checkpoint_id,
            name: cp.name,
            timestamp: cp.timestamp.to_rfc3339(),
            files: cp.files,
            message: cp.message,
            kind: "git".to_string(),
            deletable: false,
        }
    }
}

#[tauri::command]
pub async fn create_checkpoint(
    project_path: String,
    message: String,
) -> Result<CheckpointInfo, String> {
    let checkpoint = git_ops::create_checkpoint(&project_path, &message)?;
    Ok(checkpoint.into())
}

#[tauri::command]
pub async fn list_checkpoints(project_path: String) -> Result<Vec<CheckpointInfo>, String> {
    let checkpoints = git_ops::list_checkpoints(&project_path)?;
    Ok(checkpoints.into_iter().map(CheckpointInfo::from).collect())
}

#[tauri::command]
pub async fn get_checkpoint_files(
    project_path: String,
    stash_id: String,
) -> Result<Vec<String>, String> {
    git_ops::get_checkpoint_files(&project_path, &stash_id)
}

#[tauri::command]
pub async fn restore_checkpoint(project_path: String, stash_id: String) -> Result<(), String> {
    git_ops::restore_checkpoint(&project_path, &stash_id)
}

#[tauri::command]
pub async fn restore_checkpoint_safe(
    project_path: String,
    stash_id: String,
    dry_run: bool,
    create_safety_checkpoint: bool,
    selected_files: Option<Vec<String>>,
    expected_plan_hash: Option<String>,
) -> Result<git_ops::RestoreCheckpointResult, String> {
    git_ops::restore_checkpoint_safe(
        &project_path,
        &stash_id,
        dry_run,
        create_safety_checkpoint,
        selected_files,
        expected_plan_hash,
    )
}

#[tauri::command]
pub async fn delete_checkpoint(project_path: String, stash_id: String) -> Result<(), String> {
    git_ops::delete_checkpoint(&project_path, &stash_id)
}
