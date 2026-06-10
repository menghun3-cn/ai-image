// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ai_image_v2_lib::commands;
use ai_image_v2_lib::{agnes_models, config_store, log_message, setup_logging};
use tauri::Manager;

fn main() {
    setup_logging();

    log_message("应用程序启动");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 当检测到已有实例运行时，显示已运行的窗口
            log_message("检测到已有实例运行，显示已存在的窗口");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
                let _ = window.show();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 应用启动时初始化配置
            log_message("[Main] 初始化配置存储...");
            if let Err(e) = config_store::init_config_store() {
                log_message(&format!("[Main] 配置存储初始化失败: {}", e));
            } else {
                log_message("[Main] 配置存储初始化成功");
            }

            // 启动时异步拉取 Agnes 模型
            let _app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                log_message("[Main] 准备启动时拉取 Agnes 模型...");

                // 加载配置获取 endpoint 和 api_key
                match config_store::load_config_from_store() {
                    Ok(config) => {
                        let endpoint = if config.providers.agnes.endpoint.is_empty() {
                            "https://apihub.agnes-ai.com/v1".to_string()
                        } else {
                            config.providers.agnes.endpoint.clone()
                        };
                        let api_key = config.providers.agnes.api_key.clone();

                        // 异步拉取模型（失败不阻塞）
                        agnes_models::try_fetch_on_startup(&endpoint, &api_key).await;
                    }
                    Err(e) => {
                        log_message(&format!("[Main] 加载配置失败，跳过启动拉取: {}", e));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate_image,
            commands::batch_generate_images,
            commands::optimize_prompt,
            commands::get_images,
            commands::refresh_images,
            commands::get_directory_mtime,
            commands::delete_image,
            commands::open_output_dir,
            commands::load_config,
            commands::save_config,
            commands::get_provider_models,
            commands::generate_video,
            commands::get_video_output_dir,
            commands::update_agnes_models,
            commands::get_agnes_models,
            commands::get_default_agnes_models,
            commands::get_videos,
            commands::refresh_videos,
            commands::delete_video,
            commands::open_video_dir,
            commands::fetch_provider_models,
            commands::check_update,
            commands::download_and_install_update,
            commands::get_app_version,
            commands::pick_folder,
            commands::get_default_storage_paths,
            commands::open_log_dir,
            commands::get_log_content,
            commands::retry_download_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
