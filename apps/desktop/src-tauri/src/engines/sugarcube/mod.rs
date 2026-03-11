use crate::engines::types::*;
use crate::engines::EnginePlugin;
use std::path::Path;

pub struct SugarCubePlugin;

impl EnginePlugin for SugarCubePlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "sugarcube".to_string(),
            name: "SugarCube".to_string(),
            icon: "sugarcube".to_string(),
            supports_debug: false,
            save_extensions: vec!["save".to_string()],
            description: "SugarCube / Twine 2 game saves".to_string(),
            save_dir_hint: Some(
                "Select the folder containing your exported .save files.".to_string(),
            ),
            pick_mode: "folder".to_string(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(game_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "save").unwrap_or(false)
                    && is_sugarcube_save(&path)
                {
                    return true;
                }
            }
        }
        false
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let entries = std::fs::read_dir(game_dir)
            .map_err(|e| format!("failed to read dir {}: {e}", game_dir.display()))?;

        let mut saves = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "save").unwrap_or(false) {
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
        let content = std::fs::read_to_string(save_path)
            .map_err(|e| format!("failed to read {}: {e}", save_path.display()))?;

        let raw = decompress_save(&content)?;
        let raw_value: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("failed to parse JSON: {e}"))?;

        let variables = extract_variables(&raw_value)?;

        Ok(SaveData {
            raw: raw_value,
            party: None,
            inventory: None,
            currency: None,
            variables: Some(variables),
            switches: None,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        let json_str = serde_json::to_string(&data.raw)
            .map_err(|e| format!("failed to serialize JSON: {e}"))?;

        let compressed = compress_save(&json_str)?;
        std::fs::write(save_path, compressed)
            .map_err(|e| format!("failed to write {}: {e}", save_path.display()))?;

        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }
}

/// Check if a .save file looks like a SugarCube save (base64-encoded LZString).
fn is_sugarcube_save(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| {
            let trimmed = content.trim();
            if trimmed.len() < 20 {
                return None;
            }
            decompress_save(trimmed).ok()
        })
        .and_then(|json_str| {
            let val: serde_json::Value = serde_json::from_str(&json_str).ok()?;
            val.get("state")?.get("delta")?;
            Some(true)
        })
        .unwrap_or(false)
}

/// Decompress a SugarCube save string (LZString base64) to JSON string.
fn decompress_save(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    let utf16: Vec<u16> =
        lz_str::decompress_from_base64(trimmed).ok_or("failed to decompress LZString")?;
    String::from_utf16(&utf16).map_err(|e| format!("failed to decode UTF-16: {e}"))
}

/// Compress a JSON string to SugarCube save format (LZString base64).
fn compress_save(json_str: &str) -> Result<String, String> {
    let utf16: Vec<u16> = json_str.encode_utf16().collect();
    Ok(lz_str::compress_to_base64(&utf16))
}

/// Extract variables from the parsed save JSON.
/// Handles both old format (variables in delta[0].variables)
/// and new compressed format (data + values dictionary).
fn extract_variables(raw: &serde_json::Value) -> Result<Vec<Variable>, String> {
    let delta = raw
        .get("state")
        .and_then(|s| s.get("delta"))
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .ok_or("missing state.delta[0]")?;

    // Try old format first: direct variables dict
    if let Some(vars) = delta.get("variables").and_then(|v| v.as_object()) {
        if !vars.is_empty() {
            return Ok(vars
                .iter()
                .enumerate()
                .map(|(i, (key, val))| Variable {
                    id: i as u32,
                    name: Some(key.clone()),
                    value: val.clone(),
                    group: None,
                })
                .collect());
        }
    }

    // New compressed format: reconstruct from data + values
    if let (Some(values_arr), Some(data_arr)) = (
        delta.get("values").and_then(|v| v.as_array()),
        delta.get("data").and_then(|v| v.as_array()),
    ) {
        let resolved = resolve_compressed(data_arr, values_arr);
        if let serde_json::Value::Object(map) = resolved {
            return Ok(map
                .iter()
                .enumerate()
                .map(|(i, (key, val))| Variable {
                    id: i as u32,
                    name: Some(key.clone()),
                    value: val.clone(),
                    group: None,
                })
                .collect());
        }
    }

    Ok(Vec::new())
}

