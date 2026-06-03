use crate::types::ImageInfo;
use std::path::Path;

#[tauri::command]
pub fn get_images(output_dir: String) -> Result<Vec<ImageInfo>, String> {
    // 如果是相对路径，拼接项目根目录
    let path = Path::new(&output_dir);
    let full_path = if path.is_relative() {
        crate::get_project_root().join(output_dir)
    } else {
        path.to_path_buf()
    };
    
    // 记录日志便于调试
    crate::log_message(&format!("[Gallery] 查找图片目录: {}", full_path.to_string_lossy()));
    
    if !full_path.exists() {
        crate::log_message(&format!("[Gallery] 目录不存在: {}", full_path.to_string_lossy()));
        return Ok(vec![]);
    }

    let mut images = vec![];

    if let Ok(entries) = std::fs::read_dir(&full_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let time = modified
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;

                            images.push(ImageInfo {
                                path: path.to_string_lossy().to_string(),
                                name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                                time,
                            });
                        }
                    }
                }
            }
        }
    }

    // 按时间倒序排列
    images.sort_by(|a, b| b.time.cmp(&a.time));

    crate::log_message(&format!("[Gallery] 找到 {} 张图片", images.len()));

    Ok(images)
}

#[tauri::command]
pub fn delete_image(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {}", e))
}

#[tauri::command]
pub fn open_output_dir(path: String) -> Result<(), String> {
    // 如果是相对路径，拼接项目根目录
    let output_path = Path::new(&path);
    let full_path = if output_path.is_relative() {
        crate::get_project_root().join(&path)
    } else {
        output_path.to_path_buf()
    };

    let full_path_str = full_path.to_string_lossy().to_string();
    crate::log_message(&format!("[open_output_dir] 打开目录: 原始路径={}, 完整路径={}", path, full_path_str));

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&full_path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&full_path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&full_path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    Ok(())
}
