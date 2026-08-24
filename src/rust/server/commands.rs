//! 寸止端口监听服务的 Tauri 命令
//!
//! 提供启动/停止/状态查询等功能，集成到设置页

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::Mutex;

/// 全局服务器进程状态
static SERVER_PROCESS: Lazy<Mutex<Option<ServerProcess>>> = Lazy::new(|| Mutex::new(None));

/// 服务器进程信息
struct ServerProcess {
    child: Child,
    port: u16,
}

/// 服务器状态响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CunzhiServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub active_ports: Vec<u16>,
    pub error: Option<String>,
}

/// 获取 cunzhi-server.py 脚本路径
fn get_server_script_path() -> Option<std::path::PathBuf> {
    // 优先查找 bin 目录
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

    // 尝试多个可能的位置
    let candidates = [
        exe_dir.join("cunzhi-server.py"),
        exe_dir.join("../bin/cunzhi-server.py"),
        dirs::home_dir()?.join("cunzhi/bin/cunzhi-server.py"),
        dirs::home_dir()?.join("bin/cunzhi-server.py"),
    ];

    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// 获取活跃端口列表
fn get_active_ports() -> Vec<u16> {
    let port_dir = match dirs::home_dir() {
        Some(home) => home.join(".cunzhi_ports"),
        None => return vec![],
    };

    if !port_dir.exists() {
        return vec![];
    }

    let mut ports = vec![];
    if let Ok(entries) = std::fs::read_dir(&port_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(port) = name.parse::<u16>() {
                    ports.push(port);
                }
            }
        }
    }
    ports.sort();
    ports
}

/// 检查端口是否可用
fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// 查找可用端口
fn find_available_port(start: u16) -> u16 {
    for port in start..start + 100 {
        if is_port_available(port) {
            return port;
        }
    }
    start
}

/// 清理端口文件
fn cleanup_port_file(port: u16) {
    if let Some(home) = dirs::home_dir() {
        let port_file = home.join(".cunzhi_ports").join(port.to_string());
        let _ = std::fs::remove_file(port_file);
    }
}

/// 启动寸止端口监听服务
#[tauri::command]
pub async fn start_cunzhi_server(port: Option<u16>) -> Result<CunzhiServerStatus, String> {
    let mut server = SERVER_PROCESS
        .lock()
        .map_err(|e| format!("锁定失败: {}", e))?;

    // 如果已经在运行，返回当前状态
    if let Some(ref mut proc) = *server {
        // 检查进程是否还活着
        match proc.child.try_wait() {
            Ok(Some(_)) => {
                // 进程已退出，清理
                cleanup_port_file(proc.port);
                *server = None;
            }
            Ok(None) => {
                // 进程还在运行
                return Ok(CunzhiServerStatus {
                    running: true,
                    port: Some(proc.port),
                    pid: Some(proc.child.id()),
                    active_ports: get_active_ports(),
                    error: None,
                });
            }
            Err(_) => {
                *server = None;
            }
        }
    }

    // 确定端口
    let target_port = port.unwrap_or(5321);
    let actual_port = if is_port_available(target_port) {
        target_port
    } else {
        find_available_port(target_port + 1)
    };

    // 获取脚本路径
    let script_path =
        get_server_script_path().ok_or_else(|| "找不到 cunzhi-server.py 脚本".to_string())?;

    // 启动服务器进程
    let child = Command::new("python3")
        .arg(&script_path)
        .arg(actual_port.to_string())
        .spawn()
        .map_err(|e| format!("启动服务器失败: {}", e))?;

    let pid = child.id();

    *server = Some(ServerProcess {
        child,
        port: actual_port,
    });

    // 等待服务器启动
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(CunzhiServerStatus {
        running: true,
        port: Some(actual_port),
        pid: Some(pid),
        active_ports: get_active_ports(),
        error: if actual_port != target_port {
            Some(format!(
                "端口 {} 被占用，已切换到 {}",
                target_port, actual_port
            ))
        } else {
            None
        },
    })
}

/// 停止寸止端口监听服务
#[tauri::command]
pub async fn stop_cunzhi_server() -> Result<CunzhiServerStatus, String> {
    let mut server = SERVER_PROCESS
        .lock()
        .map_err(|e| format!("锁定失败: {}", e))?;

    if let Some(mut proc) = server.take() {
        let port = proc.port;
        let _ = proc.child.kill();
        let _ = proc.child.wait();
        cleanup_port_file(port);
    }

    Ok(CunzhiServerStatus {
        running: false,
        port: None,
        pid: None,
        active_ports: get_active_ports(),
        error: None,
    })
}

/// 获取寸止端口监听服务状态
#[tauri::command]
pub async fn get_cunzhi_server_status() -> Result<CunzhiServerStatus, String> {
    let mut server = SERVER_PROCESS
        .lock()
        .map_err(|e| format!("锁定失败: {}", e))?;

    if let Some(ref mut proc) = *server {
        // 检查进程是否还活着
        match proc.child.try_wait() {
            Ok(Some(_)) => {
                // 进程已退出
                let port = proc.port;
                cleanup_port_file(port);
                *server = None;

                return Ok(CunzhiServerStatus {
                    running: false,
                    port: None,
                    pid: None,
                    active_ports: get_active_ports(),
                    error: Some("服务器已意外退出".to_string()),
                });
            }
            Ok(None) => {
                // 进程还在运行
                return Ok(CunzhiServerStatus {
                    running: true,
                    port: Some(proc.port),
                    pid: Some(proc.child.id()),
                    active_ports: get_active_ports(),
                    error: None,
                });
            }
            Err(e) => {
                return Ok(CunzhiServerStatus {
                    running: false,
                    port: None,
                    pid: None,
                    active_ports: get_active_ports(),
                    error: Some(format!("检查进程状态失败: {}", e)),
                });
            }
        }
    }

    Ok(CunzhiServerStatus {
        running: false,
        port: None,
        pid: None,
        active_ports: get_active_ports(),
        error: None,
    })
}

/// 清理残留端口文件
#[tauri::command]
pub async fn cleanup_cunzhi_ports() -> Result<Vec<u16>, String> {
    let port_dir = dirs::home_dir()
        .ok_or_else(|| "找不到 home 目录".to_string())?
        .join(".cunzhi_ports");

    if !port_dir.exists() {
        return Ok(vec![]);
    }

    let mut cleaned = vec![];

    if let Ok(entries) = std::fs::read_dir(&port_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(port) = name.parse::<u16>() {
                    // 检查端口是否真的在使用
                    if is_port_available(port) {
                        // 端口空闲，说明是残留文件，清理掉
                        let _ = std::fs::remove_file(entry.path());
                        cleaned.push(port);
                    }
                }
            }
        }
    }

    Ok(cleaned)
}

/// 检查端口是否可用
#[tauri::command]
pub async fn check_port_available(port: u16) -> Result<bool, String> {
    Ok(is_port_available(port))
}
