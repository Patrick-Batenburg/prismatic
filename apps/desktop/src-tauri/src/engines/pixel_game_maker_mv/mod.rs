pub mod crypto;
pub mod names;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PgmmvPlugin;

impl PgmmvPlugin {
    fn info_json_path(game_dir: &Path) -> PathBuf {
        game_dir.join("Resources").join("data").join("info.json")
    }

    fn project_json_path(game_dir: &Path) -> PathBuf {
        game_dir.join("Resources").join("data").join("project.json")
    }

    fn save_dir(game_dir: &Path) -> PathBuf {
        game_dir.join("Resources").join("save")
    }

    fn load_key(game_dir: &Path) -> Result<Vec<u8>, String> {
        crypto::load_key(&Self::info_json_path(game_dir))
    }

    fn load_project(game_dir: &Path, key: &[u8]) -> Result<serde_json::Value, String> {
        let path = Self::project_json_path(game_dir);
        if !path.exists() {
            return Err("project.json not found".into());
        }
        let data = fs::read(&path)
            .map_err(|e| format!("Failed to read project.json: {e}"))?;
        let decrypted = crypto::decrypt_resource(&data, key)?;
        let json_str = String::from_utf8(decrypted)
            .map_err(|e| format!("project.json is not valid UTF-8: {e}"))?;
        serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse project.json: {e}"))
    }

    /// Extract structured data from PGMMV save JSON
    fn extract_structured(
        raw: &serde_json::Value,
        name_map: &NameMap,
    ) -> (
        Option<Vec<Variable>>,
        Option<Vec<Switch>>,
        Vec<CustomSection>,
    ) {
        let mut variables = Vec::new();
        let mut switches = Vec::new();
        let mut custom = Vec::new();

        // Extract play data variables
        if let Some(play_data) = raw.get("playData") {
            // Variables from playData
            if let Some(var_list) = play_data.get("variableList").and_then(|v| v.as_array()) {
                for (i, val) in var_list.iter().enumerate() {
                    let id = i as u32;
                    variables.push(Variable {
                        id,
                        name: name_map.variables.get(&id).cloned(),
                        value: val.clone(),
                        group: Some("Global".into()),
                    });
                }
            }

            // Switches from playData
            if let Some(sw_list) = play_data.get("switchList").and_then(|v| v.as_array()) {
                for (i, val) in sw_list.iter().enumerate() {
                    let id = i as u32;
                    switches.push(Switch {
                        id,
                        name: name_map.switches.get(&id).cloned(),
                        value: val.as_bool().unwrap_or(false),
                    });
                }
            }

            // Object data as custom sections
            if let Some(obj_list) = play_data.get("objectDataList").and_then(|v| v.as_array()) {
                custom.push(CustomSection {
                    key: "objectDataList".into(),
                    label: "Object Data".into(),
                    data: serde_json::Value::Array(obj_list.clone()),
                });
            }
        }

        (
            if variables.is_empty() { None } else { Some(variables) },
            if switches.is_empty() { None } else { Some(switches) },
            custom,
        )
    }

    /// Apply edits back to raw JSON
    fn apply_edits(raw: &mut serde_json::Value, data: &SaveData) {
        if let Some(play_data) = raw.get_mut("playData") {
            // Apply variables
            if let Some(ref variables) = data.variables {
                if let Some(var_list) = play_data.get_mut("variableList").and_then(|v| v.as_array_mut()) {
                    for var in variables {
                        if (var.id as usize) < var_list.len() {
                            var_list[var.id as usize] = var.value.clone();
                        }
                    }
                }
            }

            // Apply switches
            if let Some(ref switches) = data.switches {
                if let Some(sw_list) = play_data.get_mut("switchList").and_then(|v| v.as_array_mut()) {
                    for sw in switches {
                        if (sw.id as usize) < sw_list.len() {
                            sw_list[sw.id as usize] = serde_json::json!(sw.value);
                        }
                    }
                }
            }
        }
    }
}

impl EnginePlugin for PgmmvPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "pixel-game-maker-mv".into(),
            name: "Pixel Game Maker MV".into(),
            icon: "pixel-game-maker-mv".into(),
            supports_debug: false,
            save_extensions: vec!["json".into()],
            description: "Pixel Game Maker MV encrypted saves".into(),
            save_dir_hint: None,
            pick_mode: "folder".into(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        Self::info_json_path(game_dir).exists()
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let save_dir = Self::save_dir(game_dir);
        if !save_dir.exists() {
            return Ok(vec![]);
        }

        let mut saves = Vec::new();
        let entries = fs::read_dir(&save_dir)
            .map_err(|e| format!("Failed to read save directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                || path.extension().is_none()
            {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let meta = entry.metadata().map_err(|e| format!("Metadata error: {e}"))?;
                let modified = meta
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<Local>::from(t).to_rfc3339())
                    .unwrap_or_default();

                saves.push(SaveFile {
                    path: path.to_string_lossy().to_string(),
                    name,
                    modified,
                    size: meta.len(),
                });
            }
        }

        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, game_dir: &Path) -> Result<SaveData, String> {
        let key = Self::load_key(game_dir)?;
        let data = fs::read(save_path)
            .map_err(|e| format!("Failed to read save file: {e}"))?;
        let decrypted = crypto::decrypt_resource(&data, &key)?;
        let json_str = String::from_utf8(decrypted)
            .map_err(|e| format!("Save file is not valid UTF-8: {e}"))?;
        let raw: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse save JSON: {e}"))?;

        // Skip inline name resolution if project.json is very large (>10MB)
        // to avoid freezing the UI. Names will be resolved separately.
        let name_map = if Self::project_json_path(game_dir).metadata().map(|m| m.len()).unwrap_or(0) > 10_000_000 {
            NameMap::default()
        } else {
            self.resolve_names(game_dir).unwrap_or_default()
        };
        let (variables, switches, custom_sections) = Self::extract_structured(&raw, &name_map);

        Ok(SaveData {
            raw,
            party: None, // PGMMV doesn't have a standard party system
            inventory: None,
            currency: None,
            variables,
            switches,
            custom_sections,
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        let game_dir = save_path
            .ancestors()
            .find(|p| Self::info_json_path(p).exists())
            .ok_or("Could not find game directory from save path")?;
        let key = Self::load_key(game_dir)?;

        let mut raw = data.raw.clone();
        Self::apply_edits(&mut raw, data);

        let json_bytes = serde_json::to_string(&raw)
            .map_err(|e| format!("Failed to serialize: {e}"))?
            .into_bytes();

        let encrypted = crypto::encrypt_resource(&json_bytes, &key)?;
        fs::write(save_path, encrypted)
            .map_err(|e| format!("Failed to write save file: {e}"))?;
        Ok(())
    }

    fn resolve_names(&self, game_dir: &Path) -> Result<NameMap, String> {
        let key = Self::load_key(game_dir)?;
        let project = Self::load_project(game_dir, &key)?;
        Ok(names::resolve_names_from_project(&project))
    }
}
