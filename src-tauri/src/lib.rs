//! Back-end w Rust dla „Ogulniega Full Launcher” (Tauri v2).

use std::sync::Arc;

pub mod commands;
pub mod download;
pub mod error;

use commands::AppState;
use download::Downloader;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Katalog danych aplikacji: na desktopie `~/.local/share/<id>`,
            // na Androidzie prywatny sandbox apki — jedyny dozwolony katalog zapisu.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let downloader = Downloader::new()?;
            app.manage(AppState {
                downloader: Arc::new(downloader),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_json,
            commands::list_versions,
            commands::download_file,
            commands::sha512_of,
            commands::backend_info
        ])
        .run(tauri::generate_context!())
        .expect("błąd uruchamiania aplikacji Tauri");
}
