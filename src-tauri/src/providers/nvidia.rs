use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use async_trait::async_trait;
use std::path::Path;

pub struct NvidiaProvider {
    config: ProviderConfig,
}

impl NvidiaProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }
}

#[async_trait]
impl ImageProvider for NvidiaProvider {
    fn name(&self) -> &'static str {
        "nvidia"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "black-forest-labs/flux.2-klein-4b".to_string(),
            "black-forest-labs/flux.1-kontext-dev".to_string(),
            "black-forest-labs/flux.1-dev".to_string(),
            "black-forest-labs/flux.1-schnell".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options
            .model
            .as_deref()
            .unwrap_or("black-forest-labs/flux.2-klein-4b");

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 根据模型选择不同的参数格式
        // flux.2-klein-4b: width/height, steps=4, 无 cfg_scale
        // flux.1-dev: width/height, steps>=5, 有 cfg_scale
        // flux.1-schnell: width/height, steps=4, 无 cfg_scale (cfg_scale必须<=0)
        // flux.1-kontext-dev: aspect_ratio, cfg_scale, steps=30, 需要 image (图生图)
        let is_klein = model.contains("klein");
        let is_dev = model.contains("flux.1-dev");
        let is_schnell = model.contains("schnell");
        let is_kontext = model.contains("kontext");

        let request_body = if is_klein || is_dev || is_schnell {
            // flux.2-klein-4b / flux.1-dev / flux.1-schnell: 使用 width/height
            let width = if options.width > 0 {
                options.width
            } else {
                1024
            };
            let height = if options.height > 0 {
                options.height
            } else {
                1024
            };

            if is_schnell {
                // flux.1-schnell: 无 cfg_scale, steps=4
                let steps = options.steps.map(|v| v.clamp(1, 50)).unwrap_or(4);
                serde_json::json!({
                    "prompt": &options.prompt,
                    "width": width,
                    "height": height,
                    "seed": options.seed.unwrap_or(0),
                    "steps": steps,
                })
            } else if is_dev {
                // flux.1-dev: 有 cfg_scale, steps>=5
                let cfg_scale = options
                    .guidance_scale
                    .map(|v| v.clamp(1.0, 10.0))
                    .unwrap_or(3.5);
                let steps = options.steps.map(|v| v.clamp(5, 50)).unwrap_or(20);
                serde_json::json!({
                    "prompt": &options.prompt,
                    "width": width,
                    "height": height,
                    "cfg_scale": cfg_scale,
                    "seed": options.seed.unwrap_or(0),
                    "steps": steps,
                })
            } else {
                // flux.2-klein-4b: cfg_scale <= 1, steps=4
                let cfg_scale = options
                    .guidance_scale
                    .map(|v| v.clamp(0.0, 1.0))
                    .unwrap_or(1.0);
                let steps = options.steps.map(|v| v.clamp(1, 50)).unwrap_or(4);
                serde_json::json!({
                    "prompt": &options.prompt,
                    "width": width,
                    "height": height,
                    "cfg_scale": cfg_scale,
                    "seed": options.seed.unwrap_or(0),
                    "steps": steps,
                })
            }
        } else if is_kontext {
            // flux.1-kontext-dev: 图生图，需要 image 参数
            // 暂时不支持，返回错误
            return Err(ProviderError::InvalidResponse(
                "flux.1-kontext-dev 模型需要输入图片，当前仅支持文生图".to_string(),
            ));
        } else {
            // 默认使用 width/height 格式
            let width = if options.width > 0 {
                options.width
            } else {
                1024
            };
            let height = if options.height > 0 {
                options.height
            } else {
                1024
            };
            let steps = options.steps.map(|v| v.clamp(1, 50)).unwrap_or(4);
            serde_json::json!({
                "prompt": &options.prompt,
                "width": width,
                "height": height,
                "seed": options.seed.unwrap_or(0),
                "steps": steps,
            })
        };

        let url = format!("https://ai.api.nvidia.com/v1/genai/{}", model);
        crate::log_message(&format!("[NVIDIA] 请求接口: POST {}", url));
        crate::log_message(&format!(
            "[NVIDIA] 请求体: {}",
            serde_json::to_string(&request_body).unwrap_or_default()
        ));
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
                message: format!(
                    "NVIDIA API Error: {} - {}",
                    status,
                    &text[..text.len().min(200)]
                ),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        crate::log_message(&format!(
            "[NVIDIA] 响应内容: {}",
            serde_json::to_string(&result).unwrap_or_default()
        ));

        // 获取 base64 图片数据 - NVIDIA 响应格式: {"artifacts": [{"base64": "...", "finishReason": "SUCCESS", "seed": 123}]}
        let base64_data = result
            .get("artifacts")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|artifact| artifact.get("base64"))
            .and_then(|b| b.as_str())
            .ok_or_else(|| {
                ProviderError::InvalidResponse(format!(
                    "响应中不包含图片数据，实际响应: {:?}",
                    result
                ))
            })?;

        // 解码并保存图片
        let image_data =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
                .map_err(|e| ProviderError::InvalidResponse(format!("Base64 解码失败: {}", e)))?;

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
