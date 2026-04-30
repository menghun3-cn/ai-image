use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult};
use crate::ProviderConfig;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

pub struct ModelScopeProvider {
    config: ProviderConfig,
}

impl ModelScopeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImageProvider for ModelScopeProvider {
    fn name(&self) -> &'static str {
        "modelscope"
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "Qwen/Qwen-Image".to_string(),
            "damo/cv_diffusion_text-to-image".to_string(),
        ]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        let model = options.model.as_deref().unwrap_or("Qwen/Qwen-Image");
        
        crate::log_message(&format!("[ModelScope] 开始生成图片，模型: {}", model));

        // 检查 API Key
        if self.config.api_key.is_empty() {
            crate::log_message("[ModelScope] API Key 未配置");
            return Err(ProviderError::Api {
                status: 401,
                message: "ModelScope API Key 未配置，请在设置中配置".to_string(),
            });
        }

        // 脱敏显示 API Key 前15位
        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 构建请求体，根据宽高计算 size 参数
        let size = if options.width > 0 && options.height > 0 {
            format!("{}x{}", options.width, options.height)
        } else {
            "1024x1024".to_string()
        };

        let request_body = serde_json::json!({
            "model": model,
            "prompt": &options.prompt,
            "size": size,
            "temperature": 0,
        });

        let client = reqwest::Client::new();
        
        crate::log_message(&format!("[ModelScope] 请求接口: POST https://api-inference.modelscope.cn/v1/images/generations"));
        crate::log_message(&format!("[ModelScope] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[ModelScope] API Key: {}", key_preview));
        
        let response = client
            .post("https://api-inference.modelscope.cn/v1/images/generations")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("X-ModelScope-Async-Mode", "true")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                crate::log_message(&format!("[ModelScope] 请求发送失败: {}", e));
                ProviderError::Network(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            crate::log_message(&format!("[ModelScope] 请求失败: status={}, body={}", status, text));
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: format!("请求失败: {}", text),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| {
                crate::log_message(&format!("[ModelScope] 解析响应失败: {}", e));
                ProviderError::InvalidResponse(e.to_string())
            })?;

        // 检查响应中是否有错误信息
        if let Some(error_msg) = result.get("error").and_then(|e| e.as_str()) {
            crate::log_message(&format!("[ModelScope] API 返回错误: {}", error_msg));
            return Err(ProviderError::Api {
                status: 400,
                message: format!("ModelScope API 错误: {}", error_msg),
            });
        }

        // 检查是否有直接返回的 URL
        if let Some(url) = result
            .get("output")
            .and_then(|o| o.get("results"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("url"))
            .and_then(|u| u.as_str())
        {
            crate::log_message("[ModelScope] 直接返回图片 URL");
            return self.download_and_save(url, &options.output_dir).await;
        }

        // 异步模式，获取 task_id
        let task_id = result
            .get("task_id")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                crate::log_message(&format!("[ModelScope] 无法获取 task_id，响应: {:?}", result));
                ProviderError::InvalidResponse("无法获取 task_id".to_string())
            })?;

        crate::log_message(&format!("[ModelScope] 异步任务创建成功，task_id: {}", task_id));

        // 轮询获取结果 - 增加超时时间和调试日志
        let max_attempts = 120; // 增加到 120 次 (6分钟)
        let poll_interval = Duration::from_secs(3);
        let mut unknown_count = 0; // 记录连续 UNKNOWN 状态次数
        let max_unknown_count = 10; // 最多允许连续 10 次 UNKNOWN

        for attempt in 0..max_attempts {
            sleep(poll_interval).await;

            crate::log_message(&format!("[ModelScope] 第 {}/{} 次轮询...", attempt + 1, max_attempts));

            let poll_response = client
                .get(format!(
                    "https://api-inference.modelscope.cn/v1/tasks/{}",
                    task_id
                ))
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("X-ModelScope-Task-Type", "image_generation")
                .send()
                .await;

            let poll_response = match poll_response {
                Ok(r) => r,
                Err(e) => {
                    crate::log_message(&format!("[ModelScope] 轮询请求失败: {}", e));
                    continue;
                }
            };

            if !poll_response.status().is_success() {
                let status = poll_response.status();
                let text = poll_response.text().await.unwrap_or_default();
                crate::log_message(&format!("[ModelScope] 轮询响应异常: status={}, body={}", status, text));

                // 404 错误表示任务不存在或接口错误，不应继续轮询
                if status.as_u16() == 404 {
                    return Err(ProviderError::Api {
                        status: 404,
                        message: format!("轮询接口返回 404，任务可能不存在或接口地址错误: {}", text),
                    });
                }

                // 401/403 认证错误，不应继续轮询
                if status.as_u16() == 401 || status.as_u16() == 403 {
                    return Err(ProviderError::Api {
                        status: status.as_u16(),
                        message: format!("API Key 无效或权限不足: {}", text),
                    });
                }

                // 其他错误继续轮询（可能是临时网络问题）
                continue;
            }

            let poll_result: serde_json::Value = match poll_response.json().await {
                Ok(r) => r,
                Err(e) => {
                    crate::log_message(&format!("[ModelScope] 轮询响应解析失败: {}", e));
                    continue;
                }
            };

            // 先从根级别获取 task_status，如果不存在再从 output 中获取
            let status = poll_result
                .get("task_status")
                .and_then(|s| s.as_str())
                .or_else(|| {
                    poll_result
                        .get("output")
                        .and_then(|o| o.get("task_status"))
                        .and_then(|s| s.as_str())
                })
                .unwrap_or("UNKNOWN");

            crate::log_message(&format!("[ModelScope] 任务状态: {}", status));

            match status {
                "SUCCEEDED" | "SUCCEED" => {
                    crate::log_message("[ModelScope] 任务完成，获取图片 URL...");
                    
                    // 尝试多种路径获取图片 URL（与原项目一致）
                    let image_url = poll_result
                        .get("output_images")
                        .and_then(|o| o.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|url| url.as_str())
                        .or_else(|| {
                            poll_result
                                .get("output")
                                .and_then(|o| o.get("results"))
                                .and_then(|r| r.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|item| item.get("url"))
                                .and_then(|u| u.as_str())
                        });
                    
                    if let Some(url) = image_url {
                        crate::log_message(&format!("[ModelScope] 获取到图片 URL: {}", url));
                        return self.download_and_save(url, &options.output_dir).await;
                    } else {
                        crate::log_message(&format!("[ModelScope] 任务完成但无法获取图片 URL，响应: {:?}", poll_result));
                        return Err(ProviderError::InvalidResponse("任务完成但无法获取图片 URL".to_string()));
                    }
                }
                "FAILED" => {
                    // 尝试多种路径获取错误信息
                    let error_msg = poll_result
                        .get("errors")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .or_else(|| {
                            poll_result
                                .get("output")
                                .and_then(|o| o.get("message"))
                                .and_then(|m| m.as_str())
                        })
                        .unwrap_or("任务执行失败");
                    crate::log_message(&format!("[ModelScope] 任务失败: {}", error_msg));
                    return Err(ProviderError::Api {
                        status: 500,
                        message: format!("任务失败: {}", error_msg),
                    });
                }
                "PENDING" | "RUNNING" | "PROCESSING" => {
                    // 任务进行中，重置 UNKNOWN 计数
                    unknown_count = 0;
                    continue;
                }
                _ => {
                    crate::log_message(&format!("[ModelScope] 未知任务状态: {}", status));
                    unknown_count += 1;
                    
                    // 如果连续多次 UNKNOWN，可能是任务异常，停止轮询
                    if unknown_count >= max_unknown_count {
                        crate::log_message(&format!("[ModelScope] 连续 {} 次获取到 UNKNOWN 状态，任务可能异常", max_unknown_count));
                        return Err(ProviderError::Api {
                            status: 500,
                            message: format!("任务状态异常，连续 {} 次获取到 UNKNOWN 状态", max_unknown_count),
                        });
                    }
                    continue;
                }
            }
        }

        crate::log_message(&format!("[ModelScope] 轮询超时，task_id: {}", task_id));
        Err(ProviderError::Api {
            status: 408,
            message: "轮询超时，请稍后重试或检查 ModelScope 服务状态".to_string(),
        })
    }
}

