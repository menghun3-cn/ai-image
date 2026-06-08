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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub status: String,
    pub progress: u64,
    pub total: Option<u64>,
    pub message: String,
}

/// 检查更新
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    log::info!("[Updater] 开始检查更新...");

    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("构建更新器失败: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            log::info!("[Updater] 发现新版本: {}", update.version);

            // 获取当前版本
            let current_version = app.package_info().version.to_string();

            Ok(UpdateInfo {
                version: update.version.to_string(),
                current_version,
                notes: update.body.clone(),
                pub_date: update.date.map(|d| d.to_string()),
                has_update: true,
            })
        }
        Ok(None) => {
            log::info!("[Updater] 当前已是最新版本");

            let current_version = app.package_info().version.to_string();

            Ok(UpdateInfo {
                version: current_version.clone(),
                current_version,
                notes: None,
                pub_date: None,
                has_update: false,
            })
        }
        Err(e) => {
            log::error!("[Updater] 检查更新失败: {}", e);
            Err(format!("检查更新失败: {}", e))
        }
    }
}

/// 下载并安装更新
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    log::info!("[Updater] 开始下载并安装更新...");

    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| format!("构建更新器失败: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            log::info!("[Updater] 开始下载更新: {}", update.version);

            // 下载并安装更新
            update
                .download_and_install(|chunk, content_length| {
                    let downloaded = chunk.len() as u64;
                    let total = content_length.unwrap_or(0);
                    let percent = if total > 0 {
                        (downloaded as f64 / total as f64 * 100.0) as u64
                    } else {
                        0
                    };
                    log::info!("[Updater] 下载进度: {}%", percent);
                }, || {
                    log::info!("[Updater] 下载完成，准备安装...");
                })
                .await
                .map_err(|e| format!("下载或安装更新失败: {}", e))?;

            log::info!("[Updater] 更新安装完成，准备重启...");
            Ok(())
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

/// 获取当前应用版本
#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
