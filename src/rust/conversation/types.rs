use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub node_type: NodeType,
    pub content: String,
    pub is_markdown: bool,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineImageAttachment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    pub media_type: String,
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_len: Option<u64>,
}

impl NodeType {
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetadata {
    #[serde(default)]
    pub conversation_id: Option<String>,
    pub project_path: Option<String>,
    pub predefined_options: Option<Vec<String>>,
    pub selected_option: Option<String>,
    pub images: Option<Vec<TimelineImageAttachment>>,
    pub link_url: Option<String>,
    pub link_title: Option<String>,
    pub request_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub stale_of: Option<String>,
    #[serde(default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub checkpoint_id: Option<String>,
    #[serde(default)]
    pub checkpoint_commit: Option<String>,
    #[serde(default)]
    pub checkpoint_message: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTree {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub current_node_id: String,
    pub nodes: HashMap<String, ConversationNode>,
    pub branches: HashMap<String, Vec<String>>,
}
