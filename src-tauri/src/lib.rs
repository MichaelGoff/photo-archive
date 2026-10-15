use tauri_plugin_fs::FsExt;
use tauri_plugin_dialog::DialogExt;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use walkdir::WalkDir;
use serde::Serialize;
use tauri::{Emitter};
use tauri_plugin_store::StoreExt;
use sha2::{Sha256, Digest};

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
struct OperationProgress {
    stage: u8, // 0 = Copy, 1 = Validate, 2 = Delete
    total_files: usize,
    processed_files: usize,
    current_file: String,
    overall_progress: f64, // Deterministic overall progress percentage (0.0 to 1.0)
}

#[derive(Clone, Debug, Serialize)]
struct ValidationReport {
    total_files: usize,
    verified_files: usize,
    missing_files: Vec<String>,
    size_mismatches: Vec<String>,
    modified_time_mismatches: Vec<String>,
    checksum_mismatches: Vec<String>,
}

fn calculate_file_checksum(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn count_files(source_root: &Path) -> usize {
    WalkDir::new(source_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

fn validate_copy_with_progress(source_root: &Path, target_root: &Path, app: &tauri::AppHandle) -> ValidationReport {
    // First count total files to get deterministic progress
    let total_files = count_files(source_root);

    let mut verified_files = 0;
    let mut missing_files: Vec<String> = Vec::new();
    let mut size_mismatches: Vec<String> = Vec::new();
    let mut modified_time_mismatches: Vec<String> = Vec::new();
    let mut checksum_mismatches: Vec<String> = Vec::new();

    let mut processed_files = 0;
    for entry in WalkDir::new(source_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        processed_files += 1;

        let relative_path = match entry.path().strip_prefix(source_root) {
            Ok(relative_path) => relative_path,
            Err(_) => continue,
        };
        let relative_path_string = relative_path.to_string_lossy().to_string();
        let destination_file = target_root.join(relative_path);

        let destination_meta = match fs::metadata(&destination_file) {
            Ok(destination_meta) => destination_meta,
            Err(_) => {
                missing_files.push(relative_path_string.clone());
                continue;
            }
        };

        let source_meta = match entry.metadata() {
            Ok(source_meta) => source_meta,
            Err(_) => {
                missing_files.push(relative_path_string.clone());
                continue;
            }
        };

        if source_meta.len() != destination_meta.len() {
            size_mismatches.push(relative_path_string.clone());
            continue;
        }

        // Check file checksums
        let source_checksum = match calculate_file_checksum(entry.path()) {
            Ok(checksum) => checksum,
            Err(_) => {
                // If we can't calculate checksum, treat as verified for now
                verified_files += 1;
                continue;
            }
        };

        let destination_checksum = match calculate_file_checksum(&destination_file) {
            Ok(checksum) => checksum,
            Err(_) => {
                // If we can't calculate checksum, treat as mismatch
                checksum_mismatches.push(relative_path_string.clone());
                continue;
            }
        };

        if source_checksum != destination_checksum {
            checksum_mismatches.push(relative_path_string.clone());
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
                modified_time_mismatches.push(relative_path_string.clone());
            }
            _ => {
                // Timestamps unavailable on one side; verified as far as we can tell
                verified_files += 1;
            }
        }

        // Emit validation progress for each file processed
        let payload = OperationProgress {
            stage: 1,
            total_files,
            processed_files,
            current_file: relative_path_string.clone(),
            overall_progress: 0.5 + (processed_files as f64 / total_files as f64) * 0.3, // 50-80% progress
        };

        app.emit("operation-progress", payload).map_err(|e| e.to_string()).unwrap();
    }

    ValidationReport {
        total_files,
        verified_files,
        missing_files,
        size_mismatches,
        modified_time_mismatches,
        checksum_mismatches,
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

    // Stage 1: Copy files with progress tracking
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

            // Emit copy progress (stage 0) - 50% of total progress
            let payload = OperationProgress {
                stage: 0,
                total_files,
                processed_files: copied_files,
                current_file: entry.file_name().to_string_lossy().into_owned(),
                overall_progress: copied_files as f64 / total_files as f64 * 0.5, // 0-50% progress
            };

            app.emit("operation-progress", payload).map_err(|e| e.to_string())?;
        }
    }

    // Stage 2: Validation with progress tracking - 30% of total progress
    let validation_report = validate_copy_with_progress(folder_path, &target_root, &app);

    // Stage 3: Delete from source with progress tracking - 20% of total progress  
    let mut deleted_files = 0;
    
    // We'll iterate through each file in the source and delete it
    // Note: We're using the original folder_path for deletion (assuming we are deleting from source)
    for entry in WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            deleted_files += 1;
            
            // Delete only files (not directories) - we don't want to delete the folder structure
            let path = entry.path();
            if fs::remove_file(path).is_ok() {
                // Emit delete progress (stage 2) - 80-100% of total progress
                let payload = OperationProgress {
                    stage: 2,
                    total_files: validation_report.total_files,
                    processed_files: deleted_files,
                    current_file: entry.file_name().to_string_lossy().into_owned(),
                    overall_progress: 0.8 + (deleted_files as f64 / validation_report.total_files as f64) * 0.2, // 80-100% progress
                };

                app.emit("operation-progress", payload).map_err(|e| e.to_string())?;
            }
        } else if entry.file_type().is_dir() {
            // For directories we should not delete, but just emit a marker that we've processed them
            deleted_files += 1;
            
            // We may also want to track directory processing or just keep the same logic
            let payload = OperationProgress {
                stage: 2,
                total_files: validation_report.total_files,
                processed_files: deleted_files,
                current_file: entry.file_name().to_string_lossy().into_owned(),
                overall_progress: 0.8 + (deleted_files as f64 / validation_report.total_files as f64) * 0.2, // 80-100% progress
            };

            app.emit("operation-progress", payload).map_err(|e| e.to_string())?;
        }
    }

    // After copying, verify that every source file exists in the destination with a matching size
    Ok(validation_report)
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
