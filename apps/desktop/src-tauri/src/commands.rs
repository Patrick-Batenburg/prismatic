use crate::backup::{BackupEntry, BackupManager};
use crate::engines::types::*;
use crate::engines::EngineRegistry;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::State;

/// Cache for deep scan results: (dir_path, extension) -> (file_count, dir_modified_time)
pub type ScanCache = HashMap<(String, String), (usize, SystemTime)>;

pub struct AppState {
    pub registry: EngineRegistry,
    pub current_engine: Mutex<Option<String>>,
    pub current_game_dir: Mutex<Option<String>>,
    pub last_loaded_save: Mutex<Option<(String, serde_json::Value)>>,
    pub scan_cache: Mutex<ScanCache>,
}

impl AppState {
    fn engine_and_dir(&self) -> Result<(std::sync::Arc<dyn crate::engines::EnginePlugin>, String), String> {
        let engine_id = self
            .current_engine
            .lock()
            .unwrap()
            .clone()
            .ok_or("No engine selected")?;
        let game_dir = self
            .current_game_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or("No game directory selected")?;
        let engine = self
            .registry
            .get_engine(&engine_id)
            .ok_or("Engine not found")?;
        Ok((engine, game_dir))
    }
}

#[tauri::command]
pub fn list_engines(state: State<AppState>) -> Vec<EngineInfo> {
    state.registry.list_engines()
}

#[tauri::command]
pub fn detect_engine(state: State<AppState>, game_dir: String) -> Option<EngineInfo> {
    let path = Path::new(&game_dir);
    state.registry.detect_engine(path).map(|e| e.info())
}

#[tauri::command]
pub fn set_game(state: State<AppState>, engine_id: String, game_dir: String) -> Result<(), String> {
    *state.current_engine.lock().unwrap() = Some(engine_id);
    *state.current_game_dir.lock().unwrap() = Some(game_dir);
    Ok(())
}

#[tauri::command]
pub fn list_saves(state: State<AppState>) -> Result<Vec<SaveFile>, String> {
    let (engine, game_dir) = state.engine_and_dir()?;
    engine.list_saves(Path::new(&game_dir))
}

// Heavy commands are async so they run on a background thread,
// keeping the window responsive and allowing skeleton loaders to show.

#[tauri::command]
pub async fn load_save(state: State<'_, AppState>, save_path: String) -> Result<SaveData, String> {
    let (engine, game_dir) = state.engine_and_dir()?;
    let save_path_clone = save_path.clone();
    let data = engine.parse_save(Path::new(&save_path_clone), Path::new(&game_dir))?;

    *state.last_loaded_save.lock().unwrap() = Some((save_path, data.raw.clone()));

    Ok(data)
}

#[tauri::command]
pub async fn save_file(
    state: State<'_, AppState>,
    save_path: String,
    data: SaveData,
) -> Result<String, String> {
    let (engine, _) = state.engine_and_dir()?;

    let path = Path::new(&save_path);
    let backup = BackupManager::create_backup(path)?;
    engine.write_save(path, &data)?;

    Ok(format!("Saved. Backup at: {}", backup.display()))
}

#[tauri::command]
pub async fn get_names(state: State<'_, AppState>) -> Result<NameMap, String> {
    let (engine, game_dir) = state.engine_and_dir()?;
    engine.resolve_names(Path::new(&game_dir))
}

