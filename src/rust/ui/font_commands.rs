use crate::config::{save_config, AppState, FontConfig};
use crate::constants::font;
use tauri::{AppHandle, State};

/// 字体配置信息
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FontInfo {
    pub font_family: String,
    pub font_size: String,
    pub custom_font_family: String,
}

/// 字体系列选项
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FontFamilyOption {
    pub id: String,
    pub name: String,
    pub css_value: String,
}

/// 字体大小选项
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FontSizeOption {
    pub id: String,
    pub name: String,
    pub scale: f32,
}

#[tauri::command]
pub async fn get_font_config(state: State<'_, AppState>) -> Result<FontInfo, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取配置失败: {}", e))?;

    Ok(FontInfo {
        font_family: config.ui_config.font_config.font_family.clone(),
        font_size: config.ui_config.font_config.font_size.clone(),
        custom_font_family: config.ui_config.font_config.custom_font_family.clone(),
    })
}

#[tauri::command]
pub async fn set_font_family(
    font_family: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.ui_config.font_config.font_family = font_family;
    }

    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_font_size(
    font_size: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.ui_config.font_config.font_size = font_size;
    }

    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_custom_font_family(
    custom_font_family: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.ui_config.font_config.custom_font_family = custom_font_family;
    }

    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_font_family_options() -> Result<Vec<FontFamilyOption>, String> {
    Ok(font::FONT_FAMILIES
        .iter()
        .map(|(id, name, css_value)| FontFamilyOption {
            id: id.to_string(),
            name: name.to_string(),
            css_value: css_value.to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn get_font_size_options() -> Result<Vec<FontSizeOption>, String> {
    Ok(font::FONT_SIZES
        .iter()
        .map(|(id, name, scale)| FontSizeOption {
            id: id.to_string(),
            name: name.to_string(),
            scale: *scale,
        })
        .collect())
}

#[tauri::command]
pub async fn reset_font_config(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.ui_config.font_config = FontConfig {
            font_family: font::DEFAULT_FONT_FAMILY.to_string(),
            font_size: font::DEFAULT_FONT_SIZE.to_string(),
            custom_font_family: font::DEFAULT_CUSTOM_FONT_FAMILY.to_string(),
        };
    }

    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(())
}
