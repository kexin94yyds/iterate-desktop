use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tempfile::NamedTempFile;

use super::audio_assets::{get_audio_asset_manager, AudioSource};
use crate::config::{save_config, AppState};
use crate::constants::audio::{
    is_safe_audio_config_reference, is_supported_audio_format, is_valid_audio_file_size,
    managed_audio_reference, SUPPORTED_FORMATS,
};
use crate::log_important;

// 音频播放控制器 - 只存储控制信号，不存储音频流
pub struct AudioController {
    pub should_stop: Arc<AtomicBool>,
}

#[tauri::command]
pub async fn get_audio_notification_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取配置失败: {}", e))?;
    Ok(config.audio_config.notification_enabled)
}

#[tauri::command]
pub async fn set_audio_notification_enabled(
    enabled: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 如果是首次启用音频通知，先复制音频文件
    if enabled {
        if let Err(e) = ensure_audio_file_exists(&app).await {
            return Err(format!("准备音频文件失败: {}", e));
        }
    }

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.audio_config.notification_enabled = enabled;
    }

    // 保存配置到文件
    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_audio_url(state: State<'_, AppState>) -> Result<String, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取配置失败: {}", e))?;
    if is_safe_audio_config_reference(&config.audio_config.custom_url) {
        Ok(config.audio_config.custom_url.clone())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub async fn set_audio_url(
    url: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    validate_audio_reference(&app, &url)?;

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.audio_config.custom_url = url;
    }

    // 保存配置到文件
    save_config(&state, &app)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CustomAudioImportResult {
    pub reference: String,
}

#[tauri::command]
pub async fn import_custom_audio(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<CustomAudioImportResult>, String> {
    let selected = app
        .dialog()
        .file()
        .set_title("选择自定义提示音")
        .add_filter("音频文件", SUPPORTED_FORMATS)
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let source_path = selected
        .into_path()
        .map_err(|_| "无法读取所选音频文件".to_string())?;
    let reference = store_managed_custom_audio(&app, &source_path)?;

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.audio_config.custom_url = reference.clone();
    }
    save_config(&state, &app)
        .await
        .map_err(|_| "保存自定义提示音配置失败".to_string())?;

    Ok(Some(CustomAudioImportResult { reference }))
}

#[tauri::command]
pub async fn play_notification_sound(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 检查是否启用音频通知
    let (enabled, audio_url) = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        (
            config.audio_config.notification_enabled,
            config.audio_config.custom_url.clone(),
        )
    };

    if !enabled {
        return Ok(());
    }

    if audio_url.is_empty() {
        let has_embedded_audio = get_audio_asset_manager()
            .lock()
            .map(|manager| !manager.get_all_assets().is_empty())
            .unwrap_or(false);
        if !has_embedded_audio {
            return Ok(());
        }
    }

    // 异步播放音频，避免阻塞主线程
    tokio::spawn(async move {
        if play_audio_file(&app, &audio_url).await.is_err() {
            log_important!(warn, "播放音频失败，已静音回退");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn test_audio_sound(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 获取当前配置的音效URL
    let audio_url = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取配置失败: {}", e))?;
        config.audio_config.custom_url.clone()
    };

    // 同步测试音频播放，确保能捕获错误
    match play_audio_file(&app, &audio_url).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("音效测试失败: {}", e)),
    }
}

#[tauri::command]
pub async fn stop_audio_sound(app: tauri::AppHandle) -> Result<(), String> {
    // 设置停止信号
    if let Some(audio_controller) = app.try_state::<AudioController>() {
        audio_controller.should_stop.store(true, Ordering::Relaxed);
    }
    Ok(())
}

pub async fn play_audio_file(app: &AppHandle, audio_url: &str) -> Result<()> {
    // 重置停止信号
    if let Some(audio_controller) = app.try_state::<AudioController>() {
        audio_controller.should_stop.store(false, Ordering::Relaxed);
    }

    let audio_source = {
        let manager = get_audio_asset_manager();
        let manager = manager
            .lock()
            .map_err(|e| anyhow::anyhow!("获取管理器锁失败: {}", e))?;
        manager.parse_audio_url(app, audio_url)?
    };

    match audio_source {
        AudioSource::File(path) => {
            // 本地文件路径
            if path.exists() {
                let app_handle = app.clone();
                tokio::task::spawn_blocking(move || {
                    play_audio_sync_with_controller(&path, &app_handle)
                })
                .await
                .map_err(|e| anyhow::anyhow!("音频播放任务失败: {}", e))?
            } else {
                Err(anyhow::anyhow!("音频文件不存在: {:?}", path))
            }
        }
        AudioSource::Asset(asset_id) => {
            // 内置音频资源
            let audio_path = {
                let manager = get_audio_asset_manager();
                let manager = manager
                    .lock()
                    .map_err(|e| anyhow::anyhow!("获取管理器锁失败: {}", e))?;
                manager.ensure_audio_exists(app, &asset_id)?
            };
            let app_handle = app.clone();
            tokio::task::spawn_blocking(move || {
                play_audio_sync_with_controller(&audio_path, &app_handle)
            })
            .await
            .map_err(|e| anyhow::anyhow!("音频播放任务失败: {}", e))?
        }
    }
}

fn play_audio_sync_with_controller(audio_path: &PathBuf, app: &AppHandle) -> Result<()> {
    // 创建音频输出流
    let (_stream, stream_handle) =
        OutputStream::try_default().map_err(|e| anyhow::anyhow!("创建音频输出流失败: {}", e))?;

    // 创建音频播放器
    let sink =
        Sink::try_new(&stream_handle).map_err(|e| anyhow::anyhow!("创建音频播放器失败: {}", e))?;

    // 读取音频文件
    let file =
        std::fs::File::open(audio_path).map_err(|e| anyhow::anyhow!("打开音频文件失败: {}", e))?;
    let buf_reader = BufReader::new(file);

    // 解码音频
    let source =
        Decoder::new(buf_reader).map_err(|e| anyhow::anyhow!("解码音频文件失败: {}", e))?;

    // 播放音频
    sink.append(source);

    // 检查停止信号并播放
    if let Some(audio_controller) = app.try_state::<AudioController>() {
        while !sink.empty() {
            if audio_controller.should_stop.load(Ordering::Relaxed) {
                sink.stop();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    } else {
        // 如果没有控制器，使用原来的方式
        sink.sleep_until_end();
    }

    Ok(())
}

/// 确保默认音频文件存在，如果不存在则从资源目录复制
pub async fn ensure_audio_file_exists(app: &AppHandle) -> Result<()> {
    let manager = get_audio_asset_manager();
    let manager = manager
        .lock()
        .map_err(|e| anyhow::anyhow!("获取管理器锁失败: {}", e))?;

    // 确保第一个可用的音频资源存在
    let all_assets = manager.get_all_assets();
    if let Some(first_asset) = all_assets.first() {
        manager.ensure_audio_exists(app, &first_asset.id)?;
    }

    Ok(())
}

fn validate_audio_reference(app: &AppHandle, reference: &str) -> Result<(), String> {
    if !is_safe_audio_config_reference(reference) {
        return Err("只允许内置音效或已导入的本地提示音".to_string());
    }
    if reference.is_empty() {
        return Ok(());
    }

    let source = get_audio_asset_manager()
        .lock()
        .map_err(|_| "音频资源暂不可用".to_string())?
        .parse_audio_url(app, reference)
        .map_err(|_| "音频来源无效或不可用".to_string())?;
    if let AudioSource::File(path) = source {
        if !path.is_file() {
            return Err("已导入的提示音文件不存在".to_string());
        }
    }
    Ok(())
}

fn store_managed_custom_audio(app: &AppHandle, source_path: &Path) -> Result<String, String> {
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| is_supported_audio_format(value))
        .ok_or_else(|| "不支持该音频格式".to_string())?;

    let metadata = fs::metadata(source_path).map_err(|_| "无法读取所选音频文件".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err("所选内容不是有效的音频文件".to_string());
    }
    if !is_valid_audio_file_size(metadata.len()) {
        return Err("音频文件不能超过 10MB".to_string());
    }

    let validation_file =
        File::open(source_path).map_err(|_| "无法读取所选音频文件".to_string())?;
    Decoder::new(BufReader::new(validation_file)).map_err(|_| "音频文件无法解码".to_string())?;

    let reference =
        managed_audio_reference(&extension).ok_or_else(|| "不支持该音频格式".to_string())?;
    let filename = crate::constants::audio::managed_audio_filename(&reference)
        .ok_or_else(|| "无法创建受管音频标识".to_string())?;
    let audio_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "无法获取应用数据目录".to_string())?
        .join("audio");
    fs::create_dir_all(&audio_dir).map_err(|_| "无法创建提示音存储目录".to_string())?;

    let mut source = File::open(source_path).map_err(|_| "无法读取所选音频文件".to_string())?;
    let mut temporary =
        NamedTempFile::new_in(&audio_dir).map_err(|_| "无法准备提示音文件".to_string())?;
    std::io::copy(&mut source, &mut temporary).map_err(|_| "无法复制提示音文件".to_string())?;
    temporary
        .flush()
        .map_err(|_| "无法写入提示音文件".to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| "无法写入提示音文件".to_string())?;
    temporary
        .persist(audio_dir.join(filename))
        .map_err(|_| "无法保存提示音文件".to_string())?;

    for old_extension in SUPPORTED_FORMATS {
        if *old_extension == extension {
            continue;
        }
        let old_filename = format!(
            "{}.{}",
            crate::constants::audio::MANAGED_AUDIO_BASENAME,
            old_extension
        );
        let _ = fs::remove_file(audio_dir.join(old_filename));
    }

    Ok(reference)
}

pub async fn migrate_legacy_custom_audio(
    state: &State<'_, AppState>,
    app: &AppHandle,
) -> Result<(), String> {
    let legacy_source = {
        let config = state
            .config
            .lock()
            .map_err(|_| "无法读取音频配置".to_string())?;
        config.audio_config.custom_url.clone()
    };
    if is_safe_audio_config_reference(&legacy_source) {
        return Ok(());
    }

    let migrated_reference = if Path::new(&legacy_source).is_file() {
        store_managed_custom_audio(app, Path::new(&legacy_source)).ok()
    } else {
        None
    };
    {
        let mut config = state
            .config
            .lock()
            .map_err(|_| "无法更新音频配置".to_string())?;
        config.audio_config.custom_url = migrated_reference.unwrap_or_default();
        if config.audio_config.custom_url.is_empty() {
            config.audio_config.notification_enabled = false;
        }
    }
    save_config(state, app)
        .await
        .map_err(|_| "无法保存迁移后的音频配置".to_string())?;
    Ok(())
}
