use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use async_trait::async_trait;
use std::path::Path;

pub struct SiliconFlowProvider {
    config: ProviderConfig,
}

impl SiliconFlowProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for SiliconFlowProvider {
    fn name(&self) -> &'static str {
        "siliconflow"
    }

    fn list_models(&self) -> Vec<String> {
        vec!["Kwai-Kolors/Kolors".to_string()]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("Kwai-Kolors/Kolors");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 构建请求体
        let mut request_body = serde_json::json!({
            "model": model,
            "prompt": &options.prompt,
            "image_size": if options.width > 0 && options.height > 0 {
                format!("{}x{}", options.width, options.height)
            } else {
                "1024x1024".to_string()
            },
            "batch_size": 1,
        });

        // 添加可选参数
        if let Some(steps) = options.steps {
            request_body["num_inference_steps"] = serde_json::json!(steps);
        }
        if let Some(guidance_scale) = options.guidance_scale {
            request_body["guidance_scale"] = serde_json::json!(guidance_scale);
        }

        crate::log_message(&format!(
            "[SiliconFlow] 请求接口: POST https://api.siliconflow.cn/v1/images/generations"
        ));
        crate::log_message(&format!(
            "[SiliconFlow] 请求体: {}",
            serde_json::to_string(&request_body).unwrap_or_default()
        ));
        crate::log_message(&format!("[SiliconFlow] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Network(e))?;

        let response = client
            .post("https://api.siliconflow.cn/v1/images/generations")
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
                message: format!(
                    "SiliconFlow API Error: {} - {}",
                    status,
                    &text[..text.len().min(200)]
                ),
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

        // 尝试多种可能的响应格式获取图片 URL
        let image_url = if let Some(url) = result
            .get("images")
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("url"))
            .and_then(|u| u.as_str())
        {
            Some(url.to_string())
        } else if let Some(url) = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("url"))
            .and_then(|u| u.as_str())
        {
            Some(url.to_string())
        } else if let Some(url) = result
            .get("images")
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.as_str())
        {
            Some(url.to_string())
        } else {
            None
        };

        let image_url = image_url
            .ok_or_else(|| ProviderError::InvalidResponse("响应中未找到图片 URL".to_string()))?;

        // 下载图片
        let image_response = client
            .get(&image_url)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !image_response.status().is_success() {
            return Err(ProviderError::Api {
                status: image_response.status().as_u16(),
                message: "下载图片失败".to_string(),
            });
        }

        let image_data = image_response
            .bytes()
            .await
            .map_err(|e| ProviderError::Network(e))?
            .to_vec();

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
