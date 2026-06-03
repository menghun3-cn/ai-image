use async_trait::async_trait;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{GenerationOptions, GenerationResult, VideoGenerationOptions, VideoGenerationResult};
use crate::ProviderConfig;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tauri::Emitter;

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

// 视频生成实现（非 trait 方法）
impl AgnesProvider {
    /// 将图片路径或URL转换为 Base64 或保持URL
    /// 支持：本地文件路径、http/https URL、data URL
    async fn process_image_input(&self, image_input: &str) -> Result<String> {
        // 如果已经是 data URL，直接返回
        if image_input.starts_with("data:image/") {
            return Ok(image_input.to_string());
        }

        // 如果是 http/https URL，直接返回
        if image_input.starts_with("http://") || image_input.starts_with("https://") {
            return Ok(image_input.to_string());
        }

        // 本地文件路径，读取并转为 Base64
        let path = std::path::Path::new(image_input);
        if !path.exists() {
            return Err(ProviderError::InvalidResponse(format!("图片文件不存在: {}", image_input)));
        }

        let image_data = tokio::fs::read(path)
            .await
            .map_err(|e| ProviderError::FileSystem(e))?;

        // 检测图片类型
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => "image/png",
        };

        let base64_data = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_data
        );

        Ok(format!("data:{};base64,{}", mime_type, base64_data))
    }

    pub async fn generate_video(&self, options: &VideoGenerationOptions, app: &tauri::AppHandle) -> Result<VideoGenerationResult> {
        let model = "agnes-video-v2.0";

        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 判断是否为图生视频模式
        let image_mode = options.image_mode.as_deref().unwrap_or("text");
        let is_image_to_video = image_mode != "text" && (options.image.is_some() || options.images.is_some());

        // 构建请求体，只添加非空参数
        let mut request_body = serde_json::json!({
            "model": model,
            "prompt": &options.prompt,
        });

        if let Some(width) = options.width {
            request_body["width"] = serde_json::json!(width);
        }
        if let Some(height) = options.height {
            request_body["height"] = serde_json::json!(height);
        }
        if let Some(num_frames) = options.num_frames {
            request_body["num_frames"] = serde_json::json!(num_frames);
        }
        if let Some(frame_rate) = options.frame_rate {
            request_body["frame_rate"] = serde_json::json!(frame_rate);
        }
        if let Some(seed) = options.seed {
            request_body["seed"] = serde_json::json!(seed);
        }
        if let Some(negative_prompt) = &options.negative_prompt {
            request_body["negative_prompt"] = serde_json::json!(negative_prompt);
        }

        // 处理图生视频参数
        if is_image_to_video {
            crate::log_message(&format!("[Agnes Video] 图生视频模式: {}", image_mode));

            match image_mode {
                "single" => {
                    // 单图生视频：直接使用 image 参数
                    if let Some(image_path) = &options.image {
                        let processed_image = self.process_image_input(image_path).await?;
                        request_body["image"] = serde_json::json!(processed_image);
                        crate::log_message(&format!("[Agnes Video] 单图模式，图片已处理"));
                    }
                }
                "multi" | "keyframes" => {
                    // 多图/关键帧模式：使用 extra_body.image 数组
                    if let Some(images) = &options.images {
                        let mut processed_images = Vec::new();
                        for image_path in images {
                            let processed = self.process_image_input(image_path).await?;
                            processed_images.push(processed);
                        }

                        let mut extra_body = serde_json::json!({
                            "image": processed_images
                        });

                        // 关键帧模式添加 mode 参数
                        if image_mode == "keyframes" {
                            extra_body["mode"] = serde_json::json!("keyframes");
                        }

                        request_body["extra_body"] = extra_body;
                        crate::log_message(&format!("[Agnes Video] {}模式，已处理 {} 张图片", image_mode, processed_images.len()));
                    }
                }
                _ => {}
            }
        }

        let create_endpoint = if self.config.endpoint.is_empty() {
            "https://apihub.agnes-ai.com/v1/videos".to_string()
        } else {
            format!("{}/videos", self.config.endpoint.trim_end_matches('/'))
        };

        crate::log_message(&format!("[Agnes Video] ========== 开始视频生成任务 =========="));
        crate::log_message(&format!("[Agnes Video] 模式: {}", if is_image_to_video { "图生视频" } else { "文生视频" }));
        crate::log_message(&format!("[Agnes Video] 创建任务: POST {}", create_endpoint));
        crate::log_message(&format!("[Agnes Video] 请求体: {}", serde_json::to_string(&request_body).unwrap_or_default()));
        crate::log_message(&format!("[Agnes Video] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Network(e))?;

        // 步骤1: 创建视频生成任务
        let response = client
            .post(&create_endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            crate::log_message(&format!("[Agnes Video] 创建任务失败: HTTP {} - {}", status, text));
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: format!("Agnes Video API Error: {} - {}", status, &text[..text.len().min(200)]),
            });
        }

        let create_result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let task_id = create_result
            .get("task_id")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProviderError::InvalidResponse("响应中未找到 task_id".to_string()))?;

        crate::log_message(&format!("[Agnes Video] 任务创建成功, task_id: {}", task_id));

        // 步骤2: 轮询获取结果
        let retrieve_endpoint = if self.config.endpoint.is_empty() {
            format!("https://apihub.agnes-ai.com/v1/videos/{}", task_id)
        } else {
            format!("{}/videos/{}", self.config.endpoint.trim_end_matches('/'), task_id)
        };

        let video_url = loop {
            sleep(Duration::from_secs(5)).await; // 每5秒轮询一次

            crate::log_message(&format!("[Agnes Video] 轮询任务状态: GET {}", retrieve_endpoint));

            let poll_response = client
                .get(&retrieve_endpoint)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .send()
                .await
                .map_err(ProviderError::Network)?;

            if !poll_response.status().is_success() {
                let status = poll_response.status();
                let text = poll_response.text().await.unwrap_or_default();
                crate::log_message(&format!("[Agnes Video] 轮询任务状态失败: HTTP {} - {}", status, text));
                return Err(ProviderError::Api {
                    status: status.as_u16(),
                    message: format!("轮询任务状态失败: {} - {}", status, &text[..text.len().min(200)]),
                });
            }

            let poll_result: serde_json::Value = poll_response
                .json()
                .await
                .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

            let status = poll_result
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("UNKNOWN");

            // 提取进度百分比
            let progress = poll_result
                .get("progress")
                .and_then(|p| p.as_i64())
                .unwrap_or(0) as i32;

            crate::log_message(&format!("[Agnes Video] 任务状态: {}, 进度: {}%", status, progress));
            crate::log_message(&format!("[Agnes Video] 完整响应: {}", poll_result.to_string()));

            // 发送进度事件到前端
            let _ = app.emit("video-generation-progress", serde_json::json!({
                "status": status,
                "progress": progress,
            }));

            match status.to_uppercase().as_str() {
                "SUCCESS" | "COMPLETED" => {
                    // 尝试从多个可能的字段获取视频URL
                    // Agnes API 使用 remixed_from_video_id 而不是 video_url
                    let url_fields = ["video_url", "remixed_from_video_id", "url", "output_url"];
                    let mut found_url: Option<String> = None;
                    
                    for field in &url_fields {
                        if let Some(url_value) = poll_result.get(field) {
                            crate::log_message(&format!("[Agnes Video] {} 字段值: {:?}", field, url_value));
                            
                            if let Some(url_str) = url_value.as_str() {
                                if !url_str.is_empty() {
                                    // 去除可能的反引号包裹
                                    let cleaned_url = url_str.trim_matches('`').to_string();
                                    crate::log_message(&format!("[Agnes Video] 找到视频URL (来自 {}): {}", field, cleaned_url));
                                    found_url = Some(cleaned_url);
                                    break;
                                }
                            }
                        }
                    }
                    
                    if let Some(url) = found_url {
                        break url;
                    }
                    
                    // 如果没有找到URL，记录错误
                    crate::log_message(&format!("[Agnes Video] 响应内容: {}", poll_result.to_string()));
                    return Err(ProviderError::InvalidResponse(
                        format!("响应中未找到有效的视频URL。响应内容: {}", poll_result.to_string())
                    ));
                }
                "FAILED" | "FAILURE" | "ERROR" => {
                    return Err(ProviderError::Api {
                        status: 500,
                        message: "视频生成失败".to_string(),
                    });
                }
                "PENDING" | "PROCESSING" | "QUEUED" | "CREATED" | "RUNNING" | "IN_PROGRESS" => {
                    continue; // 继续轮询
                }
                _ => {
                    return Err(ProviderError::InvalidResponse(format!("未知任务状态: {}", status)));
                }
            }
        };

        crate::log_message(&format!("[Agnes Video] 视频生成完成, URL: {}", video_url));

        // 步骤3: 下载视频
        let video_response = client
            .get(&video_url)
            .send()
            .await
            .map_err(ProviderError::Network)?;

        if !video_response.status().is_success() {
            return Err(ProviderError::Api {
                status: video_response.status().as_u16(),
                message: "下载视频失败".to_string(),
            });
        }

        let video_data = video_response
            .bytes()
            .await
            .map_err(|e| ProviderError::Network(e))?
            .to_vec();

        // 步骤4: 保存视频
        let output_path = Path::new(&options.output_dir);
        if !output_path.exists() {
            tokio::fs::create_dir_all(output_path)
                .await
                .map_err(|e| ProviderError::FileSystem(e))?;
        }

        let video_path = crate::get_next_video_path(&options.output_dir)?;

        tokio::fs::write(&video_path, video_data)
            .await
            .map_err(|e| ProviderError::FileSystem(e))?;

        crate::log_message(&format!("[Agnes Video] 视频保存成功: {}", video_path));
        crate::log_message(&format!("[Agnes Video] ========== 视频生成任务完成 =========="));

        Ok(VideoGenerationResult {
            success: true,
            video_path: Some(video_path),
            error: None,
        })
    }
}