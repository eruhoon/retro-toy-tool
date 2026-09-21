pub mod models;
pub mod ssh_client;
pub mod gamelist_xml;
pub mod rom_scanner;
pub mod scraper;
pub mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            test_connection,
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
            upload_rom_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

