//! Back-end w Rust dla „Ogulniega Full Launcher” (Tauri v2).

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::fetch_json, commands::list_versions, commands::download_file, commands::sha512_of, commands::backend_info])
        .run(tauri::generate_context!())
        .expect("błąd uruchamiania aplikacji Tauri");
}
