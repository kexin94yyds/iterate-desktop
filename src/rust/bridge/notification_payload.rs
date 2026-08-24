pub(super) fn extract_notification_body(payload: &serde_json::Value) -> Option<String> {
    if bridge_payload_suppresses_remote_notification(payload) {
        return None;
    }

    payload
        .get("request")
        .and_then(|request| request.get("message"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            payload
                .get("message")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
}

pub(super) fn bridge_payload_suppresses_remote_notification(payload: &serde_json::Value) -> bool {
    payload
        .get("suppress_remote_notification")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || payload
            .get("sync_response")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || payload
            .get("metadata")
            .and_then(|metadata| metadata.get("suppress_remote_notification"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

pub(super) fn extract_notification_project_path(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("request")
        .and_then(|request| request.get("project_path"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

pub(super) fn trim_notification_body(body: &str, max_chars: usize) -> String {
    let trimmed = body.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut shortened = trimmed.chars().take(max_chars).collect::<String>();
    shortened.push_str("...");
    shortened
}

#[cfg(test)]
mod tests {
    use super::{
        bridge_payload_suppresses_remote_notification, extract_notification_body,
        extract_notification_project_path, trim_notification_body,
    };

    #[test]
    fn extracts_notification_body_from_request_or_message() {
        let request_payload = serde_json::json!({
            "request": {
                "message": "from request",
                "project_path": "/tmp/project"
            },
            "message": "fallback"
        });
        assert_eq!(
            extract_notification_body(&request_payload).as_deref(),
            Some("from request")
        );
        assert_eq!(
            extract_notification_project_path(&request_payload).as_deref(),
            Some("/tmp/project")
        );

        let fallback_payload = serde_json::json!({ "message": "fallback" });
        assert_eq!(
            extract_notification_body(&fallback_payload).as_deref(),
            Some("fallback")
        );
    }

    #[test]
    fn suppresses_remote_notification_from_known_flags() {
        for payload in [
            serde_json::json!({ "suppress_remote_notification": true }),
            serde_json::json!({ "sync_response": true }),
            serde_json::json!({
                "metadata": {
                    "suppress_remote_notification": true
                }
            }),
        ] {
            assert!(bridge_payload_suppresses_remote_notification(&payload));
            assert!(extract_notification_body(&payload).is_none());
        }
    }

    #[test]
    fn trims_notification_body_by_chars() {
        assert_eq!(trim_notification_body("  hello  ", 10), "hello");
        assert_eq!(trim_notification_body("abcdef", 3), "abc...");
        assert_eq!(trim_notification_body("你好世界", 2), "你好...");
    }
}
