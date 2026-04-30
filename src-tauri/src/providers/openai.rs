use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use std::path::Path;

pub struct OpenAiProvider {
    config: ProviderConfig,
}

impl OpenAiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "gpt-image-1".to_string(),
            "dall-e-3".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("gpt-image-1");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 构建尺寸参数
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

        crate::log_message(&format!("[OpenAI] 请求接口: POST https://api.openai.com/v1/images/generations"));
        crate::log_message(&format!("[OpenAI] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[OpenAI] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Network(e))?;

        let response = client
            .post("https://api.openai.com/v1/images/generations")
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
                message: format!("OpenAI API Error: {} - {}", status, &text[..text.len().min(200)]),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // 确保输出目录存在
        let output_path = Path::new(&options.output_dir);
        if !output_path.exists() {
            tokio::fs::create_dir_all(output_path)
                .await
                .map_err(|e| ProviderError::FileSystem(e))?;
        }

        // 生成文件名
        let image_path = crate::get_next_image_path(&options.output_dir)?;

        // 尝试从响应中获取图片数据
        let image_data = if let Some(b64_json) = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("b64_json"))
            .and_then(|b| b.as_str()) {
            // Base64 编码的图片数据
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
            // URL 格式的图片，需要下载
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
