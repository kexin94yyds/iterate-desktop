use std::path::{Component, Path, PathBuf};

/// Normalize a workspace path to an absolute, canonical form.
/// Returns `None` if the input is empty.
pub fn normalize_workspace_path(workspace: &str) -> Option<PathBuf> {
    let workspace = workspace.trim();
    if workspace.is_empty() {
        return None;
    }

    let raw_path = PathBuf::from(workspace);
    let absolute_path = if raw_path.is_absolute() {
        raw_path
    } else {
        std::env::current_dir().ok()?.join(raw_path)
    };

    let canonical_or_abs = std::fs::canonicalize(&absolute_path).unwrap_or(absolute_path);
    let mut normalized = PathBuf::new();
    for component in canonical_or_abs.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Some(normalized)
}

/// Count the depth of a path (number of Normal components).
/// Used to select the most specific (longest prefix) workspace match.
pub fn workspace_depth(path: &Path) -> usize {
    path.components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count()
}
