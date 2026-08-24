use chrono::{Local, SecondsFormat};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn timeline_debug_log_path() -> PathBuf {
    if let Some(home_dir) = dirs::home_dir() {
        return home_dir
            .join("Library")
            .join("Logs")
            .join("iterate")
            .join("timeline-debug.log");
    }

    std::env::temp_dir()
        .join("iterate")
        .join("timeline-debug.log")
}

pub fn append_timeline_debug_log(location: &str, key_params: Value) {
    let timestamp = Local::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let payload_text = serde_json::to_string(&key_params)
        .unwrap_or_else(|_| "{\"serialize_error\":true}".to_string());
    let line = format!("{} | {} | {}\n", timestamp, location, payload_text);
    let file_path = timeline_debug_log_path();

    if let Some(parent_dir) = file_path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(file_path) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}
