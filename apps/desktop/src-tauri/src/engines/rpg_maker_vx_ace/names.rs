use crate::engines::types::NameMap;
use marshal_rs::{self, Get};
use std::collections::HashMap;
use std::path::Path;

/// Load name mappings from Data/*.rvdata2 files for display purposes.
pub fn resolve_names(game_dir: &Path) -> Result<NameMap, String> {
    let data_dir = game_dir.join("Data");
    if !data_dir.exists() {
        return Err("Data directory not found".into());
    }

    let mut map = NameMap::default();

    extract_data_names(&data_dir.join("Actors.rvdata2"), &mut map.actors);
    extract_data_names(&data_dir.join("Classes.rvdata2"), &mut map.classes);
    extract_data_names(&data_dir.join("Items.rvdata2"), &mut map.items);
    extract_data_names(&data_dir.join("Weapons.rvdata2"), &mut map.weapons);
    extract_data_names(&data_dir.join("Armors.rvdata2"), &mut map.armors);
    extract_data_names(&data_dir.join("Skills.rvdata2"), &mut map.skills);

    extract_system_names(&data_dir.join("System.rvdata2"), &mut map);

    Ok(map)
}

/// Parse a Data file (e.g., Actors.rvdata2) which is a single Marshal dump
/// containing an Array of RPG::* objects. Index 0 is nil.
/// Each object has `@id` (Int) and `@name` (String).
fn extract_data_names(path: &Path, target: &mut HashMap<u32, String>) {
    let buf = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return,
    };
    let value = match marshal_rs::load(&buf, None) {
        Ok(v) => v,
        Err(_) => return,
    };
    let arr = match value.as_array() {
        Some(a) => a,
        None => return,
    };

    for item in arr.iter() {
        if item.is_null() {
            continue;
        }
        let id = match Get::<&str>::get(item, "@id").and_then(|v| v.as_int()) {
            Some(id) => id as u32,
            None => continue,
        };
        let name = match Get::<&str>::get(item, "@name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        target.insert(id, name);
    }
}

/// Parse System.rvdata2 for variable and switch names.
/// The System object has `@variables` and `@switches` arrays of strings.
/// Index 0 is nil/empty, index 1+ are names (may be empty strings).
fn extract_system_names(path: &Path, map: &mut NameMap) {
    let buf = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return,
    };
    let value = match marshal_rs::load(&buf, None) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Extract @variables array
    if let Some(vars) = Get::<&str>::get(&value, "@variables") {
        if let Some(arr) = vars.as_array() {
            for (i, v) in arr.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if let Some(name) = v.as_str() {
                    if !name.is_empty() {
                        map.variables.insert(i as u32, name.to_string());
                    }
                }
            }
        }
    }

    // Extract @switches array
    if let Some(switches) = Get::<&str>::get(&value, "@switches") {
        if let Some(arr) = switches.as_array() {
            for (i, v) in arr.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if let Some(name) = v.as_str() {
                    if !name.is_empty() {
                        map.switches.insert(i as u32, name.to_string());
                    }
                }
            }
        }
    }
}
