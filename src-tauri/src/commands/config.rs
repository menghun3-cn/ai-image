use crate::config as app_config;
use crate::AppConfig;

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    app_config::load_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    app_config::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_models(provider: String) -> Result<Vec<String>, String> {
    let models = match provider.as_str() {
        "modelscope" => vec![
            "Qwen/Qwen-Image".to_string(),
            "damo/cv_diffusion_text-to-image".to_string(),
        ],
        "nvidia" => vec!["stable-diffusion-xl-1024-v1-0".to_string()],
        "gemini" => vec!["gemini-2.0-flash-exp-image-generation".to_string()],
        "openrouter" => vec![
            "bytedance-seed/seedream-4.5".to_string(),
            "openai/gpt-image-1".to_string(),
        ],
        "openai" => vec!["gpt-image-1".to_string(), "dall-e-3".to_string()],
        "siliconflow" => vec![
            "Kwai-Kolors/Kolors".to_string(),
            "stabilityai/stable-diffusion-3-5-large".to_string(),
        ],
        _ => vec![],
    };
    Ok(models)
}
