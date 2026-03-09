use crate::backup::{BackupEntry, BackupManager};
use crate::engines::types::*;
use crate::engines::EngineRegistry;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub registry: EngineRegistry,
    pub current_engine: Mutex<Option<String>>,
    pub current_game_dir: Mutex<Option<String>>,
    pub last_loaded_save: Mutex<Option<(String, serde_json::Value)>>,
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
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;
    engine.list_saves(Path::new(&game_dir))
}

#[tauri::command]
pub fn load_save(state: State<AppState>, save_path: String) -> Result<SaveData, String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;
    let data = engine.parse_save(Path::new(&save_path), Path::new(&game_dir))?;

    *state.last_loaded_save.lock().unwrap() = Some((save_path, data.raw.clone()));

    Ok(data)
}

#[tauri::command]
pub fn save_file(
    state: State<AppState>,
    save_path: String,
    data: SaveData,
) -> Result<String, String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;

    let path = Path::new(&save_path);
    let backup = BackupManager::create_backup(path)?;
    engine.write_save(path, &data)?;

    Ok(format!("Saved. Backup at: {}", backup.display()))
}

#[tauri::command]
pub fn get_names(state: State<AppState>) -> Result<NameMap, String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;
    engine.resolve_names(Path::new(&game_dir))
}

#[tauri::command]
pub fn get_diff(state: State<AppState>, save_path: String) -> Result<Vec<DiffEntry>, String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;

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

#[tauri::command]
pub fn apply_debug_patch(state: State<AppState>) -> Result<PatchInfo, String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;
    engine.apply_debug_patch(Path::new(&game_dir))
}

#[tauri::command]
pub fn revert_debug_patch(state: State<AppState>, patch: PatchInfo) -> Result<(), String> {
    let engine_id = state
        .current_engine
        .lock()
        .unwrap()
        .clone()
        .ok_or("No engine selected")?;
    let engine = state
        .registry
        .get_engine(&engine_id)
        .ok_or("Engine not found")?;
    let game_dir = state
        .current_game_dir
        .lock()
        .unwrap()
        .clone()
        .ok_or("No game directory selected")?;
    engine.revert_debug_patch(Path::new(&game_dir), &patch)
}
