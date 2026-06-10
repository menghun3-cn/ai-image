use crate::types::{ImageInfo, VideoInfo};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

/// 目录缓存状态
#[derive(Debug, Clone)]
pub struct DirectoryCache<T> {
    /// 缓存的数据
    pub data: Vec<T>,
    /// 目录最后修改时间（用于检测变化）
    pub last_modified: HashMap<String, SystemTime>,
    /// 缓存时间戳
    pub cached_at: SystemTime,
    /// 目录路径
    pub directory_path: String,
}

/// 图库缓存管理器
pub struct GalleryCache {
    /// 图片目录缓存
    image_cache: Mutex<Option<DirectoryCache<ImageInfo>>>,
    /// 视频目录缓存
    video_cache: Mutex<Option<DirectoryCache<VideoInfo>>>,
}

impl GalleryCache {
    /// 创建新的缓存管理器
    pub fn new() -> Self {
        Self {
            image_cache: Mutex::new(None),
            video_cache: Mutex::new(None),
        }
    }

    /// 检查目录是否有变化
    /// 返回 true 表示有变化，需要重新加载
    fn has_directory_changed(
        &self,
        dir_path: &Path,
        last_modified: &HashMap<String, SystemTime>,
    ) -> bool {
        // 如果目录不存在，视为有变化（需要重新检查）
        if !dir_path.exists() {
            return true;
        }

        // 获取当前目录下的所有文件及其修改时间
        let mut current_files: HashMap<String, SystemTime> = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let path_str = entry.path().to_string_lossy().to_string();
                        current_files.insert(path_str, modified);
                    }
                }
            }
        }

        // 比较文件数量和具体文件的修改时间
        if current_files.len() != last_modified.len() {
            return true;
        }

        // 检查每个文件的修改时间是否一致
        for (path, time) in &current_files {
            match last_modified.get(path) {
                Some(cached_time) => {
                    if cached_time != time {
                        return true;
                    }
                }
                None => return true, // 新文件
            }
        }

        false // 没有变化
    }

    /// 构建文件修改时间映射
    fn build_file_modified_map(&self, items: &[impl AsRef<Path>], dir_path: &Path) -> HashMap<String, SystemTime> {
        let mut map = HashMap::new();

        for item in items {
            let path = item.as_ref();
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    map.insert(path.to_string_lossy().to_string(), modified);
                }
            }
        }

        // 同时记录目录本身的修改时间
        if let Ok(metadata) = std::fs::metadata(dir_path) {
            if let Ok(modified) = metadata.modified() {
                map.insert(dir_path.to_string_lossy().to_string(), modified);
            }
        }

        map
    }

    /// 获取图片列表（带缓存）
    /// 如果目录没有变化，返回缓存数据；否则重新扫描
    pub fn get_images(&self, dir_path: &str) -> Result<Vec<ImageInfo>, String> {
        let path = Path::new(dir_path);
        let full_path = if path.is_relative() {
            crate::get_project_root().join(dir_path)
        } else {
            path.to_path_buf()
        };

        // 检查缓存
        let mut cache = self.image_cache.lock().unwrap();

        if let Some(ref cached) = *cache {
            // 检查是否是同一目录
            if cached.directory_path == full_path.to_string_lossy().to_string() {
                // 检查目录是否有变化
                if !self.has_directory_changed(&full_path, &cached.last_modified) {
                    crate::log_message(&format!(
                        "[GalleryCache] 使用缓存，目录无变化: {}",
                        full_path.to_string_lossy()
                    ));
                    return Ok(cached.data.clone());
                }
                crate::log_message(&format!(
                    "[GalleryCache] 目录有变化，重新加载: {}",
                    full_path.to_string_lossy()
                ));
            }
        }

        // 重新扫描目录
        crate::log_message(&format!(
            "[GalleryCache] 扫描图片目录: {}",
            full_path.to_string_lossy()
        ));

        let mut images = vec![];

        if full_path.exists() {
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
                                        name: path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string(),
                                        time,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按时间倒序排列
        images.sort_by(|a, b| b.time.cmp(&a.time));

        // 构建文件修改时间映射
        let file_paths: Vec<&Path> = images.iter().map(|img| Path::new(&img.path)).collect();
        let last_modified = self.build_file_modified_map(&file_paths, &full_path);

        // 更新缓存
        *cache = Some(DirectoryCache {
            data: images.clone(),
            last_modified,
            cached_at: SystemTime::now(),
            directory_path: full_path.to_string_lossy().to_string(),
        });

        crate::log_message(&format!(
            "[GalleryCache] 缓存已更新，共 {} 张图片",
            images.len()
        ));

        Ok(images)
    }

    /// 获取视频列表（带缓存）
    /// 如果目录没有变化，返回缓存数据；否则重新扫描
    pub fn get_videos(&self, dir_path: &str) -> Result<Vec<VideoInfo>, String> {
        let path = Path::new(dir_path);
        let full_path = if path.is_relative() {
            crate::get_project_root().join(dir_path)
        } else {
            path.to_path_buf()
        };

        // 检查缓存
        let mut cache = self.video_cache.lock().unwrap();

        if let Some(ref cached) = *cache {
            // 检查是否是同一目录
            if cached.directory_path == full_path.to_string_lossy().to_string() {
                // 检查目录是否有变化
                if !self.has_directory_changed(&full_path, &cached.last_modified) {
                    crate::log_message(&format!(
                        "[VideoCache] 使用缓存，目录无变化: {}",
                        full_path.to_string_lossy()
                    ));
                    return Ok(cached.data.clone());
                }
                crate::log_message(&format!(
                    "[VideoCache] 目录有变化，重新加载: {}",
                    full_path.to_string_lossy()
                ));
            }
        }

        // 重新扫描目录
        crate::log_message(&format!(
            "[VideoCache] 扫描视频目录: {}",
            full_path.to_string_lossy()
        ));

        let mut videos = vec![];

        if full_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&full_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ext == "mp4" || ext == "mov" || ext == "avi" || ext == "mkv" || ext == "webm" {
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let time = modified
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;

                                    videos.push(VideoInfo {
                                        path: path.to_string_lossy().to_string(),
                                        name: path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string(),
                                        time,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按时间倒序排列
        videos.sort_by(|a, b| b.time.cmp(&a.time));

        // 构建文件修改时间映射
        let file_paths: Vec<&Path> = videos.iter().map(|v| Path::new(&v.path)).collect();
        let last_modified = self.build_file_modified_map(&file_paths, &full_path);

        // 更新缓存
        *cache = Some(DirectoryCache {
            data: videos.clone(),
            last_modified,
            cached_at: SystemTime::now(),
            directory_path: full_path.to_string_lossy().to_string(),
        });

        crate::log_message(&format!(
            "[VideoCache] 缓存已更新，共 {} 个视频",
            videos.len()
        ));

        Ok(videos)
    }

    /// 清除图片缓存
    pub fn clear_image_cache(&self) {
        let mut cache = self.image_cache.lock().unwrap();
        *cache = None;
        crate::log_message("[GalleryCache] 图片缓存已清除");
    }

    /// 清除视频缓存
    pub fn clear_video_cache(&self) {
        let mut cache = self.video_cache.lock().unwrap();
        *cache = None;
        crate::log_message("[VideoCache] 视频缓存已清除");
    }

    /// 清除所有缓存
    pub fn clear_all_cache(&self) {
        self.clear_image_cache();
        self.clear_video_cache();
    }

    /// 检查图片缓存是否有效（轻量级，不返回数据）
    /// 返回 (is_valid, item_count)
    /// - is_valid: true 表示缓存有效，false 表示需要重新加载
    /// - item_count: 缓存中的项目数量（仅当 is_valid 为 true 时有意义）
    pub fn is_image_cache_valid(&self, dir_path: &str) -> (bool, usize) {
        let path = Path::new(dir_path);
        let full_path = if path.is_relative() {
            crate::get_project_root().join(dir_path)
        } else {
            path.to_path_buf()
        };

        let cache = self.image_cache.lock().unwrap();
        
        if let Some(ref cached) = *cache {
            // 检查是否是同一目录
            if cached.directory_path == full_path.to_string_lossy().to_string() {
                // 检查目录是否有变化
                if !self.has_directory_changed(&full_path, &cached.last_modified) {
                    return (true, cached.data.len());
                }
            }
        }
        
        (false, 0)
    }

    /// 检查视频缓存是否有效（轻量级，不返回数据）
    /// 返回 (is_valid, item_count)
    pub fn is_video_cache_valid(&self, dir_path: &str) -> (bool, usize) {
        let path = Path::new(dir_path);
        let full_path = if path.is_relative() {
            crate::get_project_root().join(dir_path)
        } else {
            path.to_path_buf()
        };

        let cache = self.video_cache.lock().unwrap();
        
        if let Some(ref cached) = *cache {
            // 检查是否是同一目录
            if cached.directory_path == full_path.to_string_lossy().to_string() {
                // 检查目录是否有变化
                if !self.has_directory_changed(&full_path, &cached.last_modified) {
                    return (true, cached.data.len());
                }
            }
        }
        
        (false, 0)
    }

    /// 获取目录的修改时间（mtime）
    /// 用于前端快速检测目录是否有变化
    /// 返回 (mtime, 文件数量)，如果目录不存在返回 (0, 0)
    pub fn get_directory_mtime(&self, dir_path: &str) -> (u64, usize) {
        let path = Path::new(dir_path);
        let full_path = if path.is_relative() {
            crate::get_project_root().join(dir_path)
        } else {
            path.to_path_buf()
        };

        if !full_path.exists() || !full_path.is_dir() {
            return (0, 0);
        }

        // 获取目录的修改时间
        let dir_mtime = match std::fs::metadata(&full_path) {
            Ok(metadata) => match metadata.modified() {
                Ok(time) => match time.duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs(),
                    Err(_) => 0,
                },
                Err(_) => 0,
            },
            Err(_) => 0,
        };

        // 获取目录下的文件数量（只统计直接子文件，不递归）
        let file_count = match std::fs::read_dir(&full_path) {
            Ok(entries) => entries.filter_map(|e| e.ok()).count(),
            Err(_) => 0,
        };

        (dir_mtime, file_count)
    }
}

// 全局缓存实例
use once_cell::sync::Lazy;

pub static GALLERY_CACHE: Lazy<GalleryCache> = Lazy::new(GalleryCache::new);
