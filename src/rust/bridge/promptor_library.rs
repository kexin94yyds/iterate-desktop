use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn promptor_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("PROMPTER_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("promptor")
        .join("config.json")
}

pub(super) fn read_promptor_library() -> Result<Value, String> {
    read_promptor_library_from_path(&promptor_config_path())
}

fn read_promptor_library_from_path(path: &Path) -> Result<Value, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("promptor_config_unavailable: {error}"))?;
    let config: Value = serde_json::from_str(&content)
        .map_err(|error| format!("promptor_config_invalid: {error}"))?;

    let raw_modes = config
        .get("modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let raw_prompts = config
        .get("prompts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mode_names = raw_modes
        .iter()
        .filter_map(|mode| {
            let id = mode.get("id")?.as_str()?.trim();
            let name = mode.get("name")?.as_str()?.trim();
            if id.is_empty() || name.is_empty() {
                return None;
            }
            Some((id.to_string(), name.to_string()))
        })
        .collect::<HashMap<_, _>>();

    let mut mode_counts = HashMap::<String, usize>::new();
    let items = raw_prompts
        .iter()
        .filter_map(|prompt| {
            let id = prompt.get("id")?.as_str()?.trim();
            let name = prompt.get("name")?.as_str()?.trim();
            let content = prompt.get("content")?.as_str()?;
            if id.is_empty() || name.is_empty() || content.trim().is_empty() {
                return None;
            }

            let mode_id = prompt
                .get("modeId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            let category = mode_names
                .get(mode_id)
                .cloned()
                .unwrap_or_else(|| "未分类".to_string());
            *mode_counts.entry(mode_id.to_string()).or_default() += 1;

            Some(json!({
                "id": id,
                "name": name,
                "content": content,
                "category": category,
            }))
        })
        .collect::<Vec<_>>();

    let modes = raw_modes
        .iter()
        .filter_map(|mode| {
            let id = mode.get("id")?.as_str()?.trim();
            let name = mode.get("name")?.as_str()?.trim();
            if id.is_empty() || name.is_empty() {
                return None;
            }
            Some(json!({
                "id": id,
                "name": name,
                "promptCount": mode_counts.get(id).copied().unwrap_or(0),
            }))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "version": 1,
        "source": "promptor",
        "items": items,
        "modes": modes,
    }))
}

#[cfg(test)]
mod tests {
    use super::read_promptor_library_from_path;

    #[test]
    fn maps_only_prompt_fields_and_never_leaks_private_config() {
        let path = std::env::temp_dir().join(format!(
            "iterate-promptor-library-{}.json",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "license": { "key": "must-not-leak" },
                "windowBounds": { "x": 10 },
                "modes": [
                    { "id": "mode-skill", "name": "神技", "private": "hidden" }
                ],
                "prompts": [
                    {
                        "id": "prompt-cha",
                        "name": "审查代码",
                        "content": "cha",
                        "modeId": "mode-skill",
                        "secret": "hidden"
                    }
                ]
            }))
            .expect("serialize fixture"),
        )
        .expect("write fixture");

        let response = read_promptor_library_from_path(&path).expect("read promptor fixture");
        assert_eq!(response["source"], "promptor");
        assert_eq!(response["items"][0]["category"], "神技");
        assert_eq!(response["items"][0]["content"], "cha");
        assert_eq!(
            response["items"][0].as_object().expect("item object").len(),
            4
        );
        assert!(response.get("license").is_none());
        assert!(response.get("windowBounds").is_none());

        let _ = std::fs::remove_file(path);
    }
}
