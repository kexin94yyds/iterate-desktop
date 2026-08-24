use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MARKDOWN_IMAGE_REGISTRY_TTL_SECS: i64 = 60 * 60;
const MARKDOWN_IMAGE_REGISTRY_MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone)]
struct RegisteredMarkdownImage {
    path: PathBuf,
    registered_at: chrono::DateTime<chrono::Utc>,
}

static MARKDOWN_IMAGE_REGISTRY: Lazy<Arc<Mutex<HashMap<String, RegisteredMarkdownImage>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static MARKDOWN_IMAGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"!\[([^\]]*)\]\(([^)]*)\)").expect("valid markdown image regex"));

fn supported_markdown_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
            )
        })
        .unwrap_or(false)
}

fn percent_decode_lossy(value: &str) -> String {
    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

fn query_param_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (name, value) = part.split_once('=')?;
        if percent_decode_lossy(name) == key {
            Some(percent_decode_lossy(value))
        } else {
            None
        }
    })
}

fn markdown_image_source_to_local_path(source: &str) -> Option<PathBuf> {
    let source = source.trim();
    if source.is_empty()
        || source.starts_with("data:")
        || source.starts_with("http://")
        || source.starts_with("https://")
    {
        return None;
    }

    if let Some(query) = source.strip_prefix("/image?") {
        return query_param_value(query, "path").map(PathBuf::from);
    }

    if let Some(file_path) = source.strip_prefix("file://") {
        return Some(PathBuf::from(percent_decode_lossy(file_path)));
    }

    if source.starts_with('/') && !source.starts_with("/image?id=") {
        return Some(PathBuf::from(percent_decode_lossy(source)));
    }

    None
}

fn canonical_markdown_image_path(path: PathBuf) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).ok()?;
    let metadata = std::fs::metadata(&canonical).ok()?;
    if !metadata.is_file() || !supported_markdown_image_extension(&canonical) {
        return None;
    }
    Some(canonical)
}

fn prune_markdown_image_registry(
    registry: &mut HashMap<String, RegisteredMarkdownImage>,
    now: chrono::DateTime<chrono::Utc>,
) {
    registry.retain(|_, image| {
        image.registered_at + chrono::Duration::seconds(MARKDOWN_IMAGE_REGISTRY_TTL_SECS) > now
    });

    if registry.len() <= MARKDOWN_IMAGE_REGISTRY_MAX_ENTRIES {
        return;
    }

    let remove_count = registry.len() - MARKDOWN_IMAGE_REGISTRY_MAX_ENTRIES;
    let mut entries = registry
        .iter()
        .map(|(id, image)| (id.clone(), image.registered_at))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    for (id, _) in entries.into_iter().take(remove_count) {
        registry.remove(&id);
    }
}

fn register_markdown_image_path(path: PathBuf) -> Option<String> {
    let canonical = canonical_markdown_image_path(path)?;
    let id = format!("img_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now();
    let mut registry = MARKDOWN_IMAGE_REGISTRY.lock().ok()?;
    prune_markdown_image_registry(&mut registry, now);
    registry.insert(
        id.clone(),
        RegisteredMarkdownImage {
            path: canonical,
            registered_at: now,
        },
    );
    Some(id)
}

pub(super) fn registered_markdown_image_path(id: &str) -> Option<PathBuf> {
    let id = id.trim();
    if id.is_empty() {
        return None;
    }

    let now = chrono::Utc::now();
    let mut registry = MARKDOWN_IMAGE_REGISTRY.lock().ok()?;
    prune_markdown_image_registry(&mut registry, now);
    registry.get(id).map(|image| image.path.clone())
}

pub(super) fn rewrite_markdown_local_images(text: &str) -> Option<String> {
    if !text.contains("![") {
        return None;
    }

    let mut output = String::with_capacity(text.len());
    let mut last_end = 0;
    let mut changed = false;

    for captures in MARKDOWN_IMAGE_RE.captures_iter(text) {
        let Some(full_match) = captures.get(0) else {
            continue;
        };
        let alt = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let source = captures.get(2).map(|m| m.as_str()).unwrap_or("");
        let Some(local_path) = markdown_image_source_to_local_path(source) else {
            continue;
        };
        let Some(image_id) = register_markdown_image_path(local_path) else {
            continue;
        };

        output.push_str(&text[last_end..full_match.start()]);
        output.push_str(&format!("![{}](/image?id={})", alt, image_id));
        last_end = full_match.end();
        changed = true;
    }

    if !changed {
        return None;
    }

    output.push_str(&text[last_end..]);
    Some(output)
}

fn rewrite_json_string_field(value: &mut serde_json::Value) {
    let Some(text) = value.as_str() else {
        return;
    };
    if let Some(rewritten) = rewrite_markdown_local_images(text) {
        *value = serde_json::Value::String(rewritten);
    }
}

pub(super) fn register_markdown_images_for_mcp_state_payload(payload: &mut serde_json::Value) {
    if let Some(message) = payload.pointer_mut("/request/message") {
        rewrite_json_string_field(message);
    }

    if let Some(nodes) = payload
        .get_mut("timelineNodes")
        .and_then(|value| value.as_array_mut())
    {
        for node in nodes {
            if let Some(content) = node.get_mut("content") {
                rewrite_json_string_field(content);
            }
        }
    }
}

#[cfg(test)]
pub(super) fn clear_markdown_image_registry_for_tests() {
    if let Ok(mut registry) = MARKDOWN_IMAGE_REGISTRY.lock() {
        registry.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_markdown_image_registry_for_tests, registered_markdown_image_path};

    #[test]
    fn markdown_local_images_are_rewritten_to_registered_ids() {
        let file_path =
            std::env::temp_dir().join(format!("iterate-md-image-{}.png", uuid::Uuid::new_v4()));
        std::fs::write(&file_path, b"test image").expect("write test image");
        clear_markdown_image_registry_for_tests();

        let encoded_path = percent_encoding::utf8_percent_encode(
            &file_path.to_string_lossy(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        let message = format!("before\n![截图](/image?path={})\nafter", encoded_path);

        let rewritten =
            super::rewrite_markdown_local_images(&message).expect("image should rewrite");
        assert!(rewritten.contains("![截图](/image?id=img_"));
        assert!(!rewritten.contains(&file_path.to_string_lossy().to_string()));

        let image_id = rewritten
            .split("/image?id=")
            .nth(1)
            .and_then(|tail| tail.split(')').next())
            .expect("rewritten image id");
        assert_eq!(
            registered_markdown_image_path(image_id),
            std::fs::canonicalize(&file_path).ok()
        );

        let _ = std::fs::remove_file(&file_path);
    }
}
