use crate::gallery_cache::GALLERY_CACHE;
use crate::types::ImageInfo;
use std::path::Path;

#[tauri::command]
pub fn get_images(output_dir: String) -> Result<Vec<ImageInfo>, String> {
    // 使用缓存管理器获取图片列表
    // 如果目录没有变化，直接返回缓存数据
    GALLERY_CACHE.get_images(&output_dir)
}

#[tauri::command]
pub fn refresh_images(output_dir: String) -> Result<Vec<ImageInfo>, String> {
    // 强制刷新图片列表（清除缓存后重新加载）
    GALLERY_CACHE.clear_image_cache();
    GALLERY_CACHE.get_images(&output_dir)
}

#[tauri::command]
pub fn get_directory_mtime(output_dir: String) -> (u64, usize) {
    // 获取目录的修改时间（mtime）和文件数量
    // 用于前端快速检测目录是否有变化
    // 返回 (mtime_seconds, file_count)
    GALLERY_CACHE.get_directory_mtime(&output_dir)
}

#[tauri::command]
pub fn delete_image(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {}", e))?;
    // 删除文件后清除缓存，下次加载时会重新扫描
    GALLERY_CACHE.clear_image_cache();
    Ok(())
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
    crate::log_message(&format!(
        "[open_output_dir] 打开目录: 原始路径={}, 完整路径={}",
        path, full_path_str
    ));

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
