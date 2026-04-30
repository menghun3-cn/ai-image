use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use std::path::Path;

pub struct GeminiProvider {
    config: ProviderConfig,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "gemini-2.0-flash-exp-image-generation".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("gemini-2.0-flash-exp-image-generation");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        let request_body = serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": &options.prompt
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": 0,
                "responseModalities": ["TEXT", "IMAGE"]
            }
        });

        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model);
        crate::log_message(&format!("[Gemini] 请求接口: POST {}", url));
        crate::log_message(&format!("[Gemini] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[Gemini] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProviderError::Network(e))?;

        let response = client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.config.api_key
            ))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: format!("Gemini API Error: {} - {}", status, &text[..text.len().min(200)]),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // 解析响应中的图片数据
        let parts = result
            .get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .ok_or_else(|| ProviderError::InvalidResponse("响应格式无效".to_string()))?;

        for part in parts {
            if let Some(inline_data) = part.get("inlineData") {
                if let Some(mime_type) = inline_data.get("mimeType").and_then(|m| m.as_str()) {
                    if mime_type.starts_with("image/") {
                        let base64_data = inline_data
                            .get("data")
                            .and_then(|d| d.as_str())
                            .ok_or_else(|| ProviderError::InvalidResponse("图片数据格式无效".to_string()))?;

                        // 解码 base64 图片数据
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

                        return Ok(GenerationResult {
                            success: true,
                            image_path: Some(image_path),
                            error: None,
                            retries: None,
                        });
                    }
                }
            }
        }

        Err(ProviderError::InvalidResponse("响应中未找到图片数据".to_string()))
    }
}
