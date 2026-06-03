use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

pub mod commands;
pub mod config;
pub mod config_store;
pub mod error;
pub mod providers;
pub mod types;

pub use types::*;

// 全局日志文件句柄
static LOG_FILE: Mutex<Option<fs::File>> = Mutex::new(None);

/// 初始化日志系统
pub fn setup_logging() {
    // 获取日志目录
    let log_dir = if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            parent.join("logs")
        } else {
            std::path::PathBuf::from("logs")
        }
    } else {
        std::path::PathBuf::from("logs")
    };

    // 创建日志目录
    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!("[日志错误] 创建日志目录失败: {}", e);
        return;
    }

    // 生成日志文件名（带日期）
    let now = chrono::Local::now();
    let log_file_path = log_dir.join(format!("ai-image-v2-{}.log", now.format("%Y-%m-%d")));

    // 打开或创建日志文件
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            // 直接写入启动日志
            let start_msg = format!("\n========== AI Image V2 启动 ==========\n时间: {}\n", now.format("%Y-%m-%d %H:%M:%S"));
            let log_line = format!("[{}] {}\n", now.format("%Y-%m-%d %H:%M:%S%.3f"), start_msg);
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
            
            // 保存到全局
            let mut log_file = LOG_FILE.lock().unwrap();
            *log_file = Some(file);
            eprintln!("[日志] 日志文件路径: {}", log_file_path.display());
        }
        Err(e) => {
            eprintln!("[日志错误] 打开日志文件失败: {}", e);
        }
    }
}

/// 写入日志消息
pub fn log_message(message: &str) {
    let mut log_file = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *log_file {
        let now = chrono::Local::now();
        let log_line = format!("[{}] {}\n", now.format("%Y-%m-%d %H:%M:%S%.3f"), message);
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
    }
}

/// 获取项目根目录
/// 优先查找包含 .env.example 的目录（项目根目录）
/// 其次查找包含 .env 的目录
pub fn get_project_root() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
        let exe_dir = exe_dir.to_path_buf();

        // 开发模式特殊处理：exe 在 target/debug 或 target/release 下
        // 直接返回项目根目录（向上3层）
        let exe_str = exe_dir.to_string_lossy();
        if exe_str.contains("target\\debug") || exe_str.contains("target/release") {
            if let Some(parent) = exe_dir.parent() {
                if let Some(parent) = parent.parent() {
                    if let Some(parent) = parent.parent() {
                        return parent.to_path_buf();
                    }
                }
            }
        }

        // 向上查找包含 .env.example 的目录（项目根目录）
        let mut current = exe.as_path();
        for _ in 0..10 {
            let test_env_example = current.join(".env.example");
            if test_env_example.exists() {
                return current.to_path_buf();
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        // 最后尝试查找 .env 文件
        let mut current = exe.as_path();
        for _ in 0..10 {
            let test_env = current.join(".env");
            if test_env.exists() {
                return current.to_path_buf();
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        return exe_dir;
    }

    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// 获取下一个图片文件名（按文件数量递增，与原项目对齐）
pub fn get_next_image_path(output_dir: &str) -> crate::error::Result<String> {
    use std::fs;
    use std::path::Path;
    
    // 如果是相对路径，拼接项目根目录
    let output_path = Path::new(output_dir);
    let full_output_path = if output_path.is_relative() {
        get_project_root().join(output_dir)
    } else {
        output_path.to_path_buf()
    };
    
    crate::log_message(&format!("[get_next_image_path] 输出目录: {}, 完整路径: {}", output_dir, full_output_path.to_string_lossy()));
    
    if !full_output_path.exists() {
        crate::log_message(&format!("[get_next_image_path] 创建目录: {}", full_output_path.to_string_lossy()));
        fs::create_dir_all(&full_output_path).map_err(|e| crate::error::ProviderError::FileSystem(e))?;
    }
    
    // 获取现有 png 文件数量（与原项目逻辑一致）
    let existing_count = fs::read_dir(&full_output_path)
        .map_err(|e| crate::error::ProviderError::FileSystem(e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".png") {
                Some(())
            } else {
                None
            }
        })
        .count();
    
    // 下一个序号 = 现有文件数 + 1
    let next_number = existing_count + 1;
    
    let result_path = format!("{}\\{}.png", full_output_path.to_string_lossy(), next_number);
    crate::log_message(&format!("[get_next_image_path] 现有文件数: {}, 下一个序号: {}, 结果路径: {}", existing_count, next_number, result_path));
    
    // 返回完整路径
    Ok(result_path)
}

/// 获取下一个视频文件名（按文件数量递增）
pub fn get_next_video_path(output_dir: &str) -> crate::error::Result<String> {
    use std::fs;
    use std::path::Path;
    
    // 如果是相对路径，拼接项目根目录
    let output_path = Path::new(output_dir);
    let full_output_path = if output_path.is_relative() {
        get_project_root().join(output_dir)
    } else {
        output_path.to_path_buf()
    };
    
    crate::log_message(&format!("[get_next_video_path] 输出目录: {}, 完整路径: {}", output_dir, full_output_path.to_string_lossy()));
    
    if !full_output_path.exists() {
        crate::log_message(&format!("[get_next_video_path] 创建目录: {}", full_output_path.to_string_lossy()));
        fs::create_dir_all(&full_output_path).map_err(|e| crate::error::ProviderError::FileSystem(e))?;
    }
    
    // 获取现有 mp4 文件数量
    let existing_count = fs::read_dir(&full_output_path)
        .map_err(|e| crate::error::ProviderError::FileSystem(e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".mp4") {
                Some(())
            } else {
                None
            }
        })
        .count();
    
    // 下一个序号 = 现有文件数 + 1
    let next_number = existing_count + 1;
    
    let result_path = format!("{}\\{}.mp4", full_output_path.to_string_lossy(), next_number);
    crate::log_message(&format!("[get_next_video_path] 现有文件数: {}, 下一个序号: {}, 结果路径: {}", existing_count, next_number, result_path));
    
    // 返回完整路径
    Ok(result_path)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub providers: ProvidersConfig,
    pub default_provider: String,
    pub default_output_dir: String,
    #[serde(default)]
    pub default_video_output_dir: String,
    pub default_width: u32,
    pub default_height: u32,
    pub proxy: String,
    pub proxy_enabled: bool,
    pub theme: String,
    pub models: ModelLists,
    #[serde(default)]
    pub default_steps: Option<u32>,
    #[serde(default)]
    pub default_guidance_scale: Option<f32>,
    #[serde(default)]
    pub default_seed: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub agnes: ProviderConfig,
    pub modelscope: ProviderConfig,
    pub nvidia: ProviderConfig,
    pub gemini: ProviderConfig,
    pub openrouter: ProviderConfig,
    pub openai: ProviderConfig,
    pub siliconflow: ProviderConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    pub api_key: String,
    pub endpoint: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelLists {
    #[serde(default)]
    pub agnes: Vec<String>,
    pub modelscope: Vec<String>,
    pub nvidia: Vec<String>,
    pub gemini: Vec<String>,
    pub openrouter: Vec<String>,
    pub openai: Vec<String>,
    pub siliconflow: Vec<String>,
}