#[tauri::command]
pub fn get_diff(state: State<AppState>, save_path: String) -> Result<Vec<DiffEntry>, String> {
    let (engine, game_dir) = state.engine_and_dir()?;

    let last = state.last_loaded_save.lock().unwrap();
    let (_, old_raw) = last.as_ref().ok_or("No previous save loaded for diff")?;

    let new_data = engine.parse_save(Path::new(&save_path), Path::new(&game_dir))?;
    let diff = compute_diff(old_raw, &new_data.raw, "");
    Ok(diff)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

fn compute_diff(old: &serde_json::Value, new: &serde_json::Value, path: &str) -> Vec<DiffEntry> {
    let mut diffs = Vec::new();

    match (old, new) {
        (serde_json::Value::Object(o), serde_json::Value::Object(n)) => {
            for (key, old_val) in o {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                if let Some(new_val) = n.get(key) {
                    diffs.extend(compute_diff(old_val, new_val, &child_path));
                } else {
                    diffs.push(DiffEntry {
                        path: child_path,
                        old_value: Some(old_val.clone()),
                        new_value: None,
                    });
                }
            }
            for (key, new_val) in n {
                if !o.contains_key(key) {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    diffs.push(DiffEntry {
                        path: child_path,
                        old_value: None,
                        new_value: Some(new_val.clone()),
                    });
                }
            }
        }
        (serde_json::Value::Array(o), serde_json::Value::Array(n)) => {
            let max_len = o.len().max(n.len());
            for i in 0..max_len {
                let child_path = format!("{path}[{i}]");
                match (o.get(i), n.get(i)) {
                    (Some(ov), Some(nv)) => diffs.extend(compute_diff(ov, nv, &child_path)),
                    (Some(ov), None) => diffs.push(DiffEntry {
                        path: child_path,
                        old_value: Some(ov.clone()),
                        new_value: None,
                    }),
                    (None, Some(nv)) => diffs.push(DiffEntry {
                        path: child_path,
                        old_value: None,
                        new_value: Some(nv.clone()),
                    }),
                    (None, None) => {}
                }
            }
        }
        _ => {
            if old != new {
                diffs.push(DiffEntry {
                    path: path.to_string(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                });
            }
        }
    }

    diffs
}

#[tauri::command]
pub fn list_backups(save_path: String) -> Result<Vec<BackupEntry>, String> {
    BackupManager::list_backups(Path::new(&save_path))
}

#[tauri::command]
pub fn restore_backup(backup_path: String, save_path: String) -> Result<(), String> {
    BackupManager::restore_backup(Path::new(&backup_path), Path::new(&save_path))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveDirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub file_count: usize,
}

fn expand_env(path: &str) -> String {
    let mut result = path.to_string();
    if let Ok(appdata) = std::env::var("APPDATA") {
        result = result.replace("%APPDATA%", &appdata);
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        result = result.replace("%LOCALAPPDATA%", &localappdata);
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        result = result.replace("%USERPROFILE%", &userprofile);
    }
    result
}

#[tauri::command]
pub async fn browse_save_dir(
    dir: Option<String>,
    default_dir: Option<String>,
    extension: String,
) -> Result<(String, Vec<SaveDirEntry>), String> {
    let base = match dir {
        Some(d) => std::path::PathBuf::from(d),
        None => match default_dir {
            Some(dd) => std::path::PathBuf::from(expand_env(&dd)),
            None => return Err("No directory specified and no default directory provided".to_string()),
        },
    };

    if !base.is_dir() {
        return Err(format!("Directory not found: {}", base.display()));
    }

    let mut entries = Vec::new();
    let read = std::fs::read_dir(&base)
        .map_err(|e| format!("Failed to read {}: {e}", base.display()))?;

    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();

        if is_dir {
            // Only count matching files in immediate children (not recursive)
            // to avoid freezing on large directory trees like %LOCALAPPDATA%
            let file_count = count_files_shallow(&path, &extension);
            entries.push(SaveDirEntry {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir,
                file_count,
            });
        } else if path.extension().map(|e| e == extension.as_str()).unwrap_or(false) {
            entries.push(SaveDirEntry {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir: false,
                file_count: 0,
            });
        }
    }

    entries.sort_by(|a, b| {
        // Directories first, then by name
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });

    Ok((base.to_string_lossy().to_string(), entries))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanProgressEvent {
    pub path: String,
    pub file_count: usize,
    pub folders_done: usize,
    pub folders_total: usize,
}

fn count_files_recursive_cached(
    dir: &Path,
    extension: &str,
    cache: &Mutex<ScanCache>,
) -> usize {
    let dir_str = dir.to_string_lossy().to_string();
    let ext_str = extension.to_string();

    // Check cache: if dir modified time matches, use cached count
    if let Ok(meta) = std::fs::metadata(dir) {
        if let Ok(modified) = meta.modified() {
            let cache_guard = cache.lock().unwrap();
            if let Some(&(count, cached_time)) = cache_guard.get(&(dir_str.clone(), ext_str.clone())) {
                if cached_time == modified {
                    return count;
                }
            }
        }
    }

    // Not cached or stale — count recursively
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files_recursive_cached(&path, extension, cache);
            } else if path.extension().map(|e| e == extension).unwrap_or(false) {
                count += 1;
            }
        }
    }

    // Store in cache
    if let Ok(meta) = std::fs::metadata(dir) {
        if let Ok(modified) = meta.modified() {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert((dir_str, ext_str), (count, modified));
        }
    }

    count
}

#[tauri::command]
pub async fn deep_scan_dir(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    dir: String,
    extension: String,
) -> Result<(), String> {
    use tauri::Emitter;

    let base = std::path::PathBuf::from(&dir);
    if !base.is_dir() {
        return Err(format!("Directory not found: {}", base.display()));
    }

    // Collect subdirectories first
    let subdirs: Vec<std::path::PathBuf> = std::fs::read_dir(&base)
        .map_err(|e| format!("Failed to read {}: {e}", base.display()))?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();

    let total = subdirs.len();

    // Clone the cache for the scan operation
    let scan_cache = state.scan_cache.lock().unwrap().clone();
    let cache = std::sync::Arc::new(Mutex::new(scan_cache));

    // Scan each subdirectory and emit progress
    for (i, subdir) in subdirs.iter().enumerate() {
        let count = count_files_recursive_cached(subdir, &extension, &cache);

        let _ = app.emit("scan-progress", ScanProgressEvent {
            path: subdir.to_string_lossy().to_string(),
            file_count: count,
            folders_done: i + 1,
            folders_total: total,
        });
    }

    // Write updated cache back to AppState
    let updated = std::sync::Arc::try_unwrap(cache)
        .unwrap_or_else(|arc| Mutex::new(arc.lock().unwrap().clone()))
        .into_inner()
        .unwrap();
    *state.scan_cache.lock().unwrap() = updated;

    let _ = app.emit("scan-complete", ());

    Ok(())
}

/// Count matching files in a directory's immediate children only (no recursion).
fn count_files_shallow(dir: &Path, extension: &str) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() && path.extension().map(|e| e == extension).unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

#[tauri::command]
pub fn apply_debug_patch(state: State<AppState>) -> Result<PatchInfo, String> {
    let (engine, game_dir) = state.engine_and_dir()?;
    engine.apply_debug_patch(Path::new(&game_dir))
}

#[tauri::command]
pub fn revert_debug_patch(state: State<AppState>, patch: PatchInfo) -> Result<(), String> {
    let (engine, game_dir) = state.engine_and_dir()?;
    engine.revert_debug_patch(Path::new(&game_dir), &patch)
}
