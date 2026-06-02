// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ai_image_v2_lib::commands;
use ai_image_v2_lib::{setup_logging, log_message, config_store};

fn main() {
    setup_logging();
    
    log_message("应用程序启动");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            // 应用启动时初始化配置
            log_message("[Main] 初始化配置存储...");
            if let Err(e) = config_store::init_config_store() {
                log_message(&format!("[Main] 配置存储初始化失败: {}", e));
            } else {
                log_message("[Main] 配置存储初始化成功");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate_image,
            commands::batch_generate_images,
            commands::optimize_prompt,
            commands::get_images,
            commands::delete_image,
            commands::open_output_dir,
            commands::load_config,
            commands::save_config,
            commands::get_provider_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
