use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct OpenRouterProvider {
    config: ProviderConfig,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "bytedance-seed/seedream-4.5".to_string(),
            "openai/gpt-image-1".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("bytedance-seed/seedream-4.5");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 构建请求体（与原项目保持一致）
        let mut request_body = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": &options.prompt
                }
            ],
            "modalities": ["image"],
            "temperature": 0,
        });

        // 添加 seed 参数（如果提供）
        if let Some(seed) = options.seed {
            request_body["seed"] = serde_json::json!(seed);
        }

        // 添加图片尺寸配置（如果提供）
        if options.width > 0 && options.height > 0 {
            // 计算宽高比
            let gcd = |mut a: u32, mut b: u32| -> u32 {
                while b != 0 {
                    let temp = b;
                    b = a % b;
                    a = temp;
                }
                a
            };
            let divisor = gcd(options.width as u32, options.height as u32);
            let ratio_w = options.width as u32 / divisor;
            let ratio_h = options.height as u32 / divisor;
            
            request_body["image_config"] = serde_json::json!({
                "aspect_ratio": format!("{}:{}", ratio_w, ratio_h)
            });
        }

        crate::log_message(&format!("[OpenRouter] 请求接口: POST {}", self.config.endpoint));
        crate::log_message(&format!("[OpenRouter] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[OpenRouter] API Key: {}", key_preview));

        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://ai-image.app")
            .header("X-Title", "AI Image Generator")
            .json(&request_body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: text,
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // 解析图片数据（与原项目一致，支持 message.images 格式）
        let message = result
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"));

        let mut image_bytes: Option<Vec<u8>> = None;

        // 尝试从 message.images 获取图片
        if let Some(images) = message.and_then(|msg| msg.get("images")).and_then(|v| v.as_array()) {
            for image in images {
                if let Some(image_url) = image
                    .get("image_url")
                    .and_then(|u| u.get("url"))
                    .and_then(|u| u.as_str())
                {
                    if image_url.starts_with("data:") {
                        // Base64 编码的图片
                        if let Some(base64_data) = image_url.split(',').nth(1) {
                            image_bytes = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                base64_data,
                            ).ok();
                        }
                    } else {
                        // URL 格式的图片
                        let image_response = client
                            .get(image_url)
                            .send()
                            .await
                            .map_err(ProviderError::Network)?;

                        if image_response.status().is_success() {
                            image_bytes = Some(
                                image_response
                                    .bytes()
                                    .await
                                    .map_err(ProviderError::Network)?
                                    .to_vec(),
                            );
                        }
                    }
                    break;
                }
            }
        }

        // 尝试从 content 数组获取图片（备用格式）
        if image_bytes.is_none() {
            if let Some(content_url) = message
                .and_then(|msg| msg.get("content"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("image_url"))
                .and_then(|img| img.get("url"))
                .and_then(|url| url.as_str())
            {
                let image_response = client
                    .get(content_url)
                    .send()
                    .await
                    .map_err(ProviderError::Network)?;

                if image_response.status().is_success() {
                    image_bytes = Some(
                        image_response
                            .bytes()
                            .await
                            .map_err(ProviderError::Network)?
                            .to_vec(),
                    );
                }
            }
        }

        let image_bytes = image_bytes.ok_or_else(|| {
            ProviderError::InvalidResponse("无法获取图片数据".to_string())
        })?;

        // 保存图片
        let output_dir = &options.output_dir;
        std::fs::create_dir_all(output_dir)
            .map_err(ProviderError::FileSystem)?;

        let filename = format!("{}.png", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        let image_path = Path::new(output_dir).join(&filename);

        std::fs::write(&image_path, &image_bytes)
            .map_err(ProviderError::FileSystem)?;

        Ok(GenerationResult {
            success: true,
            image_path: Some(image_path.to_string_lossy().to_string()),
            error: None,
            retries: None,
        })
    }
}
