use tauri_plugin_fs::FsExt;
use tauri_plugin_dialog::DialogExt;
use std::path::Path;
use std::fs;
use walkdir::WalkDir;
use serde::Serialize;
use tauri::{Emitter};

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

#[derive(Clone, Serialize)]
struct CopyProgress {
    total_files: usize,
    copied_files: usize,
    current_file: String,
}

#[tauri::command]
async fn copy_folder_with_progress(app: tauri::AppHandle, source: String, destination: String) -> Result<(), String> {
    let src_path = Path::new(&source);
    let dest_path = Path::new(&destination);

    if !src_path.exists() {
        return Err("Source path does not exist".to_string());
    }

    if !dest_path.exists() {
        fs::create_dir_all(dest_path).map_err(|e| e.to_string())?;
    }

    let total_files = WalkDir::new(src_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let mut copied_files = 0;

    for entry in WalkDir::new(src_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Calculate where the file/folder should go in the destination
        let relative_path = path.strip_prefix(src_path).map_err(|e| e.to_string())?;
        let target_path = dest_path.join(relative_path);

        if entry.file_type().is_dir() {
            // Create directories as we encounter them
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            // Copy the file
            fs::copy(path, &target_path).map_err(|e| e.to_string())?;
            copied_files += 1;

            // Step 3: Emit the progress state back to the frontend
            let payload = CopyProgress {
                total_files,
                copied_files,
                current_file: entry.file_name().to_string_lossy().into_owned(),
            };

            // "copy-progress" is the event name the frontend will listen for
            app.emit("copy-progress", payload).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![greet, select_and_allow_directory, copy_folder_with_progress])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