impl ModelScopeProvider {
    async fn download_and_save(&self, url: &str, output_dir: &str) -> Result<GenerationResult> {
        crate::log_message(&format!("[ModelScope] 下载图片: {}", url));
        
        let client = reqwest::Client::new();
        let image_response = client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                crate::log_message(&format!("[ModelScope] 下载图片请求失败: {}", e));
                ProviderError::Network(e)
            })?;

        if !image_response.status().is_success() {
            let status = image_response.status();
            crate::log_message(&format!("[ModelScope] 下载图片失败: status={}", status));
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: "下载图片失败".to_string(),
            });
        }

        let image_bytes = image_response
            .bytes()
            .await
            .map_err(|e| {
                crate::log_message(&format!("[ModelScope] 读取图片数据失败: {}", e));
                ProviderError::Network(e)
            })?;

        // 使用统一的命名规则生成文件名
        let image_path = crate::get_next_image_path(output_dir)?;

        std::fs::write(&image_path, &image_bytes)
            .map_err(|e| {
                crate::log_message(&format!("[ModelScope] 保存图片失败: {}", e));
                ProviderError::FileSystem(e)
            })?;

        crate::log_message(&format!("[ModelScope] 图片保存成功: {}", image_path));

        Ok(GenerationResult {
            success: true,
            image_path: Some(image_path),
            error: None,
            retries: None,
        })
    }
}
