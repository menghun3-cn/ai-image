use crate::providers::agnes::AgnesProvider;
use crate::types::{VideoGenerationOptions, VideoGenerationResult};
use crate::config_store;
use crate::ProviderConfig;
use tauri::AppHandle;
use serde_json::json;

#[tauri::command]
pub async fn generate_video(
    app: AppHandle,
    options: VideoGenerationOptions
) -> Result<VideoGenerationResult, String> {
    let config = config_store::load_config_from_store().map_err(|e| e.to_string())?;

    // 视频生成仅支持 Agnes Provider
    let provider_config = config.providers.agnes;

    if provider_config.api_key.is_empty() {
        return Err("Agnes API Key 未配置，请先在设置中配置".to_string());
    }

    let provider = AgnesProvider::new(provider_config);

    match provider.generate_video(&options, &app).await {
        Ok(result) if result.success => {
            Ok(VideoGenerationResult {
                success: true,
                video_path: result.video_path,
                error: None,
            })
        }
        Ok(result) => {
            Ok(VideoGenerationResult {
                success: false,
                video_path: None,
                error: result.error,
            })
        }
        Err(e) => {
            Ok(VideoGenerationResult {
                success: false,
                video_path: None,
                error: Some(e.to_string()),
            })
        }
    }
}

#[tauri::command]
pub async fn get_video_output_dir() -> Result<String, String> {
    let config = config_store::load_config_from_store().map_err(|e| e.to_string())?;
    
    // 视频默认保存到 video 目录
    let video_dir = if config.default_output_dir.is_empty() {
        "video".to_string()
    } else {
        std::path::Path::new(&config.default_output_dir)
            .parent()
            .map(|p| p.join("video").to_string_lossy().to_string())
            .unwrap_or_else(|| "video".to_string())
    };
    
    Ok(video_dir)
}
