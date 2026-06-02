use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use std::path::Path;

pub struct AgnesProvider {
    config: ProviderConfig,
}

impl AgnesProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for AgnesProvider {
    fn name(&self) -> &'static str {
        "agnes"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "agnes-image-2.1-flash".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("agnes-image-2.1-flash");

        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        let size = if options.width > 0 && options.height > 0 {
            format!("{}x{}", options.width, options.height)
        } else {
            "1024x1024".to_string()
        };

        let request_body = serde_json::json!({
            "model": model,
            "prompt": &options.prompt,
            "n": 1,
            "size": size,
        });

        let endpoint = if self.config.endpoint.is_empty() {
            "https://apihub.agnes-ai.com/v1/images/generations".to_string()
        } else {
            format!("{}/images/generations", self.config.endpoint.trim_end_matches('/'))
        };

        crate::log_message(&format!("[Agnes] 请求接口: POST {}", endpoint));
        crate::log_message(&format!("[Agnes] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[Agnes] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Network(e))?;

        let response = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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
                message: format!("Agnes API Error: {} - {}", status, &text[..text.len().min(200)]),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let output_path = Path::new(&options.output_dir);
        if !output_path.exists() {
            tokio::fs::create_dir_all(output_path)
                .await
                .map_err(|e| ProviderError::FileSystem(e))?;
        }

        let image_path = crate::get_next_image_path(&options.output_dir)?;

        let image_data = if let Some(b64_json) = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("b64_json"))
            .and_then(|b| b.as_str()) {
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                b64_json
            ).map_err(|e| ProviderError::InvalidResponse(format!("Base64 解码失败: {}", e)))?
        } else if let Some(url) = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("url"))
            .and_then(|u| u.as_str()) {
            let image_response = client
                .get(url)
                .send()
                .await
                .map_err(ProviderError::Network)?;

            if !image_response.status().is_success() {
                return Err(ProviderError::Api {
                    status: image_response.status().as_u16(),
                    message: "下载图片失败".to_string(),
                });
            }

            image_response
                .bytes()
                .await
                .map_err(|e| ProviderError::Network(e))?
                .to_vec()
        } else {
            return Err(ProviderError::InvalidResponse("响应中未找到图片数据".to_string()));
        };

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