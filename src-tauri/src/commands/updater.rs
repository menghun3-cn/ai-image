use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub has_update: bool,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub status: String,
    pub progress: u64,
    pub total: Option<u64>,
    pub message: String,
}

/// 检查更新 - 使用 Tauri 官方 updater 插件
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    log::info!("[Updater] 开始检查更新...");

    let current_version = app.package_info().version.to_string();
    log::info!("[Updater] 当前版本: {}", current_version);

    // 使用 Tauri updater 插件检查更新
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    log::info!("[Updater] 发现新版本: {}", update.version);
                    log::info!("[Updater] 下载地址: {:?}", update.download_url);

                    Ok(UpdateInfo {
                        version: update.version.to_string(),
                        current_version,
                        notes: update.body,
                        pub_date: update.date.map(|d| d.to_string()),
                        has_update: true,
                        download_url: Some(update.download_url.to_string()),
                    })
                }
                Ok(None) => {
                    log::info!("[Updater] 当前已是最新版本");
                    Ok(UpdateInfo {
                        version: current_version.clone(),
                        current_version,
                        notes: None,
                        pub_date: None,
                        has_update: false,
                        download_url: None,
                    })
                }
                Err(e) => {
                    log::error!("[Updater] 检查更新失败: {}", e);
                    Err(format!("检查更新失败: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("[Updater] 获取 updater 实例失败: {}", e);
            Err(format!("检查更新失败: {}", e))
        }
    }
}

/// 下载并安装更新 - 使用 Tauri 官方 updater 插件
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    log::info!("[Updater] 开始下载并安装更新...");

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    log::info!("[Updater] 开始下载更新: {}", update.version);

                    // 下载并安装更新
                    match update.download_and_install(|_, _| {}, || {}).await {
                        Ok(_) => {
                            log::info!("[Updater] 更新下载并安装成功");
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("[Updater] 下载或安装更新失败: {}", e);
                            Err(format!("下载或安装更新失败: {}", e))
                        }
                    }
                }
                Ok(None) => {
                    log::info!("[Updater] 没有可用更新");
                    Err("没有可用更新".to_string())
                }
                Err(e) => {
                    log::error!("[Updater] 检查更新失败: {}", e);
                    Err(format!("检查更新失败: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("[Updater] 获取 updater 实例失败: {}", e);
            Err(format!("检查更新失败: {}", e))
        }
    }
}

/// 获取当前应用版本
#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
