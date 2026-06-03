use crate::config_store;
use crate::providers::{create_provider, ImageProvider};
use crate::types::GenerationOptions;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchGenerationOptions {
    pub prompts: Vec<String>,
    pub provider: String,
    pub model: Option<String>,
    pub output_dir: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchGenerationResult {
    pub total: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub results: Vec<SingleGenerationResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SingleGenerationResult {
    pub index: usize,
    pub prompt: String,
    pub success: bool,
    pub image_path: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn batch_generate_images(
    options: BatchGenerationOptions,
    window: tauri::Window,
) -> Result<BatchGenerationResult, String> {
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

    let total = options.prompts.len();
    let mut results = Vec::with_capacity(total);
    let mut success_count = 0;
    let mut failed_count = 0;

    for (index, prompt) in options.prompts.iter().enumerate() {
        // 发送进度更新
        let _ = window.emit(
            "batch-progress",
            serde_json::json!({
                "current": index + 1,
                "total": total,
                "prompt": prompt,
                "status": "generating"
            }),
        );

        let start_time = std::time::Instant::now();

        let gen_options = GenerationOptions {
            prompt: prompt.clone(),
            provider: options.provider.clone(),
            model: options.model.clone(),
            output_dir: options.output_dir.clone(),
            width: options.width,
            height: options.height,
            steps: None,
            guidance_scale: None,
            seed: None,
        };

        // 重试机制
        let mut last_error = None;
        let mut image_path = None;
        let mut success = false;

        for attempt in 0..=1 {
            match provider.generate(&gen_options).await {
                Ok(result) if result.success => {
                    image_path = result.image_path;
                    success = true;
                    break;
                }
                Ok(result) => {
                    last_error = result.error;
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }

            if attempt < 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if success {
            success_count += 1;
        } else {
            failed_count += 1;
        }

        let single_result = SingleGenerationResult {
            index,
            prompt: prompt.clone(),
            success,
            image_path,
            error: last_error,
            duration_ms,
        };

        // 发送单个结果更新
        let _ = window.emit(
            "batch-item-complete",
            serde_json::json!({
                "index": index,
                "total": total,
                "result": &single_result
            }),
        );

        results.push(single_result);
    }

    // 发送完成事件
    let _ = window.emit(
        "batch-complete",
        serde_json::json!({
            "total": total,
            "success_count": success_count,
            "failed_count": failed_count
        }),
    );

    Ok(BatchGenerationResult {
        total,
        success_count,
        failed_count,
        results,
    })
}
