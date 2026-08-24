pub(super) fn json_string_field(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = payload
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
    }
    None
}

pub(super) fn nested_metadata_string_field(
    payload: &serde_json::Value,
    keys: &[&str],
) -> Option<String> {
    payload.get("metadata").and_then(|metadata| {
        for key in keys {
            if let Some(value) = metadata
                .get(*key)
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(value.to_string());
            }
        }
        None
    })
}
