pub mod commands;
pub mod manager;
pub mod types;

pub use manager::ConversationManager;
pub use types::{
    ConversationNode, ConversationTree, NodeMetadata, NodeType, TimelineImageAttachment,
};

fn normalize_route_part(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != "Unknown")
        .map(ToOwned::to_owned)
}

pub fn resolve_tree_route_key(
    request_id: Option<&str>,
    project_path: Option<&str>,
) -> Option<String> {
    normalize_route_part(request_id).or_else(|| normalize_route_part(project_path))
}
