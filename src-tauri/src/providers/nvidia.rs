use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use std::path::Path;

pub struct NvidiaProvider {
    config: ProviderConfig,
}

impl NvidiaProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 { a } else { Self::gcd(b, a % b) }
    }
}

#[async_trait]
impl ImageProvider for NvidiaProvider {
    fn name(&self) -> &'static str {
        "nvidia"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "stabilityai/stable-diffusion-3-medium".to_string(),
            "stabilityai/stable-diffusion-3_5-large".to_string(),
            "stabilityai/stable-diffusion-xl-1024-v1-0".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("stabilityai/stable-diffusion-xl-1024-v1-0");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 计算宽高比
        let aspect_ratio = if options.width > 0 && options.height > 0 {
            let divisor = Self::gcd(options.width, options.height);
            format!("{}:{}", options.width / divisor, options.height / divisor)
        } else {
            "1:1".to_string()
        };

        let request_body = serde_json::json!({
            "prompt": &options.prompt,
            "cfg_scale": options.guidance_scale.unwrap_or(5.0),
            "aspect_ratio": aspect_ratio,
            "seed": options.seed.unwrap_or(0),
            "steps": options.steps.unwrap_or(50),
            "temperature": 0,
            "negative_prompt": "",
        });

        let url = format!("https://ai.api.nvidia.com/v1/genai/{}", model);
        crate::log_message(&format!("[NVIDIA] 请求接口: POST {}", url));
        crate::log_message(&format!("[NVIDIA] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[NVIDIA] API Key: {}", key_preview));

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: format!("NVIDIA API Error: {} - {}", status, &text[..text.len().min(200)]),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // 获取 base64 图片数据
        let base64_data = result
            .get("image")
            .and_then(|i| i.as_str())
            .ok_or_else(|| ProviderError::InvalidResponse("响应中不包含图片数据".to_string()))?;

        // 解码并保存图片
        let image_data = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            base64_data
        ).map_err(|e| ProviderError::InvalidResponse(format!("Base64 解码失败: {}", e)))?;

        // 确保输出目录存在
        let output_path = Path::new(&options.output_dir);
        if !output_path.exists() {
            tokio::fs::create_dir_all(output_path)
                .await
                .map_err(|e| ProviderError::FileSystem(e))?;
        }

        // 生成文件名
        let image_path = crate::get_next_image_path(&options.output_dir)?;

        // 保存图片
        tokio::fs::write(&image_path, image_data)
            .await
            .map_err(|e| ProviderError::FileSystem(e))?;

        Ok(GenerationResult {
            success: true,
            image_path: Some(image_path),
            error: None,
            retries: None,
        })
    }
}
