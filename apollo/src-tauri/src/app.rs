use crate::about::info;
use crate::timeline::{query, update, upload};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            info::about_me,
            info::metadata,
            query::query_timeline,
            query::list_artifacts,
            update::apply_tag,
            upload::timeline_and_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
