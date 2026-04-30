use crate::AppConfig;
use crate::config_store;

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    config_store::load_config_from_store().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    config_store::save_config_to_store(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_models(provider: String) -> Result<Vec<String>, String> {
    let models = match provider.as_str() {
        "modelscope" => vec![
            "Qwen/Qwen-Image".to_string(),
        ],
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
        "siliconflow" => vec![
            "Kwai-Kolors/Kolors".to_string(),
        ],
        _ => vec![],
    };
    Ok(models)
}
