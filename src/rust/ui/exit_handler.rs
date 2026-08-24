use crate::config::AppState;
use crate::log_important;
use tauri::{AppHandle, Manager};

/// 处理应用退出请求（从前端快捷键调用）
pub async fn handle_exit_request_internal(app_handle: AppHandle) -> Result<bool, String> {
    let state = app_handle.state::<AppState>();

    log_important!(info, "🔥 处理应用内退出请求");

    crate::ui::exit::handle_system_exit_request(
        state,
        &app_handle,
        false, // 非手动关闭
    )
    .await
}

/// 设置应用退出处理器（保留向后兼容性）
pub fn setup_exit_handlers(_app_handle: &AppHandle) -> Result<(), String> {
    log_important!(info, "✅ 应用退出处理器已设置（前端快捷键处理）");
    Ok(())
}

/// 清理退出处理器（空操作，保持接口一致性）
pub fn cleanup_exit_handlers(_app_handle: &AppHandle) {
    log_important!(info, "应用退出处理器无需清理");
}
