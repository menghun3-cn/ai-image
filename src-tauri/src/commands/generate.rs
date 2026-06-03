use crate::config_store;
use crate::providers::{create_provider, ImageProvider};
use crate::types::{GenerationOptions, GenerationResult};

#[tauri::command]
pub async fn generate_image(options: GenerationOptions) -> Result<GenerationResult, String> {
    let config = config_store::load_config_from_store().map_err(|e| e.to_string())?;

    let provider_config = match options.provider.as_str() {
        "agnes" => config.providers.agnes,
        "modelscope" => config.providers.modelscope,
        "nvidia" => config.providers.nvidia,
        "gemini" => config.providers.gemini,
        "openrouter" => config.providers.openrouter,
        "openai" => config.providers.openai,
        "siliconflow" => config.providers.siliconflow,
        _ => return Err(format!("未知的提供商: {}", options.provider)),
    };

    let provider = create_provider(&options.provider, provider_config)
        .ok_or_else(|| format!("无法创建提供商: {}", options.provider))?;

    // 重试机制
    let mut last_result = GenerationResult {
        success: false,
        image_path: None,
        error: Some("未知错误".to_string()),
        retries: Some(0),
    };

    for attempt in 0..=1 {
        match provider.generate(&options).await {
            Ok(result) if result.success => {
                let final_result = GenerationResult {
                    success: true,
                    image_path: result.image_path.clone(),
                    error: None,
                    retries: Some(attempt),
                };
                // 打印返回结果到日志
                crate::log_message(&format!(
                    "[Generate Image] 返回结果: {}",
                    serde_json::to_string(&final_result).unwrap_or_default()
                ));
                return Ok(final_result);
            }
            Ok(result) => {
                last_result = GenerationResult {
                    success: false,
                    image_path: None,
                    error: result.error,
                    retries: Some(attempt),
                };
            }
            Err(e) => {
                last_result = GenerationResult {
                    success: false,
                    image_path: None,
                    error: Some(e.to_string()),
                    retries: Some(attempt),
                };
            }
        }

        if attempt < 1 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    // 打印最终失败结果到日志
    crate::log_message(&format!(
        "[Generate Image] 返回结果: {}",
        serde_json::to_string(&last_result).unwrap_or_default()
    ));
    Ok(last_result)
}
