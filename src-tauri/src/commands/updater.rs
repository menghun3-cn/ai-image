use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

/// 从 GitHub 获取最新的 release 信息
async fn fetch_latest_release_info() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/menghun3-cn/ai-image/releases/latest")
        .header("User-Agent", "AI-Image-Updater")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("GitHub API 返回错误: {} - {}", status, text));
    }
    
    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 GitHub API 响应失败: {}", e))?;
    
    Ok(release)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub status: String,
    pub progress: u64,
    pub total: Option<u64>,
    pub message: String,
}

/// 检查更新 - 使用 GitHub API 直接检查
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    log::info!("[Updater] 开始检查更新...");
    
    let current_version = app.package_info().version.to_string();
    log::info!("[Updater] 当前版本: {}", current_version);

    // 使用 GitHub API 获取最新 release 信息
    match fetch_latest_release_info().await {
        Ok(release) => {
            log::info!("[Updater] GitHub API 返回的 release 信息:");
            
            // 获取最新版本号
            let latest_version = release
                .get("tag_name")
                .and_then(|v| v.as_str())
                .map(|v| v.trim_start_matches('v'))
                .unwrap_or("");
            
            log::info!("  - 最新版本标签: {}", latest_version);
            log::info!("  - 名称: {:?}", release.get("name"));
            
            // 列出所有资产文件名
            let mut has_installer = false;
            if let Some(assets) = release.get("assets").and_then(|a| a.as_array()) {
                log::info!("  - 资源数量: {}", assets.len());
                for (i, asset) in assets.iter().enumerate() {
                    if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                        log::info!("  - 资产[{}]: {}", i, name);
                        // 检查是否有 Windows 安装程序
                        if name.ends_with(".exe") || name.ends_with(".msi") {
                            has_installer = true;
                        }
                    }
                }
            }
            
            // 比较版本号
            let has_update = if latest_version.is_empty() {
                false
            } else {
                compare_versions(&current_version, latest_version)
            };
            
            log::info!("[Updater] 版本比较: 当前={}, 最新={}, 有更新={}", 
                current_version, latest_version, has_update);
            
            if has_update {
                // 构建发布说明
                let notes = release
                    .get("body")
                    .and_then(|b| b.as_str())
                    .map(|s| s.to_string());
                
                let pub_date = release
                    .get("published_at")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());
                
                // 如果有安装程序资产，返回更新信息
                if has_installer {
                    Ok(UpdateInfo {
                        version: latest_version.to_string(),
                        current_version,
                        notes,
                        pub_date,
                        has_update: true,
                    })
                } else {
                    // 没有安装程序，但仍然提示有新版本
                    Ok(UpdateInfo {
                        version: latest_version.to_string(),
                        current_version,
                        notes: Some(format!("发现新版本 {}，但未找到安装程序。\n\n请访问 GitHub Releases 页面手动下载。\n\n{}", 
                            latest_version,
                            notes.unwrap_or_default())),
                        pub_date,
                        has_update: true,
                    })
                }
            } else {
                Ok(UpdateInfo {
                    version: current_version.clone(),
                    current_version,
                    notes: None,
                    pub_date: None,
                    has_update: false,
                })
            }
        }
        Err(e) => {
            log::error!("[Updater] 获取 GitHub release 信息失败: {}", e);
            Err(format!("检查更新失败: {}", e))
        }
    }
}

/// 比较版本号
/// 返回 true 如果 remote_version 比 current_version 新
fn compare_versions(current: &str, remote: &str) -> bool {
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    let remote_parts: Vec<u32> = remote
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    for i in 0..std::cmp::max(current_parts.len(), remote_parts.len()) {
        let current_part = current_parts.get(i).copied().unwrap_or(0);
        let remote_part = remote_parts.get(i).copied().unwrap_or(0);
        
        if remote_part > current_part {
            return true;
        } else if remote_part < current_part {
            return false;
        }
    }
    
    false
}

/// 下载并安装更新
/// 由于 GitHub releases 没有 latest.json，我们打开浏览器让用户手动下载
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    log::info!("[Updater] 开始下载并安装更新...");

    // 获取最新 release 信息
    match fetch_latest_release_info().await {
        Ok(release) => {
            let html_url = release
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or("https://github.com/menghun3-cn/ai-image/releases");
            
            // 查找 Windows 安装程序
            let mut installer_url: Option<String> = None;
            if let Some(assets) = release.get("assets").and_then(|a| a.as_array()) {
                for asset in assets.iter() {
                    if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                        if name.ends_with(".exe") || name.ends_with(".msi") {
                            installer_url = asset
                                .get("browser_download_url")
                                .and_then(|u| u.as_str())
                                .map(|s| s.to_string());
                            log::info!("[Updater] 找到安装程序: {}", name);
                            break;
                        }
                    }
                }
            }
            
            // 如果有直接的下载链接，打开下载
            if let Some(url) = installer_url {
                log::info!("[Updater] 打开下载链接: {}", url);
                
                // 使用系统默认浏览器打开下载链接
                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("cmd")
                        .args(["/c", "start", "", &url])
                        .spawn()
                        .map_err(|e| format!("打开下载链接失败: {}", e))?;
                }
                
                #[cfg(target_os = "macos")]
                {
                    std::process::Command::new("open")
                        .arg(&url)
                        .spawn()
                        .map_err(|e| format!("打开下载链接失败: {}", e))?;
                }
                
                #[cfg(target_os = "linux")]
                {
                    std::process::Command::new("xdg-open")
                        .arg(&url)
                        .spawn()
                        .map_err(|e| format!("打开下载链接失败: {}", e))?;
                }
                
                Ok(())
            } else {
                // 没有安装程序，打开 releases 页面
                log::info!("[Updater] 打开 releases 页面: {}", html_url);
                
                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("cmd")
                        .args(["/c", "start", "", html_url])
                        .spawn()
                        .map_err(|e| format!("打开 releases 页面失败: {}", e))?;
                }
                
                #[cfg(target_os = "macos")]
                {
                    std::process::Command::new("open")
                        .arg(html_url)
                        .spawn()
                        .map_err(|e| format!("打开 releases 页面失败: {}", e))?;
                }
                
                #[cfg(target_os = "linux")]
                {
                    std::process::Command::new("xdg-open")
                        .arg(html_url)
                        .spawn()
                        .map_err(|e| format!("打开 releases 页面失败: {}", e))?;
                }
                
                Ok(())
            }
        }
        Err(e) => {
            log::error!("[Updater] 获取 release 信息失败: {}", e);
            Err(format!("获取更新信息失败: {}", e))
        }
    }
}

/// 获取当前应用版本
#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
