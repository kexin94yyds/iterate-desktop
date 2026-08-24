use once_cell::sync::Lazy;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter, Runtime};

const UI_PTY_ENABLE_ENV: &str = "CUNZHI_ENABLE_UI_PTY";
const MAX_PTY_WRITE_BYTES: usize = 64 * 1024;
const MIN_PTY_ROWS: u16 = 1;
const MAX_PTY_ROWS: u16 = 200;
const MIN_PTY_COLS: u16 = 1;
const MAX_PTY_COLS: u16 = 500;

struct PtyState {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
}

static PTY_STATE: Lazy<Arc<Mutex<PtyState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(PtyState {
        master: None,
        writer: None,
    }))
});

#[derive(Serialize, Deserialize, Clone)]
pub struct TerminalPayload {
    pub data: String,
}

fn is_truthy_env_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn ui_pty_enabled() -> bool {
    std::env::var(UI_PTY_ENABLE_ENV)
        .map(|value| is_truthy_env_value(&value))
        .unwrap_or(false)
}

fn ensure_ui_pty_enabled() -> Result<(), String> {
    if ui_pty_enabled() {
        Ok(())
    } else {
        Err(format!(
            "内嵌 PTY 默认禁用；如需开发调试，请在启动前设置 {}=1",
            UI_PTY_ENABLE_ENV
        ))
    }
}

fn canonical_pty_cwd(cwd: Option<String>) -> Result<Option<PathBuf>, String> {
    let Some(path) = cwd else {
        return Ok(None);
    };
    if path.trim().is_empty() {
        return Ok(None);
    }

    let canonical = PathBuf::from(&path)
        .canonicalize()
        .map_err(|e| format!("PTY 工作目录不存在或不可访问 {}: {}", path, e))?;
    if !canonical.is_dir() {
        return Err(format!("PTY 工作目录不是目录: {}", path));
    }
    Ok(Some(canonical))
}

fn ensure_write_size(data: &str) -> Result<(), String> {
    let size = data.len();
    if size > MAX_PTY_WRITE_BYTES {
        return Err(format!(
            "PTY 单次写入过大（{} bytes），最大允许 {} bytes",
            size, MAX_PTY_WRITE_BYTES
        ));
    }
    Ok(())
}

fn ensure_pty_size(rows: u16, cols: u16) -> Result<(), String> {
    if !(MIN_PTY_ROWS..=MAX_PTY_ROWS).contains(&rows) {
        return Err(format!(
            "PTY rows 超出范围: {}，允许 {}..={}",
            rows, MIN_PTY_ROWS, MAX_PTY_ROWS
        ));
    }
    if !(MIN_PTY_COLS..=MAX_PTY_COLS).contains(&cols) {
        return Err(format!(
            "PTY cols 超出范围: {}，允许 {}..={}",
            cols, MIN_PTY_COLS, MAX_PTY_COLS
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn open_pty<R: Runtime>(app: AppHandle<R>, cwd: Option<String>) -> Result<(), String> {
    ensure_ui_pty_enabled()?;
    let cwd = canonical_pty_cwd(cwd)?;

    {
        let state = PTY_STATE.lock();
        if state.master.is_some() {
            return Ok(());
        }
    }

    let pty_system = native_pty_system();

    // 获取默认 Shell
    #[cfg(target_os = "macos")]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    #[cfg(target_os = "windows")]
    let shell = "powershell.exe".to_string();
    #[cfg(target_os = "linux")]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    #[cfg(target_os = "android")]
    let shell = "/system/bin/sh".to_string();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法开启 PTY: {}", e))?;

    let mut cmd = CommandBuilder::new(shell);
    if let Some(path) = cwd {
        cmd.cwd(path);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("子进程启动失败: {}", e))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("获取 Reader 失败: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("获取 Writer 失败: {}", e))?;

    {
        let mut state = PTY_STATE.lock();
        state.master = Some(pair.master);
        state.writer = Some(writer);
    }

    // 在独立线程中读取 PTY 输出
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = app.emit("terminal-data", TerminalPayload { data });
                }
                Err(_) => break,
            }
        }
        let _ = child.wait();
    });

    Ok(())
}

#[tauri::command]
pub async fn write_to_pty(data: String) -> Result<(), String> {
    ensure_ui_pty_enabled()?;
    ensure_write_size(&data)?;

    if let Some(writer) = PTY_STATE.lock().writer.as_mut() {
        writer
            .write_all(data.as_bytes())
            .map_err(|e| format!("写入 PTY 失败: {}", e))?;
        writer
            .flush()
            .map_err(|e| format!("Flush PTY 失败: {}", e))?;
    } else {
        return Err("PTY 尚未开启".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_pty(rows: u16, cols: u16) -> Result<(), String> {
    ensure_ui_pty_enabled()?;
    ensure_pty_size(rows, cols)?;

    if let Some(master) = PTY_STATE.lock().master.as_mut() {
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Resize PTY 失败: {}", e))?;
    } else {
        return Err("PTY 尚未开启".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_pty_cwd, ensure_pty_size, ensure_write_size, is_truthy_env_value,
        MAX_PTY_WRITE_BYTES,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_truthy_env_values() {
        assert!(is_truthy_env_value("1"));
        assert!(is_truthy_env_value("TRUE"));
        assert!(is_truthy_env_value(" yes "));
        assert!(is_truthy_env_value("on"));
        assert!(!is_truthy_env_value("0"));
        assert!(!is_truthy_env_value("false"));
    }

    #[test]
    fn canonical_pty_cwd_rejects_file_paths() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pty-cwd-guard-{unique}"));
        fs::create_dir_all(&root).expect("should create temp dir");
        let file_path = root.join("not-a-dir.txt");
        fs::write(&file_path, "not a dir").expect("should write temp file");

        let result = canonical_pty_cwd(Some(file_path.to_string_lossy().to_string()));

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("不是目录"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pty_write_size_rejects_large_input() {
        let data = "x".repeat(MAX_PTY_WRITE_BYTES + 1);

        let result = ensure_write_size(&data);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("单次写入过大"));
    }

    #[test]
    fn pty_resize_rejects_out_of_range_values() {
        assert!(ensure_pty_size(0, 80).is_err());
        assert!(ensure_pty_size(24, 0).is_err());
        assert!(ensure_pty_size(24, 80).is_ok());
    }
}
