use crate::agnes_models;
use crate::config_store;
use crate::AppConfig;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct DefaultStoragePaths {
    pub image_dir: String,
    pub video_dir: String,
    pub data_dir: String,
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    config_store::load_config_from_store().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_default_storage_paths() -> Result<DefaultStoragePaths, String> {
    let image_dir = config_store::get_default_image_output_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())?;
    
    let video_dir = config_store::get_default_video_output_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())?;
    
    let data_dir = config_store::get_default_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())?;
    
    Ok(DefaultStoragePaths {
        image_dir,
        video_dir,
        data_dir,
    })
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config_store::save_config_to_store(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle, default_path: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = app.dialog().file();
    
    if let Some(_path) = default_path {
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
    }
    
    let result = dialog.blocking_pick_folder();
    
    match result {
        Some(path) => {
            // FilePath 实现了 Display trait，可以直接转换为字符串
            Ok(Some(path.to_string()))
        },
        None => Ok(None),
    }
}

#[tauri::command]
pub fn get_provider_models(provider: String) -> Result<Vec<String>, String> {
    let models = match provider.as_str() {
        "agnes" => {
            // 从拉取的模型列表中获取文生图模型
            match agnes_models::load_agnes_models() {
                Ok(store) if !store.text_to_image.is_empty() => {
                    agnes_models::get_model_ids(&store.text_to_image)
                }
                _ => {
                    // 如果没有拉取到模型，返回默认模型
                    vec!["agnes-image-2.1-flash".to_string()]
                }
            }
        }
        "modelscope" => vec!["Qwen/Qwen-Image".to_string()],
        "nvidia" => vec![
            "black-forest-labs/flux.2-klein-4b".to_string(),
            "black-forest-labs/flux.1-kontext-dev".to_string(),
            "black-forest-labs/flux.1-dev".to_string(),
            "black-forest-labs/flux.1-schnell".to_string(),
        ],
        "gemini" => vec!["gemini-2.0-flash-exp-image-generation".to_string()],
        "openrouter" => vec![
            "bytedance-seed/seedream-4.5".to_string(),
            "openai/gpt-image-1".to_string(),
        ],
        "openai" => vec!["gpt-image-1".to_string(), "dall-e-3".to_string()],
        "siliconflow" => vec!["Kwai-Kolors/Kolors".to_string()],
        _ => vec![],
    };
    Ok(models)
}

/// 获取日志文件内容
#[tauri::command]
pub fn get_log_content() -> Result<String, String> {
    let log_dir = crate::get_log_dir();
    
    // 确保目录存在
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;
    }
    
    // 获取今天的日志文件
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_file = log_dir.join(format!("ai-image-v2-{}.log", today));
    
    // 如果今天的日志文件不存在，尝试找到最新的日志文件
    if !log_file.exists() {
        let mut log_files: Vec<_> = std::fs::read_dir(&log_dir)
            .map_err(|e| format!("读取日志目录失败: {}", e))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_str()
                    .map(|name| name.starts_with("ai-image-v2-") && name.ends_with(".log"))
                    .unwrap_or(false)
            })
            .collect();
        
        // 按修改时间排序
        log_files.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        
        if let Some(latest) = log_files.last() {
            let content = std::fs::read_to_string(latest.path())
                .map_err(|e| format!("读取日志文件失败: {}", e))?;
            return Ok(content);
        }
        
        return Ok("暂无日志内容".to_string());
    }
    
    let content = std::fs::read_to_string(&log_file)
        .map_err(|e| format!("读取日志文件失败: {}", e))?;
    
    Ok(content)
}

/// 打开日志目录
#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let log_dir = crate::get_log_dir();
    
    crate::log_message(&format!("[Config] 打开日志目录: {}", log_dir.display()));
    
    // 确保目录存在
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;
    }
    
    // 使用系统默认程序打开目录
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("打开日志目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("打开日志目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(log_dir)
            .spawn()
            .map_err(|e| format!("打开日志目录失败: {}", e))?;
    }
    
    Ok(())
}
