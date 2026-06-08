use crate::agnes_models;
use crate::config_store;
use crate::AppConfig;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    config_store::load_config_from_store().map_err(|e| e.to_string())
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
