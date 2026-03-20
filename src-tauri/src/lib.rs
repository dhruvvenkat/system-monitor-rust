mod commands;

use tauri::Manager;

pub use commands::{MonitorQuery, MonitorSnapshot};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![commands::monitor_snapshot])
        .setup(|app| {
            // The window is configured in tauri.conf.json; this just keeps startup explicit.
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
