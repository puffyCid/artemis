use crate::about::info;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![info::about_me])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
