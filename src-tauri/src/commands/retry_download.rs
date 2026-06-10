use crate::types::RetryDownloadResult;

/// 重新下载图片命令
/// 当图片下载失败时，允许用户使用原始URL重新下载
#[tauri::command]
pub async fn retry_download_image(
    image_url: String,
    output_dir: String,
    filename: Option<String>,
) -> Result<RetryDownloadResult, String> {
    crate::log_message(&format!(
        "[Retry Download] 开始重新下载图片，URL: {}",
        &image_url[..image_url.len().min(80)]
    ));

    // 创建输出目录
    let output_path = std::path::Path::new(&output_dir);
    if !output_path.exists() {
        tokio::fs::create_dir_all(output_path)
            .await
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
    }

    // 确定文件名
    let image_path_str = if let Some(name) = filename {
        output_path.join(name).to_string_lossy().to_string()
    } else {
        crate::get_next_image_path(&output_dir).map_err(|e| e.to_string())?
    };
    let image_path = std::path::PathBuf::from(&image_path_str);

    // 创建 HTTP 客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 重试下载（最多3次）
    let mut last_error = None;
    for attempt in 0..3 {
        if attempt > 0 {
            crate::log_message(&format!(
                "[Retry Download] 第 {} 次重试...",
                attempt
            ));
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        match client.get(&image_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            // 保存图片
                            match tokio::fs::write(&image_path, bytes).await {
                                Ok(_) => {
                                    let path_str = image_path.to_string_lossy().to_string();
                                    crate::log_message(&format!(
                                        "[Retry Download] 图片下载成功，保存到: {}",
                                        path_str
                                    ));
                                    return Ok(RetryDownloadResult {
                                        success: true,
                                        image_path: Some(path_str),
                                        error: None,
                                    });
                                }
                                Err(e) => {
                                    last_error = Some(format!("保存图片失败: {}", e));
                                    continue;
                                }
                            }
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

    // 所有重试都失败了
    let error_msg = last_error.unwrap_or_else(|| "图片下载失败，已重试3次".to_string());
    crate::log_message(&format!("[Retry Download] 下载失败: {}", error_msg));
    
    Ok(RetryDownloadResult {
        success: false,
        image_path: None,
        error: Some(error_msg),
    })
}
