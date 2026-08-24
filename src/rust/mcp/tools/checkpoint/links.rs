use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointLinkEntry {
    pub schema_version: u32,
    pub project_path: String,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_message: Option<String>,
    pub tree_id: String,
    pub node_id: String,
    pub node_type: String,
    pub source: String,
    pub logged_at: String,
}

fn checkpoint_links_path(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(".cunzhi-memory")
        .join("checkpoint_links.jsonl")
}

pub fn append_checkpoint_link(entry: CheckpointLinkEntry) {
    let Some(checkpoint_id) = entry.checkpoint_id.as_deref() else {
        return;
    };
    let Some(checkpoint_commit) = entry.checkpoint_commit.as_deref() else {
        return;
    };
    if entry.project_path.trim().is_empty()
        || entry.request_id.trim().is_empty()
        || entry.tree_id.trim().is_empty()
        || entry.node_id.trim().is_empty()
    {
        return;
    }

    let registry_dir = PathBuf::from(&entry.project_path).join(".cunzhi-memory");
    if fs::create_dir_all(&registry_dir).is_err() {
        log::warn!(
            "[CHE-DEBUG][checkpoint-link] create_dir_all failed: {}",
            registry_dir.display()
        );
        return;
    }

    let Ok(line) = serde_json::to_string(&entry) else {
        log::warn!(
            "[CHE-DEBUG][checkpoint-link] serialize failed project_path={} request_id={} checkpoint_id={}",
            entry.project_path,
            entry.request_id,
            checkpoint_id
        );
        return;
    };

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(checkpoint_links_path(&entry.project_path))
    {
        let _ = writeln!(file, "{}", line);
        log::info!(
            "[CHE-DEBUG][checkpoint-link] appended project_path={} request_id={} checkpoint_id={} checkpoint_commit={}",
            entry.project_path,
            entry.request_id,
            checkpoint_id,
            checkpoint_commit
        );
    } else {
        log::warn!(
            "[CHE-DEBUG][checkpoint-link] open failed project_path={} request_id={} checkpoint_id={}",
            entry.project_path,
            entry.request_id,
            checkpoint_id
        );
    }
}

pub fn build_checkpoint_link_entry(
    project_path: &str,
    request_id: &str,
    parent_request_id: Option<&str>,
    checkpoint_id: Option<&str>,
    checkpoint_commit: Option<&str>,
    checkpoint_message: Option<&str>,
    tree_id: &str,
    node_id: &str,
    node_type: &str,
    source: &str,
) -> Option<CheckpointLinkEntry> {
    let Some(checkpoint_id) = checkpoint_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return None;
    };
    let Some(checkpoint_commit) = checkpoint_commit
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return None;
    };

    Some(CheckpointLinkEntry {
        schema_version: 1,
        project_path: project_path.to_string(),
        request_id: request_id.trim().to_string(),
        parent_request_id: parent_request_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        checkpoint_id: Some(checkpoint_id.to_string()),
        checkpoint_commit: Some(checkpoint_commit.to_string()),
        checkpoint_message: checkpoint_message
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        tree_id: tree_id.to_string(),
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        source: source.to_string(),
        logged_at: Local::now().to_rfc3339(),
    })
}
