use rmcp::model::Meta;
use serde_json::{Map, Value};

const THREAD_DEEPLINK_PREFIX: &str = "codex://threads/";
const THREAD_ID_KEYS: &[&str] = &[
    "threadId",
    "thread_id",
    "codex_thread_id",
    "codexThreadId",
    "conversationId",
    "conversation_id",
    "session_id",
    "sessionId",
];
const CODEX_META_PARENT_KEYS: &[&str] = &[
    "x-codex-turn-metadata",
    "codex_turn_metadata",
    "codexTurnMetadata",
];
const META_CONTAINER_KEYS: &[&str] = &[
    "_meta",
    "meta",
    "metadata",
    "request_meta",
    "requestMeta",
    "tool_call_context",
    "toolCallContext",
];

pub fn normalize_codex_thread_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() < 8 || trimmed.len() > 128 {
        return None;
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }

    Some(trimmed.to_string())
}

pub fn codex_thread_deeplink(thread_id: &str) -> Option<String> {
    normalize_codex_thread_id(thread_id)
        .map(|normalized| format!("{THREAD_DEEPLINK_PREFIX}{normalized}"))
}

pub fn normalize_codex_thread_deeplink(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let thread_id = trimmed
        .strip_prefix(THREAD_DEEPLINK_PREFIX)?
        .trim_end_matches('/');
    if thread_id.contains('?') || thread_id.contains('#') {
        return None;
    }

    codex_thread_deeplink(thread_id)
}

pub fn extract_codex_thread_id_from_metas<'a>(
    metas: impl IntoIterator<Item = &'a Meta>,
) -> Option<String> {
    metas
        .into_iter()
        .find_map(extract_codex_thread_id_from_meta)
}

pub fn extract_codex_thread_id_from_meta(meta: &Meta) -> Option<String> {
    extract_codex_thread_id_from_object(&meta.0)
}

pub fn extract_codex_thread_id_from_value(value: &Value) -> Option<String> {
    value
        .as_object()
        .and_then(extract_codex_thread_id_from_object)
}

fn extract_codex_thread_id_from_object(object: &Map<String, Value>) -> Option<String> {
    find_thread_id_in_object(object).or_else(|| {
        for parent_key in CODEX_META_PARENT_KEYS {
            let Some(parent) = object.get(*parent_key).and_then(Value::as_object) else {
                continue;
            };
            if let Some(thread_id) = find_thread_id_in_object(parent) {
                return Some(thread_id);
            }
        }

        for container_key in META_CONTAINER_KEYS {
            let Some(container) = object.get(*container_key).and_then(Value::as_object) else {
                continue;
            };
            if let Some(thread_id) = extract_codex_thread_id_from_object(container) {
                return Some(thread_id);
            }
        }

        None
    })
}

fn find_thread_id_in_object(object: &Map<String, Value>) -> Option<String> {
    THREAD_ID_KEYS
        .iter()
        .find_map(|key| thread_id_from_value(object.get(*key)))
}

fn thread_id_from_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .and_then(normalize_codex_thread_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_thread_id_from_meta_top_level() {
        let mut object = Map::new();
        object.insert(
            "threadId".to_string(),
            Value::String("019eca23-a6a1-7683-9674-4d37b577033c".to_string()),
        );
        let meta = Meta(object);

        assert_eq!(
            extract_codex_thread_id_from_meta(&meta).as_deref(),
            Some("019eca23-a6a1-7683-9674-4d37b577033c")
        );
    }

    #[test]
    fn extracts_thread_id_from_codex_turn_metadata_value() {
        let value = serde_json::json!({
            "_meta": {
                "x-codex-turn-metadata": {
                    "thread_id": "019eca23-a6a1-7683-9674-4d37b577033c"
                }
            }
        });

        assert_eq!(
            extract_codex_thread_id_from_value(&value).as_deref(),
            Some("019eca23-a6a1-7683-9674-4d37b577033c")
        );
    }
}
