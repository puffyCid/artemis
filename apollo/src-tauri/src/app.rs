use crate::about::info;
use crate::timeline::query;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            info::about_me,
            info::metadata,
            query::query_timeline,
            query::list_artifacts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
