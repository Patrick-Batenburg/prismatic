use crate::engines::types::NameMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Find the data directory (www/data/ for MV, data/ for MZ)
pub fn find_data_dir(game_dir: &Path) -> Result<PathBuf, String> {
    let candidates = [
        game_dir.join("www").join("data"),
        game_dir.join("data"),
    ];
    for candidate in &candidates {
        if candidate.is_dir() {
            return Ok(candidate.clone());
        }
    }
    Err("Could not find data directory (www/data/ or data/)".into())
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))
}

pub fn resolve_names(game_dir: &Path) -> Result<NameMap, String> {
    let data_dir = find_data_dir(game_dir)?;
    let mut names = NameMap::default();

    // System.json — variables and switches are string arrays
    if let Ok(system) = read_json(&data_dir.join("System.json")) {
        if let Some(vars) = system["variables"].as_array() {
            for (i, v) in vars.iter().enumerate() {
                if let Some(name) = v.as_str() {
                    if !name.is_empty() {
                        names.variables.insert(i as u32, name.to_string());
                    }
                }
            }
        }
        if let Some(switches) = system["switches"].as_array() {
            for (i, s) in switches.iter().enumerate() {
                if let Some(name) = s.as_str() {
                    if !name.is_empty() {
                        names.switches.insert(i as u32, name.to_string());
                    }
                }
            }
        }
    }

    // Named entity files — all share array-of-objects structure with null at index 0
    let entity_files: Vec<(&str, &mut std::collections::HashMap<u32, String>)> = vec![
        ("Actors.json", &mut names.actors),
        ("Classes.json", &mut names.classes),
        ("Items.json", &mut names.items),
        ("Weapons.json", &mut names.weapons),
        ("Armors.json", &mut names.armors),
        ("Skills.json", &mut names.skills),
    ];

    for (file, map) in entity_files {
        if let Ok(data) = read_json(&data_dir.join(file)) {
            if let Some(arr) = data.as_array() {
                for item in arr.iter().skip(1) {
                    if item.is_null() {
                        continue;
                    }
                    if let (Some(id), Some(name)) =
                        (item["id"].as_u64(), item["name"].as_str())
                    {
                        if !name.is_empty() {
                            map.insert(id as u32, name.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(names)
}
