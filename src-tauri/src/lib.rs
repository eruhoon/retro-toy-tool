pub mod models;
pub mod ssh_client;
pub mod gamelist_xml;
pub mod rom_scanner;
pub mod scraper;
pub mod emulator_core;
pub mod commands;
pub mod updater;

use commands::*;
use updater::*;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => {
                    eprintln!(">>> Window 'main' not found! Creating programmatically via WebviewWindowBuilder...");
                    WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                        .title("Toy Manager")
                        .inner_size(1280.0, 820.0)
                        .min_inner_size(960.0, 640.0)
                        .center()
                        .visible(true)
                        .focused(true)
                        .build()
                        .map_err(|e| {
                            eprintln!(">>> Failed to build window: {:?}", e);
                            e
                        })?
                }
            };

            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            println!(">>> Toy Manager window ready and displayed!");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_connection,
            ping_device,
            detect_storages_cmd,
            get_systems,
            get_system_games,
            save_system_games_cmd,
            upload_game_image_cmd,
            fetch_remote_image_cmd,
            fetch_remote_video_cmd,
            upload_game_video_cmd,
            search_game_metadata_cmd,
            test_screenscraper_account_cmd,
            download_and_upload_scraped_image_cmd,
            download_and_upload_scraped_video_cmd,
            upload_rom_files,
            get_installed_cores_cmd,
            check_update,
            install_update
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

