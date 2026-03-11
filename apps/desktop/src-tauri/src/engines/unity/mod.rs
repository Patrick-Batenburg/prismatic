use crate::engines::types::*;
use crate::engines::EnginePlugin;
use std::path::Path;

const SUPPORTED_EXTENSIONS: &[&str] = &["json", "xml", "es3"];

pub struct UnityPlugin;

impl EnginePlugin for UnityPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "unity".to_string(),
            name: "Unity".to_string(),
            icon: "unity".to_string(),
            supports_debug: false,
            save_extensions: SUPPORTED_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            description: "Unity game saves (JSON, XML, ES3)".to_string(),
            save_dir_hint: Some(
                "Select the folder containing your Unity save files. Usually in AppData/LocalLow/CompanyName/GameName."
                    .to_string(),
            ),
            pick_mode: "folder".to_string(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        let entries = match std::fs::read_dir(game_dir) {
            Ok(e) => e,
            Err(_) => return false,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str()) {
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
            let is_match = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| {
                    let ext_lower = ext.to_lowercase();
                    SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str())
                })
                .unwrap_or(false);

            if is_match {
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
        let ext = save_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let content = std::fs::read_to_string(save_path)
            .map_err(|e| format!("failed to read {}: {e}", save_path.display()))?;

        let raw: serde_json::Value = match ext.as_str() {
            "json" => serde_json::from_str(&content)
                .map_err(|e| format!("failed to parse JSON: {e}"))?,
            "xml" => xml_to_json(&content)?,
            "es3" => serde_json::from_str(&content).map_err(|_| {
                "ES3 file appears to be encrypted. Encrypted ES3 files are not supported."
                    .to_string()
            })?,
            _ => return Err(format!("unsupported extension: {ext}")),
        };

        let variables = extract_variables(&raw);

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
        let ext = save_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let output = match ext.as_str() {
            "json" | "es3" => serde_json::to_string_pretty(&data.raw)
                .map_err(|e| format!("failed to serialize JSON: {e}"))?,
            "xml" => return Err("XML write not yet supported".to_string()),
            _ => return Err(format!("unsupported extension: {ext}")),
        };

        std::fs::write(save_path, output)
            .map_err(|e| format!("failed to write {}: {e}", save_path.display()))?;

        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }
}

fn extract_variables(raw: &serde_json::Value) -> Vec<Variable> {
    match raw {
        serde_json::Value::Object(map) => map
            .iter()
            .enumerate()
            .map(|(i, (key, val))| Variable {
                id: i as u32,
                name: Some(key.clone()),
                value: val.clone(),
                group: None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse XML content into a JSON value.
///
/// - Elements become object keys
/// - Attributes are stored as `@attr`
/// - Text content is stored as `#text`
/// - Duplicate sibling elements become arrays
fn xml_to_json(xml_content: &str) -> Result<serde_json::Value, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use serde_json::{Map, Value};

    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);

    // Parse recursively using a stack-based approach
    let mut stack: Vec<(String, Map<String, Value>)> = Vec::new();
    // Root container
    stack.push(("__root__".to_string(), Map::new()));

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut element_map = Map::new();

                // Process attributes
                for attr in e.attributes().flatten() {
                    let key = format!("@{}", String::from_utf8_lossy(attr.key.as_ref()));
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    element_map.insert(key, Value::String(val));
                }

                stack.push((tag_name, element_map));
            }
            Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut element_map = Map::new();

                for attr in e.attributes().flatten() {
                    let key = format!("@{}", String::from_utf8_lossy(attr.key.as_ref()));
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    element_map.insert(key, Value::String(val));
                }

                let child_value = if element_map.is_empty() {
                    Value::Null
                } else {
                    Value::Object(element_map)
                };

                // Add to parent
                if let Some(parent) = stack.last_mut() {
                    insert_into_map(&mut parent.1, &tag_name, child_value);
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().map_err(|err| format!("XML text error: {err}"))?.to_string();
                if !text.is_empty() {
                    if let Some(current) = stack.last_mut() {
                        current.1.insert("#text".to_string(), Value::String(text));
                    }
                }
            }
            Ok(Event::End(_)) => {
                let (tag_name, element_map) = stack.pop().ok_or("unexpected closing tag")?;

                let child_value = if element_map.len() == 1 {
                    if let Some(text) = element_map.get("#text") {
                        // Element with only text content — unwrap to just the string
                        text.clone()
                    } else {
                        Value::Object(element_map)
                    }
                } else if element_map.is_empty() {
                    Value::Null
                } else {
                    Value::Object(element_map)
                };

                if let Some(parent) = stack.last_mut() {
                    insert_into_map(&mut parent.1, &tag_name, child_value);
                } else {
                    // This was the root element
                    let mut root = Map::new();
                    root.insert(tag_name, child_value);
                    return Ok(Value::Object(root));
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // skip comments, CDATA, PI, etc.
            Err(e) => return Err(format!("XML parse error: {e}")),
        }
    }

    // Return root container contents
    let (_, root_map) = stack.pop().ok_or("empty XML document")?;
    if root_map.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Object(root_map))
    }
}

/// Insert a value into a map, converting to array if the key already exists.
fn insert_into_map(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: serde_json::Value) {
    use serde_json::Value;
    if let Some(existing) = map.get_mut(key) {
        match existing {
            Value::Array(arr) => arr.push(value),
            _ => {
                let prev = existing.take();
                *existing = Value::Array(vec![prev, value]);
            }
        }
    } else {
        map.insert(key.to_string(), value);
    }
}
