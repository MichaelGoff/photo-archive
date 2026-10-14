use tauri_plugin_fs::FsExt;
use tauri_plugin_dialog::DialogExt;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use walkdir::WalkDir;
use serde::Serialize;
use tauri::{Emitter};
use tauri_plugin_store::StoreExt;

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

#[derive(Clone, Debug, Serialize)]
struct ValidationReport {
    total_files: usize,
    verified_files: usize,
    missing_files: Vec<String>,
    size_mismatches: Vec<String>,
    modified_time_mismatches: Vec<String>,
}

fn validate_copy(source_root: &Path, target_root: &Path) -> ValidationReport {
    let mut total_files = 0;
    let mut verified_files = 0;
    let mut missing_files: Vec<String> = Vec::new();
    let mut size_mismatches: Vec<String> = Vec::new();
    let mut modified_time_mismatches: Vec<String> = Vec::new();

    for entry in WalkDir::new(source_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        total_files += 1;

        let relative_path = match entry.path().strip_prefix(source_root) {
            Ok(relative_path) => relative_path,
            Err(_) => continue,
        };
        let relative_path_string = relative_path.to_string_lossy().to_string();
        let destination_file = target_root.join(relative_path);

        let destination_meta = match fs::metadata(&destination_file) {
            Ok(destination_meta) => destination_meta,
            Err(_) => {
                missing_files.push(relative_path_string);
                continue;
            }
        };

        let source_meta = match entry.metadata() {
            Ok(source_meta) => source_meta,
            Err(_) => {
                missing_files.push(relative_path_string);
                continue;
            }
        };

        if source_meta.len() != destination_meta.len() {
            size_mismatches.push(relative_path_string);
            continue;
        }

        // Soft check: some filesystems don't support modified-time queries, so skip the
        // comparison if either side is unavailable rather than reporting a false mismatch.
        match (source_meta.modified().ok(), destination_meta.modified().ok()) {
            (Some(source_modified), Some(destination_modified))
                if source_modified == destination_modified =>
            {
                verified_files += 1;
            }
            (Some(_), Some(_)) => {
                modified_time_mismatches.push(relative_path_string);
            }
            _ => {
                // Timestamps unavailable on one side; verified as far as we can tell
                verified_files += 1;
            }
        }
    }

    ValidationReport {
        total_files,
        verified_files,
        missing_files,
        size_mismatches,
        modified_time_mismatches,
    }
}

#[tauri::command]
async fn copy_folder_with_progress(
    app: tauri::AppHandle,
    folder_path_raw: String,
) -> Result<ValidationReport, String> {
    let folder_path = Path::new(&folder_path_raw);

    if !folder_path.exists() {
        return Err("Folder path does not exist".to_string());
    }

    // Pull the source and destination paths from the store
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    let source_path: PathBuf = store
        .get("sourcePath")
        .and_then(|val| val.as_str().map(PathBuf::from))
        .ok_or_else(|| "sourcePath missing or not a string".to_string())?;

    let dest_path: PathBuf = store
        .get("destinationPath")
        .and_then(|val| val.as_str().map(PathBuf::from))
        .ok_or_else(|| "destinationPath missing or not a string".to_string())?;

    // The folderPath is inside the sourcePath, so we need to calculate the relative path
    // from sourcePath to folderPath. This will allow us to maintain the same folder structure
    // in the destinationPath
    let rel_source_folder_path = folder_path
        .strip_prefix(&source_path)
        .map_err(|_| format!("folderPath {:?} is not inside sourcePath {:?}", folder_path, source_path))?;

    // Now re-build that path in our destination
    let target_root = dest_path.join(rel_source_folder_path);

    if !target_root.exists() {
        fs::create_dir_all(&target_root).map_err(|e| e.to_string())?;
    }

    let total_files = WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let mut copied_files = 0;

    for entry in WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Calculate where the file/folder should go in the destination
        let relative_path = path.strip_prefix(folder_path).map_err(|e| e.to_string())?;
        let target_path = target_root.join(relative_path);

        if entry.file_type().is_dir() {
            // Create directories as we encounter them
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            fs::copy(path, &target_path).map_err(|e| e.to_string())?;
            copied_files += 1;

            // Emit the progress state back to the frontend
            let payload = CopyProgress {
                total_files,
                copied_files,
                current_file: entry.file_name().to_string_lossy().into_owned(),
            };

            app.emit("copy-progress", payload).map_err(|e| e.to_string())?;
        }
    }

    // After copying, verify that every source file exists in the destination with a matching size
    Ok(validate_copy(folder_path, &target_root))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_copy_detects_missing_and_truncated_files() {
        let base = std::env::temp_dir().join(format!(
            "photo_archive_validate_test_{}",
            std::process::id()
        ));
        let source = base.join("source");
        let destination = base.join("destination");

        // Clean up from any previous failed run, then build a small source tree
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(destination.join("sub")).unwrap();
        std::fs::write(source.join("a.txt"), b"hello").unwrap();
        std::fs::write(source.join("b.txt"), b"world!").unwrap();
        std::fs::write(source.join("sub/c.txt"), b"nested").unwrap();

        // Copy everything, as the real command would (fs::copy preserves mtimes)
        for entry in WalkDir::new(&source).into_iter().filter_map(|e| e.ok()) {
            let relative_path = entry.path().strip_prefix(&source).unwrap();
            let target_path = destination.join(relative_path);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&target_path).unwrap();
            } else {
                std::fs::copy(entry.path(), &target_path).unwrap();
            }
        }

        // Perfect state: everything verified
        let report = validate_copy(&source, &destination);
        assert_eq!(report.total_files, 3);
        assert_eq!(report.verified_files, 3);
        assert!(report.missing_files.is_empty());
        assert!(report.size_mismatches.is_empty());
        assert!(report.modified_time_mismatches.is_empty());

        // Break one file (truncation) and remove another
        std::fs::write(destination.join("b.txt"), b"wo").unwrap();
        std::fs::remove_file(destination.join("sub/c.txt")).unwrap();

        let report = validate_copy(&source, &destination);
        assert_eq!(report.total_files, 3);
        assert_eq!(report.verified_files, 1);
        // Compare paths component-wise so this works regardless of '/' vs '\' separators
        let missing: Vec<PathBuf> = report.missing_files.iter().map(PathBuf::from).collect();
        let size_mismatches: Vec<PathBuf> = report.size_mismatches.iter().map(PathBuf::from).collect();
        assert_eq!(missing, vec![PathBuf::from("sub").join("c.txt")]);
        assert_eq!(size_mismatches, vec![PathBuf::from("b.txt")]);

        // Tidy up
        let _ = std::fs::remove_dir_all(&base);
    }
}
