use crate::engines::types::NameMap;

/// Resolve names from a decrypted project.json
pub fn resolve_names_from_project(project: &serde_json::Value) -> NameMap {
    let mut names = NameMap::default();

    // Global variables
    if let Some(var_list) = project.get("variableList").and_then(|v| v.as_array()) {
        extract_named_entries(var_list, &mut names.variables);
    }

    // Global switches
    if let Some(sw_list) = project.get("switchList").and_then(|v| v.as_array()) {
        extract_named_entries(sw_list, &mut names.switches);
    }

    // Objects — extract per-object variables and switches
    if let Some(obj_list) = project.get("objectList").and_then(|v| v.as_array()) {
        for obj in obj_list {
            let obj_id = obj.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let obj_name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();

            if !obj_name.is_empty() {
                names.actors.insert(obj_id, obj_name.clone());
            }

            // Per-object variables
            if let Some(var_list) = obj.get("variableList").and_then(|v| v.as_array()) {
                let mut obj_vars = std::collections::HashMap::new();
                extract_named_entries(var_list, &mut obj_vars);
                if !obj_vars.is_empty() {
                    names.custom.insert(format!("obj_{obj_id}_vars"), obj_vars);
                }
            }

            // Per-object switches
            if let Some(sw_list) = obj.get("switchList").and_then(|v| v.as_array()) {
                let mut obj_switches = std::collections::HashMap::new();
                extract_named_entries(sw_list, &mut obj_switches);
                if !obj_switches.is_empty() {
                    names.custom.insert(format!("obj_{obj_id}_switches"), obj_switches);
                }
            }
        }
    }

    names
}

fn extract_named_entries(list: &[serde_json::Value], map: &mut std::collections::HashMap<u32, String>) {
    for entry in list {
        if let (Some(id), Some(name)) = (
            entry.get("id").and_then(|v| v.as_u64()),
            entry.get("name").and_then(|v| v.as_str()),
        ) {
            if !name.is_empty() {
                map.insert(id as u32, name.to_string());
            }
        }

        // Recurse into children
        if let Some(children) = entry.get("children").and_then(|v| v.as_array()) {
            extract_named_entries(children, map);
        }
    }
}
