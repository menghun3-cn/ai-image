use crate::agnes_models;
use crate::error::{ProviderError, Result};
use crate::providers::ImageProvider;
use crate::types::{
    GenerationOptions, GenerationResult, VideoGenerationOptions, VideoGenerationResult,
};
use crate::ProviderConfig;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;
use tauri::Emitter;
use tokio::time::sleep;

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
        vec!["agnes-image-2.1-flash".to_string()]
    }

    async fn generate(&self, options: &GenerationOptions) -> Result<GenerationResult> {
        // 优先使用用户指定的模型，如果没有则从拉取的模型列表中选择最佳模型
        let model = options.model.clone().unwrap_or_else(|| {
            agnes_models::get_best_text_to_image_model()
        });

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

        // 判断是否为图生图模式
        let is_img2img = options.image.is_some();

        // 构建请求体
        let mut request_body = if is_img2img {
            // 图生图模式 - response_format 放在 extra_body 中
            serde_json::json!({
                "model": model,
                "prompt": &options.prompt,
                "n": 1,
                "size": size,
                "extra_body": {
                    "response_format": "b64_json"
                }
            })
        } else {
            // 文生图模式
            serde_json::json!({
                "model": model,
                "prompt": &options.prompt,
                "n": 1,
                "size": size,
                "return_base64": true
            })
        };

        // 处理以图生图
        if let Some(image_data) = &options.image {
            crate::log_message(&format!(
                "[debug-point agnes-provider-input] has_image=true, image_len={}, image_prefix={}",
                image_data.len(),
                image_data.chars().take(30).collect::<String>()
            ));
            crate::log_message(&format!("[Agnes] 以图生图模式，处理参考图片"));
            match self.process_image_input(image_data).await {
                Ok((full_data, _)) => {
                    // 使用 extra_body.image 数组格式
                    // 支持 data:image/xxx;base64,xxx 或 https://xxx 格式
                    // 图生图模式下 extra_body 已存在，需要合并
                    let extra_body = request_body["extra_body"].as_object_mut().unwrap();
                    extra_body.insert("image".to_string(), serde_json::json!([full_data]));
                    crate::log_message(&format!(
                        "[debug-point agnes-provider-request] request_has_image=true, image_format=extra_body.image[0], data_prefix={}",
                        full_data.chars().take(50).collect::<String>()
                    ));
                    crate::log_message(&format!("[Agnes] 参考图片处理成功，使用 extra_body.image 格式"));
                }
                Err(e) => {
                    crate::log_message(&format!("[Agnes] 参考图片处理失败: {}", e));
                    return Err(e);
                }
            }
        } else {
            crate::log_message("[debug-point agnes-provider-input] has_image=false, image_len=0, image_prefix=null");
        }

        let endpoint = if self.config.endpoint.is_empty() {
            "https://apihub.agnes-ai.com/v1/images/generations".to_string()
        } else {
            format!(
                "{}/images/generations",
                self.config.endpoint.trim_end_matches('/')
            )
        };

        crate::log_message(&format!("[Agnes] 请求接口: POST {}", endpoint));
        crate::log_message(&format!(
            "[Agnes] 请求体: {}",
            serde_json::to_string(&request_body).unwrap_or_default()
        ));
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
                message: format!(
                    "Agnes API Error: {} - {}",
                    status,
                    &text[..text.len().min(200)]
                ),
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
            .and_then(|b| b.as_str())
        {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64_json)
                .map_err(|e| ProviderError::InvalidResponse(format!("Base64 解码失败: {}", e)))?
        } else if let Some(url) = result
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("url"))
            .and_then(|u| u.as_str())
        {
            // 图片下载重试机制（最多3次）
            let mut image_data: Option<Vec<u8>> = None;
            let mut last_error = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    crate::log_message(&format!(
                        "[Agnes] 图片下载失败，第 {} 次重试...",
                        attempt
                    ));
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }

                match client
                    .get(url)
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => {
                                    crate::log_message(&format!(
                                        "[Agnes] 图片下载成功，URL: {}",
                                        &url[..url.len().min(80)]
                                    ));
                                    image_data = Some(bytes.to_vec());
                                    break;
                                }
                                Err(e) => {
                                    last_error = Some(format!("读取图片数据失败: {}", e));
                                    continue;
                                }
                            }
                        } else {
                            last_error = Some(format!("HTTP 错误: {}", response.status()));
                            continue;
                        }
                    }
                    Err(e) => {
                        last_error = Some(format!("网络错误: {}", e));
                        continue;
                    }
                }
            }
            
            // 图片下载失败，返回包含图片URL的错误，以便前端可以重新下载
            let error_msg = last_error.unwrap_or_else(|| "图片下载失败，已重试3次".to_string());
            return Err(ProviderError::DownloadFailed {
                url: url.to_string(),
                message: error_msg,
            });
        } else {
            return Err(ProviderError::InvalidResponse(
                "响应中未找到图片数据".to_string(),
            ));
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
    /// 返回: (完整data URI, 纯base64数据)
    async fn process_image_input(&self, image_input: &str) -> Result<(String, String)> {
        // 如果已经是 data URL，解析出 base64 部分
        if image_input.starts_with("data:image/") {
            let size = image_input.len();
            crate::log_message(&format!(
                "[Agnes Video] 使用已有 base64 图片数据, 大小: {} bytes ({:.2} MB)",
                size,
                size as f64 / 1024.0 / 1024.0
            ));
            // 提取 base64 部分（去掉 data:image/xxx;base64, 前缀）
            let base64_part = image_input.split(',').last().unwrap_or(image_input).to_string();
            return Ok((image_input.to_string(), base64_part));
        }

        // 如果是 http/https URL，直接返回
        if image_input.starts_with("http://") || image_input.starts_with("https://") {
            crate::log_message(&format!(
                "[Agnes Video] 使用图片 URL: {}",
                &image_input[..image_input.len().min(100)]
            ));
            return Ok((image_input.to_string(), image_input.to_string()));
        }

        // 本地文件路径，读取并转为 Base64
        let path = std::path::Path::new(image_input);
        if !path.exists() {
            return Err(ProviderError::InvalidResponse(format!(
                "图片文件不存在: {}",
                image_input
            )));
        }

        let image_data = tokio::fs::read(path)
            .await
            .map_err(|e| ProviderError::FileSystem(e))?;

        let file_size = image_data.len();
        crate::log_message(&format!(
            "[Agnes Video] 读取本地图片: {}, 原始大小: {} bytes ({:.2} MB)",
            image_input,
            file_size,
            file_size as f64 / 1024.0 / 1024.0
        ));

        // 检测图片类型
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => "image/png",
        };

        let base64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_data);

        let base64_size = base64_data.len();
        
        // 验证 base64 数据完整性
        let padding_count = base64_data.chars().filter(|&c| c == '=').count();
        let last_chars: String = base64_data.chars().rev().take(10).collect::<String>().chars().rev().collect();
        
        crate::log_message(&format!(
            "[Agnes Video] 图片转 base64 完成, base64 大小: {} bytes ({:.2} MB), 填充字符(=)数量: {}, 结尾10字符: {}",
            base64_size,
            base64_size as f64 / 1024.0 / 1024.0,
            padding_count,
            last_chars
        ));
        
        // 验证 base64 是否可以正确解码
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data) {
            Ok(decoded) => {
                crate::log_message(&format!(
                    "[Agnes Video] base64 验证通过, 解码后大小: {} bytes",
                    decoded.len()
                ));
            }
            Err(e) => {
                crate::log_message(&format!(
                    "[Agnes Video] base64 验证失败: {}",
                    e
                ));
            }
        }

        let data_uri = format!("data:{};base64,{}", mime_type, base64_data);
        Ok((data_uri, base64_data))
    }

    pub async fn generate_video(
        &self,
        options: &VideoGenerationOptions,
        app: &tauri::AppHandle,
    ) -> Result<VideoGenerationResult> {
        // 从拉取的模型列表中选择最佳文生视频模型
        let model = agnes_models::get_best_text_to_video_model();

        let key_preview = if self.config.api_key.len() > 15 {
            format!("{}...", &self.config.api_key[..15])
        } else {
            self.config.api_key.clone()
        };

        // 判断是否为图生视频模式
        let image_mode = options.image_mode.as_deref().unwrap_or("text");
        let is_image_to_video =
            image_mode != "text" && (options.image.is_some() || options.images.is_some());

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
                        crate::log_message(&format!("[Agnes Video] 处理单图: {}", image_path));
                        match self.process_image_input(image_path).await {
                            Ok((_, base64_data)) => {
                                // 使用纯 base64 数据，不包含 data URI 前缀
                                request_body["image"] = serde_json::json!(base64_data);
                                crate::log_message(&format!("[Agnes Video] 单图模式，图片处理成功，使用纯base64数据"));
                            }
                            Err(e) => {
                                crate::log_message(&format!("[Agnes Video] 单图处理失败: {}", e));
                                return Err(e);
                            }
                        }
                    } else {
                        crate::log_message(&format!("[Agnes Video] 警告: 单图模式但未提供图片路径"));
                    }
                }
                "multi" | "keyframes" => {
                    // 多图/关键帧模式：使用 extra_body.image 数组
                    if let Some(images) = &options.images {
                        crate::log_message(&format!("[Agnes Video] 处理多图/关键帧，共 {} 张", images.len()));
                        let mut processed_images = Vec::new();
                        for (i, image_path) in images.iter().enumerate() {
                            crate::log_message(&format!("[Agnes Video] 处理第 {} 张图片: {}", i + 1, image_path));
                            match self.process_image_input(image_path).await {
                                Ok((_, base64_data)) => {
                                    // 使用纯 base64 数据
                                    processed_images.push(base64_data);
                                }
                                Err(e) => {
                                    crate::log_message(&format!("[Agnes Video] 第 {} 张图片处理失败: {}", i + 1, e));
                                    return Err(e);
                                }
                            }
                        }

                        let mut extra_body = serde_json::json!({
                            "image": processed_images
                        });

                        // 关键帧模式添加 mode 参数
                        if image_mode == "keyframes" {
                            extra_body["mode"] = serde_json::json!("keyframes");
                        }

                        request_body["extra_body"] = extra_body;
                        crate::log_message(&format!(
                            "[Agnes Video] {}模式，已处理 {} 张图片",
                            image_mode,
                            processed_images.len()
                        ));
                    } else {
                        crate::log_message(&format!("[Agnes Video] 警告: 多图/关键帧模式但未提供图片路径列表"));
                    }
                }
                _ => {
                    crate::log_message(&format!("[Agnes Video] 警告: 未知的图生视频模式: {}", image_mode));
                }
            }
        }

        let create_endpoint = if self.config.endpoint.is_empty() {
            "https://apihub.agnes-ai.com/v1/videos".to_string()
        } else {
            format!("{}/videos", self.config.endpoint.trim_end_matches('/'))
        };

        crate::log_message(&format!(
            "[Agnes Video] ========== 开始视频生成任务 =========="
        ));
        crate::log_message(&format!(
            "[Agnes Video] 模式: {}",
            if is_image_to_video {
                "图生视频"
            } else {
                "文生视频"
            }
        ));
        crate::log_message(&format!("[Agnes Video] 创建任务: POST {}", create_endpoint));
        
        // 记录请求体摘要（避免输出完整的 base64 数据）
        let mut log_body = request_body.clone();
        if let Some(image) = log_body.get_mut("image").and_then(|i| i.as_str()) {
            if image.starts_with("data:image/") {
                let size = image.len();
                log_body["image"] = serde_json::json!(format!("<base64 data, {} bytes>", size));
            }
        }
        if let Some(images) = log_body.get_mut("extra_body").and_then(|e| e.get_mut("image")) {
            if let Some(arr) = images.as_array_mut() {
                for (i, img) in arr.iter_mut().enumerate() {
                    if let Some(s) = img.as_str() {
                        if s.starts_with("data:image/") {
                            let size = s.len();
                            *img = serde_json::json!(format!("<base64 data #{}, {} bytes>", i, size));
                        }
                    }
                }
            }
        }
        crate::log_message(&format!(
            "[Agnes Video] 请求体: {}",
            serde_json::to_string(&log_body).unwrap_or_default()
        ));
        crate::log_message(&format!("[Agnes Video] API Key: {}", key_preview));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
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
            crate::log_message(&format!(
                "[Agnes Video] 创建任务失败: HTTP {} - {}",
                status, text
            ));
            return Err(ProviderError::Api {
                status: status.as_u16(),
                message: format!(
                    "Agnes Video API Error: {} - {}",
                    status,
                    &text[..text.len().min(200)]
                ),
            });
        }

        let create_result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let video_id = create_result
            .get("video_id")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProviderError::InvalidResponse("响应中未找到 video_id".to_string()))?;

        crate::log_message(&format!("[Agnes Video] 任务创建成功, video_id: {}", video_id));

        // 步骤2: 轮询获取结果
        // 新 API 使用 query 参数方式查询: /agnesapi?video_id=<VIDEO_ID>
        // 注意: 这个端点不使用 /v1 前缀
        let retrieve_endpoint = if self.config.endpoint.is_empty() {
            format!("https://apihub.agnes-ai.com/agnesapi?video_id={}", video_id)
        } else {
            // 如果用户配置了自定义端点，需要确保不包含 /v1 后缀
            let base_endpoint = self.config.endpoint.trim_end_matches('/');
            // 移除可能的 /v1 后缀
            let base_endpoint = base_endpoint.trim_end_matches("/v1").trim_end_matches('/');
            format!("{}/agnesapi?video_id={}", base_endpoint, video_id)
        };

        let video_url = loop {
            sleep(Duration::from_secs(5)).await; // 每5秒轮询一次

            crate::log_message(&format!(
                "[Agnes Video] 轮询任务状态: GET {}",
                retrieve_endpoint
            ));

            let poll_response = client
                .get(&retrieve_endpoint)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .send()
                .await
                .map_err(ProviderError::Network)?;

            if !poll_response.status().is_success() {
                let status = poll_response.status();
                let text = poll_response.text().await.unwrap_or_default();
                crate::log_message(&format!(
                    "[Agnes Video] 轮询任务状态失败: HTTP {} - {}",
                    status, text
                ));
                return Err(ProviderError::Api {
                    status: status.as_u16(),
                    message: format!(
                        "轮询任务状态失败: {} - {}",
                        status,
                        &text[..text.len().min(200)]
                    ),
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

            crate::log_message(&format!(
                "[Agnes Video] 任务状态: {}, 进度: {}%",
                status, progress
            ));
            crate::log_message(&format!(
                "[Agnes Video] 完整响应: {}",
                poll_result.to_string()
            ));

            // 发送进度事件到前端
            let _ = app.emit(
                "video-generation-progress",
                serde_json::json!({
                    "status": status,
                    "progress": progress,
                }),
            );

            match status.to_uppercase().as_str() {
                "SUCCESS" | "COMPLETED" => {
                    // 尝试从多个可能的字段获取视频URL
                    // Agnes API 使用 remixed_from_video_id 而不是 video_url
                    let url_fields = ["video_url", "remixed_from_video_id", "url", "output_url"];
                    let mut found_url: Option<String> = None;

                    for field in &url_fields {
                        if let Some(url_value) = poll_result.get(field) {
                            crate::log_message(&format!(
                                "[Agnes Video] {} 字段值: {:?}",
                                field, url_value
                            ));

                            if let Some(url_str) = url_value.as_str() {
                                if !url_str.is_empty() {
                                    // 去除可能的反引号包裹
                                    let cleaned_url = url_str.trim_matches('`').to_string();
                                    crate::log_message(&format!(
                                        "[Agnes Video] 找到视频URL (来自 {}): {}",
                                        field, cleaned_url
                                    ));
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
                    crate::log_message(&format!(
                        "[Agnes Video] 响应内容: {}",
                        poll_result.to_string()
                    ));
                    return Err(ProviderError::InvalidResponse(format!(
                        "响应中未找到有效的视频URL。响应内容: {}",
                        poll_result.to_string()
                    )));
                }
                "FAILED" | "FAILURE" | "ERROR" => {
                    // 提取详细的错误信息
                    // error 字段可能是字符串或对象 {"code": "...", "message": "..."}
                    let error_msg = poll_result
                        .get("error")
                        .and_then(|e| {
                            // 尝试获取 error.message
                            e.get("message").and_then(|m| m.as_str())
                                .or_else(|| e.as_str()) // 或者 error 直接是字符串
                        })
                        .or_else(|| poll_result.get("message").and_then(|m| m.as_str()))
                        .unwrap_or("未知错误");
                    
                    crate::log_message(&format!(
                        "[Agnes Video] 任务失败: {} - 完整响应: {}",
                        error_msg,
                        poll_result.to_string()
                    ));
                    
                    return Err(ProviderError::Api {
                        status: 500,
                        message: format!("视频生成失败: {}", error_msg),
                    });
                }
                "PENDING" | "PROCESSING" | "QUEUED" | "CREATED" | "RUNNING" | "IN_PROGRESS" => {
                    continue; // 继续轮询
                }
                _ => {
                    return Err(ProviderError::InvalidResponse(format!(
                        "未知任务状态: {}",
                        status
                    )));
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
        crate::log_message(&format!(
            "[Agnes Video] ========== 视频生成任务完成 =========="
        ));

        Ok(VideoGenerationResult {
            success: true,
            video_path: Some(video_path),
            error: None,
        })
    }
}
