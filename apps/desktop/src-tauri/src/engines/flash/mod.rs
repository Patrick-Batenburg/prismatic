pub mod amf3;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use std::path::Path;

pub struct FlashSolPlugin;

impl EnginePlugin for FlashSolPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "flash".to_string(),
            name: "Flash".to_string(),
            icon: "flash".to_string(),
            supports_debug: false,
            save_extensions: vec!["sol".to_string()],
            description: "Flash .sol saves".to_string(),
            save_dir_hint: Some(
                "Select the folder containing your .sol save files.\n\
                 Typically found in:\n\
                 C:\\Users\\<you>\\AppData\\Roaming\\Macromedia\\Flash Player\\#SharedObjects"
                    .to_string(),
            ),
            pick_mode: "folder".to_string(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        // Detect Flash game by .swf files in the folder
        Self::has_swf_files(game_dir) || Self::has_sol_files(game_dir)
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let entries = std::fs::read_dir(game_dir)
            .map_err(|e| format!("failed to read dir {}: {e}", game_dir.display()))?;

        let mut saves = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "sol")
                .unwrap_or(false)
            {
                let meta = std::fs::metadata(&path).ok();
                saves.push(SaveFile {
                    path: path.to_string_lossy().to_string(),
                    name: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    modified: meta
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                    size: meta.map(|m| m.len()).unwrap_or(0),
                });
            }
        }
        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, _game_dir: &Path) -> Result<SaveData, String> {
        let data = std::fs::read(save_path)
            .map_err(|e| format!("failed to read {}: {e}", save_path.display()))?;

        let sol = amf3::parse_sol(&data)?;

        let mut raw_map = serde_json::Map::new();
        for (key, value) in &sol.pairs {
            raw_map.insert(key.clone(), value.to_json());
        }
        let raw = serde_json::Value::Object(raw_map);

        let variables: Vec<Variable> = sol
            .pairs
            .iter()
            .enumerate()
            .map(|(i, (key, value))| Variable {
                id: i as u32,
                name: Some(key.clone()),
                value: value.to_json(),
                group: None,
            })
            .collect();

        Ok(SaveData {
            raw,
            party: None,
            inventory: None,
            currency: None,
            variables: Some(variables),
            switches: None,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        let original_data = std::fs::read(save_path)
            .map_err(|e| format!("failed to read original {}: {e}", save_path.display()))?;
        let original_sol = amf3::parse_sol(&original_data)?;

        let pairs = if let serde_json::Value::Object(map) = &data.raw {
            map.iter()
                .map(|(k, v)| (k.clone(), amf3::AmfValue::from_json(v)))
                .collect()
        } else {
            return Err("raw data must be a JSON object".into());
        };

        let sol = amf3::SolFile {
            name: original_sol.name,
            amf_version: original_sol.amf_version,
            pairs,
        };

        let bytes = amf3::write_sol(&sol);
        std::fs::write(save_path, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", save_path.display()))?;

        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }
}

impl FlashSolPlugin {
    fn has_swf_files(dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "swf")
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_sol_files(dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "sol")
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }
}
