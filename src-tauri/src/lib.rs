use tauri_plugin_fs::FsExt;
use tauri_plugin_dialog::DialogExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/**
 * Using Rust directly for folder selection so that I may allow fs for all child directories recursively.
 */
#[tauri::command]
async fn select_and_allow_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    // Open the directory picker dialog
    // blocking_pick_folder returns Option<PathBuf>
    let selected_path = app.dialog().file().blocking_pick_folder();

    match selected_path {
        Some(path) => {
            // Convert FilePath to PathBuf
            // .into_path() handles both regular paths and file:// URLs
            let path_buf = path.into_path()
                .map_err(|_| "Invalid path format".to_string())?;

            // Add the selected directory to the FS scope recursively
            // true = allows access to the directory and all subdirectories/files
            app.fs_scope()
                .allow_directory(&path_buf, true)
                .map_err(|e| format!("Failed to update scope: {}", e))?;

            // Return the path as a string to the frontend
            Ok(path_buf.to_str().map(|s| s.to_string()))
        }
        None => {
            // User cancelled the dialog
            Ok(None)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![greet, select_and_allow_directory])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
