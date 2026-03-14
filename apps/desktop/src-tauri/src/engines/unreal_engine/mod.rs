use crate::engines::types::*;
use crate::engines::EnginePlugin;
use std::io::Cursor;
use std::path::Path;

const GVAS_MAGIC: &[u8; 4] = b"GVAS";

pub struct UnrealPlugin;

impl EnginePlugin for UnrealPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "unreal-engine".to_string(),
            name: "Unreal Engine".to_string(),
            icon: "unreal-engine".to_string(),
            supports_debug: false,
            save_extensions: vec!["sav".to_string()],
            description: "Unreal Engine game saves".to_string(),
            save_dir_hint: Some(if cfg!(target_os = "windows") {
                "Select the folder containing your .sav save files.\n\
                 Usually somewhere in AppData."
                    .to_string()
            } else {
                "Select the folder containing your .sav save files.\n\
                 Usually inside your Wine prefix under AppData/Local/<game>/Saved/SaveGames."
                    .to_string()
            }),
            pick_mode: "folder".to_string(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        if game_dir.join("Engine").is_dir() {
            return true;
        }
        Self::has_gvas_files(game_dir)
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let entries = std::fs::read_dir(game_dir)
            .map_err(|e| format!("failed to read dir {}: {e}", game_dir.display()))?;

        let mut saves = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "sav")
                .unwrap_or(false)
                && Self::is_gvas_file(&path)
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
                        .map(crate::engines::utils::format_modified_time)
                        .unwrap_or_default(),
                    size: meta.map(|m| m.len()).unwrap_or(0),
                });
            }
        }
        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, _game_dir: &Path) -> Result<SaveData, String> {
        let mut file = std::fs::File::open(save_path)
            .map_err(|e| format!("failed to open {}: {e}", save_path.display()))?;

        let save = uesave::Save::read(&mut file)
            .map_err(|e| format!("failed to parse GVAS: {e}"))?;

        // Convert full Save to JSON via serde
        let raw = serde_json::to_value(&save)
            .map_err(|e| format!("failed to serialize GVAS to JSON: {e}"))?;

        // Extract top-level properties as variables
        let properties_json = serde_json::to_value(&save.root.properties)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let variables: Vec<Variable> = if let serde_json::Value::Object(map) = &properties_json {
            map.iter()
                .enumerate()
                .map(|(i, (key, val))| Variable {
                    id: i as u32,
                    name: Some(key.clone()),
                    value: val.clone(),
                    group: None,
                })
                .collect()
        } else {
            Vec::new()
        };

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
        let save: uesave::Save = serde_json::from_value(data.raw.clone())
            .map_err(|e| format!("failed to deserialize edited data: {e}"))?;

        let mut buf = Cursor::new(Vec::new());
        save.write(&mut buf)
            .map_err(|e| format!("failed to write GVAS: {e}"))?;

        std::fs::write(save_path, buf.into_inner())
            .map_err(|e| format!("failed to write {}: {e}", save_path.display()))?;

        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }
}

impl UnrealPlugin {
    fn is_gvas_file(path: &Path) -> bool {
        std::fs::read(path)
            .ok()
            .map(|data| data.len() >= 4 && &data[..4] == GVAS_MAGIC)
            .unwrap_or(false)
    }

    fn has_gvas_files(dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| e == "sav")
                    .unwrap_or(false)
                    && Self::is_gvas_file(&path)
                {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::EnginePlugin;

    #[test]
    #[ignore] // requires real save file
    fn parse_real_obf_save() {
        let plugin = UnrealPlugin;
        let save_dir = Path::new(r"C:\Users\Patrick\AppData\Local\OBF\Saved\SaveGames");
        let save_path = save_dir.join("1.sav");

        let data = plugin.parse_save(&save_path, save_dir).unwrap();
        assert!(data.variables.is_some());
        let vars = data.variables.unwrap();
        assert!(!vars.is_empty(), "should have at least one variable");
        println!("Parsed {} variables from 1.sav", vars.len());
        for v in &vars {
            println!("  {} = {}", v.name.as_deref().unwrap_or("?"), v.value);
        }
    }

    #[test]
    #[ignore] // requires real save file
    fn parse_real_obf_preset() {
        let plugin = UnrealPlugin;
        let save_dir = Path::new(r"C:\Users\Patrick\AppData\Local\OBF\Saved\SaveGames");
        let save_path = save_dir.join("CP_Bovaur_Bull_Male_Minotaur.sav");

        let data = plugin.parse_save(&save_path, save_dir).unwrap();
        assert!(data.variables.is_some());
        let vars = data.variables.unwrap();
        println!("Parsed {} variables from preset", vars.len());
        for v in &vars {
            println!("  {} = {}", v.name.as_deref().unwrap_or("?"), v.value);
        }
    }

    #[test]
    #[ignore] // requires real save file
    fn roundtrip_real_obf_save() {
        let save_dir = Path::new(r"C:\Users\Patrick\AppData\Local\OBF\Saved\SaveGames");
        let save_path = save_dir.join("1.sav");

        let original_bytes = std::fs::read(&save_path).unwrap();

        // Parse
        let save = uesave::Save::read(&mut Cursor::new(&original_bytes)).unwrap();

        // Roundtrip through JSON
        let json = serde_json::to_value(&save).unwrap();
        let save2: uesave::Save = serde_json::from_value(json).unwrap();

        // Write
        let mut buf = Cursor::new(Vec::new());
        save2.write(&mut buf).unwrap();
        let written_bytes = buf.into_inner();

        assert_eq!(original_bytes.len(), written_bytes.len(), "roundtrip size mismatch");
        assert_eq!(original_bytes, written_bytes, "roundtrip bytes mismatch");
    }
}
