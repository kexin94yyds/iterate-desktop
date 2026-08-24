use crate::config::{save_config, AppState};
use crate::constants::window;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSizeUpdate {
    pub width: f64,
    pub height: f64,
    pub fixed: bool,
}

#[tauri::command]
pub async fn apply_window_constraints(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let (window_config, always_on_top) = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        (
            config.ui_config.window_config.clone(),
            config.ui_config.always_on_top,
        )
    };

    if let Some(window) = app.get_webview_window("main") {
        // 设置窗口约束
        if let Err(e) = window.set_min_size(Some(tauri::LogicalSize::new(
            window_config.min_width,
            window_config.min_height,
        ))) {
            return Err(format!("设置最小窗口大小失败: {}", e));
        }

        if let Err(e) = window.set_max_size(Some(tauri::LogicalSize::new(
            window_config.max_width,
            window_config.max_height,
        ))) {
            return Err(format!("设置最大窗口大小失败: {}", e));
        }

        // 如果启用了自动调整大小，设置为合适的初始大小
        if window_config.auto_resize {
            let initial_width = window_config.min_width;
            let initial_height = (window_config.min_height + window_config.max_height) / 2.0;

            if let Err(e) = window.set_size(tauri::LogicalSize::new(initial_width, initial_height))
            {
                return Err(format!("设置窗口大小失败: {}", e));
            }
        }

        // 确保置顶状态在应用窗口约束后仍然有效
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            if let Err(e) = window.set_always_on_top(always_on_top) {
                log::warn!("应用窗口约束后重新设置置顶状态失败: {}", e);
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn update_window_size(
    size_update: WindowSizeUpdate,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 更新配置
    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;

        // 更新模式设置
        config.ui_config.window_config.fixed = size_update.fixed;

        // 更新当前模式的尺寸
        config
            .ui_config
            .window_config
            .update_current_size(size_update.width, size_update.height);

        if size_update.fixed {
            // 固定模式：设置最大和最小尺寸为相同值
            config.ui_config.window_config.max_width = size_update.width;
            config.ui_config.window_config.max_height = size_update.height;
            config.ui_config.window_config.min_width = size_update.width;
            config.ui_config.window_config.min_height = size_update.height;
            config.ui_config.window_config.auto_resize = false;
        } else {
            // 自由拉伸模式：设置合理的最小值和限制的最大值
            config.ui_config.window_config.min_width = window::MIN_WIDTH;
            config.ui_config.window_config.min_height = window::MIN_HEIGHT;
            config.ui_config.window_config.max_width = window::MAX_WIDTH;
            config.ui_config.window_config.max_height = window::MAX_HEIGHT;
            config.ui_config.window_config.auto_resize = window::DEFAULT_AUTO_RESIZE;
        }
    }

    // 保存配置
    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

    // 获取置顶状态
    let always_on_top = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.ui_config.always_on_top
    };

    // 应用到当前窗口
    if let Some(window) = app.get_webview_window("main") {
        if size_update.fixed {
            // 固定模式：设置精确的窗口大小和约束
            if let Err(e) = window.set_size(tauri::LogicalSize::new(
                size_update.width,
                size_update.height,
            )) {
                return Err(format!("设置窗口大小失败: {}", e));
            }

            if let Err(e) = window.set_min_size(Some(tauri::LogicalSize::new(
                size_update.width,
                size_update.height,
            ))) {
                return Err(format!("设置最小窗口大小失败: {}", e));
            }

            if let Err(e) = window.set_max_size(Some(tauri::LogicalSize::new(
                size_update.width,
                size_update.height,
            ))) {
                return Err(format!("设置最大窗口大小失败: {}", e));
            }

            log::debug!(
                "窗口已设置为固定大小: {}x{}",
                size_update.width,
                size_update.height
            );
        } else {
            // 自由拉伸模式：设置合理的约束范围
            if let Err(e) = window.set_min_size(Some(tauri::LogicalSize::new(
                window::MIN_WIDTH,
                window::MIN_HEIGHT,
            ))) {
                return Err(format!("设置最小窗口大小失败: {}", e));
            }

            if let Err(e) = window.set_max_size(Some(tauri::LogicalSize::new(
                window::MAX_WIDTH,
                window::MAX_HEIGHT,
            ))) {
                return Err(format!("设置最大窗口大小失败: {}", e));
            }

            // 设置为默认大小
            if let Err(e) = window.set_size(tauri::LogicalSize::new(
                size_update.width,
                size_update.height,
            )) {
                return Err(format!("设置窗口大小失败: {}", e));
            }

            log::debug!(
                "窗口已设置为自由拉伸模式，默认大小: {}x{}",
                size_update.width,
                size_update.height
            );
        }

        // 重新应用置顶状态，确保窗口大小变更不会影响置顶设置
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            if let Err(e) = window.set_always_on_top(always_on_top) {
                log::warn!("重新应用置顶状态失败: {}", e);
            } else {
                log::debug!("置顶状态已重新应用: {}", always_on_top);
            }
        }
    }

    Ok(())
}
