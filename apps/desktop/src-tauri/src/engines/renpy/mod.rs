mod pickle;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use chrono::Local;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct RenpyPlugin;

impl RenpyPlugin {
    /// Find the game directory containing renpy files
    fn find_game_subdir(game_dir: &Path) -> PathBuf {
        let game_sub = game_dir.join("game");
        if game_sub.exists() {
            game_sub
        } else {
            game_dir.to_path_buf()
        }
    }

    /// Find saves directory — either game/saves/ or %APPDATA%/RenPy/<save_dir>/
    fn find_save_dirs(game_dir: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Local saves
        let local_saves = Self::find_game_subdir(game_dir).join("saves");
        if local_saves.is_dir() {
            dirs.push(local_saves);
        }

        // AppData saves — try to find config.save_directory from options.rpy
        if let Some(save_dir_name) = Self::find_save_directory_name(game_dir) {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let appdata_saves = PathBuf::from(appdata).join("RenPy").join(&save_dir_name);
                if appdata_saves.is_dir() {
                    dirs.push(appdata_saves);
                }
            }
        }

        // Also check AppData/RenPy/ for any directory matching the game name
        if dirs.is_empty() {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let renpy_dir = PathBuf::from(appdata).join("RenPy");
                if renpy_dir.is_dir() {
                    // Try to find a matching directory
                    let game_name = game_dir
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if let Ok(entries) = fs::read_dir(&renpy_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            if name.contains(&game_name) || game_name.contains(&name) {
                                dirs.push(entry.path());
                            }
                        }
                    }
                }
            }
        }

        dirs
    }

    /// Parse options.rpy to find config.save_directory
    fn find_save_directory_name(game_dir: &Path) -> Option<String> {
        let game_sub = Self::find_game_subdir(game_dir);

        // Try options.rpy
        for filename in &["options.rpy", "options.rpyc"] {
            let path = game_sub.join(filename);
            if path.exists() && filename.ends_with(".rpy") {
                if let Ok(content) = fs::read_to_string(&path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("define config.save_directory") {
                            // Extract the string value
                            if let Some(start) = trimmed.find('"') {
                                if let Some(end) = trimmed[start + 1..].find('"') {
                                    return Some(trimmed[start + 1..start + 1 + end].to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Parse a .save ZIP file and extract the JSON metadata + raw log data
    fn parse_save_zip(save_path: &Path) -> Result<(serde_json::Value, serde_json::Value), String> {
        let file = fs::File::open(save_path)
            .map_err(|e| format!("Failed to open save file: {e}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to open ZIP archive: {e}"))?;

        // Read JSON metadata
        let metadata = if let Ok(mut entry) = archive.by_name("json") {
            let mut content = String::new();
            entry.read_to_string(&mut content)
                .map_err(|e| format!("Failed to read json entry: {e}"))?;
            serde_json::from_str(&content)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // Read log (pickle data) — attempt to deserialize with serde-pickle
        let log_data = if let Ok(mut entry) = archive.by_name("log") {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)
                .map_err(|e| format!("Failed to read log entry: {e}"))?;

            // Parse pickle with our custom VM that handles Ren'Py types
            match pickle::parse_pickle(&bytes) {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("[renpy] pickle parse error: {e}");
                    serde_json::json!({
                        "_pickle_error": format!("Could not fully parse pickle data: {e}"),
                        "_raw_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
                        "_size": bytes.len()
                    })
                }
            }
        } else {
            serde_json::Value::Null
        };

        // Combine metadata and log data
        let mut combined = serde_json::Map::new();
        combined.insert("_metadata".into(), metadata);
        combined.insert("_save_data".into(), log_data);

        Ok((serde_json::Value::Object(combined.clone()), serde_json::Value::Object(combined)))
    }

    /// Extract variables from the parsed save data
    fn extract_variables(raw: &serde_json::Value) -> Option<Vec<Variable>> {
        let save_data = raw.get("_save_data")?;

        // Ren'Py stores variables in a dict-like structure
        // The exact structure depends on the game, but common patterns:
        // - Direct key-value pairs at top level
        // - Nested under "store" namespace
        let mut variables = Vec::new();
        let mut id = 0u32;

        fn extract_from_object(obj: &serde_json::Map<String, serde_json::Value>, vars: &mut Vec<Variable>, id: &mut u32, prefix: &str) {
            for (key, value) in obj {
                // Skip internal Ren'Py keys
                if key.starts_with("_") && !key.starts_with("_game") {
                    continue;
                }
                // Skip complex objects for the variable view (they'll be in raw editor)
                match value {
                    serde_json::Value::Number(_)
                    | serde_json::Value::Bool(_)
                    | serde_json::Value::String(_) => {
                        let name = if prefix.is_empty() {
                            key.clone()
                        } else {
                            format!("{prefix}.{key}")
                        };
                        vars.push(Variable {
                            id: *id,
                            name: Some(name),
                            value: value.clone(),
                            group: if prefix.is_empty() { None } else { Some(prefix.to_string()) },
                        });
                        *id += 1;
                    }
                    serde_json::Value::Object(inner) => {
                        let new_prefix = if prefix.is_empty() {
                            key.clone()
                        } else {
                            format!("{prefix}.{key}")
                        };
                        extract_from_object(inner, vars, id, &new_prefix);
                    }
                    _ => {}
                }
            }
        }

        if let Some(obj) = save_data.as_object() {
            extract_from_object(obj, &mut variables, &mut id, "");
        }

        if variables.is_empty() {
            None
        } else {
            Some(variables)
        }
    }
}

impl EnginePlugin for RenpyPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "renpy".into(),
            name: "Ren'Py".into(),
            icon: "renpy".into(),
            supports_debug: true,
            save_extensions: vec!["save".into()],
            description: "Ren'Py visual novel engine saves".into(),
            save_dir_hint: Some(if cfg!(target_os = "windows") {
                "Select the folder containing your .save files.\n\
                 Typically found in:\n\
                 <game_folder>/game/saves/ or %APPDATA%\\RenPy\\<game_name>"
                    .to_string()
            } else {
                "Select the folder containing your .save files.\n\
                 Typically found in:\n\
                 <game_folder>/game/saves/ or ~/.renpy/<game_name>"
                    .to_string()
            }),
            pick_mode: "folder".into(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        // Check for renpy/ directory or .rpa files
        game_dir.join("renpy").is_dir()
            || game_dir.join("lib").join("python2.7").is_dir()
            || game_dir.join("lib").join("py3-linux-x86_64").is_dir()
            || Self::find_game_subdir(game_dir)
                .read_dir()
                .ok()
                .map(|entries| {
                    entries
                        .flatten()
                        .any(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("rpa"))
                })
                .unwrap_or(false)
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let save_dirs = Self::find_save_dirs(game_dir);
        let mut saves = Vec::new();

        for save_dir in save_dirs {
            let entries = fs::read_dir(&save_dir)
                .map_err(|e| format!("Failed to read save directory: {e}"))?;

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                if name.ends_with(".save") {
                    let meta = entry.metadata().map_err(|e| format!("Metadata error: {e}"))?;
                    let modified = meta
                        .modified()
                        .ok()
                        .map(|t| chrono::DateTime::<Local>::from(t).to_rfc3339())
                        .unwrap_or_default();

                    saves.push(SaveFile {
                        path: path.to_string_lossy().to_string(),
                        name: name.replace(".save", ""),
                        modified,
                        size: meta.len(),
                    });
                }
            }
        }

        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, _game_dir: &Path) -> Result<SaveData, String> {
        let (raw, _) = Self::parse_save_zip(save_path)?;

        let variables = Self::extract_variables(&raw);

        Ok(SaveData {
            raw,
            party: None,
            inventory: None,
            currency: None,
            variables,
            switches: None,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        // For Ren'Py, writing back is complex because we need to re-pickle
        // For now, we support raw JSON editing and write back the modified structure
        // The raw editor is the primary editing interface for Ren'Py

        // Read original ZIP to preserve screenshot and other entries
        let original = fs::read(save_path)
            .map_err(|e| format!("Failed to read original save: {e}"))?;
        let cursor = std::io::Cursor::new(&original);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to open ZIP: {e}"))?;

        // Create new ZIP
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = zip::ZipWriter::new(cursor);

            // Copy all entries except json (which we'll update with metadata)
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)
                    .map_err(|e| format!("Failed to read ZIP entry: {e}"))?;
                let name = entry.name().to_string();

                if name == "json" {
                    // Write updated metadata
                    let metadata = data.raw.get("_metadata").unwrap_or(&serde_json::Value::Null);
                    let json_bytes = serde_json::to_string(metadata)
                        .map_err(|e| format!("Failed to serialize metadata: {e}"))?;
                    writer.start_file::<_, ()>("json", zip::write::SimpleFileOptions::default())
                        .map_err(|e| format!("Failed to start zip entry: {e}"))?;
                    std::io::Write::write_all(&mut writer, json_bytes.as_bytes())
                        .map_err(|e| format!("Failed to write json entry: {e}"))?;
                } else {
                    // Copy entry as-is
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)
                        .map_err(|e| format!("Failed to read entry: {e}"))?;
                    writer.start_file::<_, ()>(&name, zip::write::SimpleFileOptions::default())
                        .map_err(|e| format!("Failed to start zip entry: {e}"))?;
                    std::io::Write::write_all(&mut writer, &content)
                        .map_err(|e| format!("Failed to write entry: {e}"))?;
                }
            }

            writer.finish()
                .map_err(|e| format!("Failed to finish ZIP: {e}"))?;
        }

        fs::write(save_path, buf)
            .map_err(|e| format!("Failed to write save file: {e}"))?;
        Ok(())
    }

    fn resolve_names(&self, game_dir: &Path) -> Result<NameMap, String> {
        let mut names = NameMap::default();
        let game_sub = Self::find_game_subdir(game_dir);

        // Scan .rpy files for variable definitions
        if let Ok(entries) = fs::read_dir(&game_sub) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rpy") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        Self::scan_rpy_for_names(&content, &mut names);
                    }
                }
            }
        }

        Ok(names)
    }

    fn supports_debug_patch(&self) -> bool {
        true
    }

    fn apply_debug_patch(&self, game_dir: &Path) -> Result<PatchInfo, String> {
        let game_sub = Self::find_game_subdir(game_dir);
        let patch_path = game_sub.join("01-debug-patch.rpy");

        let content = r#"# Prismatic - Debug Mode Patch
# Enables developer console (Shift+O), reload (Shift+R), and debug menu
init +1:
    python hide:
        config.developer = True
        config.console = True
"#;

        fs::write(&patch_path, content)
            .map_err(|e| format!("Failed to write debug patch: {e}"))?;

        Ok(PatchInfo {
            engine: "renpy".into(),
            game_dir: game_dir.to_string_lossy().to_string(),
            patches: vec![PatchEntry {
                file_path: patch_path.to_string_lossy().to_string(),
                action: PatchAction::Created,
                original_hash: None,
            }],
            applied_at: Local::now().to_rfc3339(),
        })
    }

    fn revert_debug_patch(&self, _game_dir: &Path, patch: &PatchInfo) -> Result<(), String> {
        for entry in &patch.patches {
            if let PatchAction::Created = &entry.action {
                let path = Path::new(&entry.file_path);
                if path.exists() {
                    fs::remove_file(path)
                        .map_err(|e| format!("Failed to remove {}: {e}", entry.file_path))?;
                }
            }
        }
        Ok(())
    }
}

impl RenpyPlugin {
    fn scan_rpy_for_names(content: &str, names: &mut NameMap) {
        let mut var_id = 0u32;
        for line in content.lines() {
            let trimmed = line.trim();

            // define statements: define variable_name = value
            if trimmed.starts_with("define ") || trimmed.starts_with("default ") {
                let rest = if let Some(r) = trimmed.strip_prefix("define ") {
                    r
                } else if let Some(r) = trimmed.strip_prefix("default ") {
                    r
                } else {
                    continue;
                };

                if let Some(eq_pos) = rest.find('=') {
                    let var_name = rest[..eq_pos].trim().to_string();
                    if !var_name.is_empty() && !var_name.contains('(') {
                        names.variables.insert(var_id, var_name);
                        var_id += 1;
                    }
                }
            }
        }
    }
}