/// Resolve SugarCube's compressed delta format.
fn resolve_compressed(
    data: &[serde_json::Value],
    values: &[serde_json::Value],
) -> serde_json::Value {
    resolve_value(&serde_json::Value::Array(data.to_vec()), values)
}

fn resolve_value(
    item: &serde_json::Value,
    values: &[serde_json::Value],
) -> serde_json::Value {
    match item {
        serde_json::Value::Number(n) => {
            if let Some(idx) = n.as_u64() {
                let idx = idx as usize;
                if idx < values.len() {
                    return values[idx].clone();
                }
            }
            item.clone()
        }
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            match arr[0].as_u64() {
                Some(1) => {
                    // Object: [1, key_idx, val, key_idx, val, ...]
                    let mut map = serde_json::Map::new();
                    let mut i = 1;
                    while i + 1 < arr.len() {
                        let key = resolve_value(&arr[i], values);
                        let key_str = match &key {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        let val = resolve_value(&arr[i + 1], values);
                        map.insert(key_str, val);
                        i += 2;
                    }
                    serde_json::Value::Object(map)
                }
                Some(0) => {
                    // Array: [0, val, val, ...]
                    let resolved: Vec<serde_json::Value> =
                        arr[1..].iter().map(|v| resolve_value(v, values)).collect();
                    serde_json::Value::Array(resolved)
                }
                _ => {
                    let resolved: Vec<serde_json::Value> =
                        arr.iter().map(|v| resolve_value(v, values)).collect();
                    serde_json::Value::Array(resolved)
                }
            }
        }
        _ => item.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::EnginePlugin;

    #[test]
    #[ignore] // requires real save file
    fn parse_real_sugarcube_save() {
        let plugin = SugarCubePlugin;
        let save_dir = Path::new(r"D:\Personalisation\Avatar\saves\Degrees of Lewdity");
        let save_path = save_dir.join("degrees-of-lewdity-wolfgirl.save");

        let data = plugin.parse_save(&save_path, save_dir).unwrap();
        assert!(data.variables.is_some());
        let vars = data.variables.unwrap();
        assert!(!vars.is_empty(), "should have at least one variable");
        println!("Parsed {} variables", vars.len());
        for v in vars.iter().take(20) {
            println!("  {} = {}", v.name.as_deref().unwrap_or("?"), v.value);
        }
    }

    #[test]
    #[ignore] // requires real save file
    fn parse_old_format_save() {
        let plugin = SugarCubePlugin;
        let save_dir = Path::new(r"D:\Personalisation\Avatar\saves\Degrees of Lewdity");
        let save_path = save_dir.join("degrees-of-lewdity-20221130-154641.save");

        let data = plugin.parse_save(&save_path, save_dir).unwrap();
        assert!(data.variables.is_some());
        let vars = data.variables.unwrap();
        assert!(!vars.is_empty(), "should have at least one variable");
        println!("Parsed {} variables from old format", vars.len());
        for v in vars.iter().take(20) {
            println!("  {} = {}", v.name.as_deref().unwrap_or("?"), v.value);
        }
    }

    #[test]
    #[ignore] // requires real save file
    fn roundtrip_sugarcube_save() {
        let plugin = SugarCubePlugin;
        let save_dir = Path::new(r"D:\Personalisation\Avatar\saves\Degrees of Lewdity");
        let save_path = save_dir.join("degrees-of-lewdity-wolfgirl.save");

        let original = std::fs::read_to_string(&save_path).unwrap();

        let data = plugin.parse_save(&save_path, save_dir).unwrap();

        let json_str = serde_json::to_string(&data.raw).unwrap();
        let recompressed = compress_save(&json_str).unwrap();

        let original_json = decompress_save(original.trim()).unwrap();
        let roundtrip_json = decompress_save(&recompressed).unwrap();

        let orig_val: serde_json::Value = serde_json::from_str(&original_json).unwrap();
        let rt_val: serde_json::Value = serde_json::from_str(&roundtrip_json).unwrap();

        assert_eq!(orig_val, rt_val, "JSON roundtrip mismatch");
    }
}
