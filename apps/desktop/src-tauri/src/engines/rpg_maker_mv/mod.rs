pub mod names;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub struct RpgMakerMvPlugin;

/// Unwrap RPG Maker MV's serialized array format.
/// Arrays may be plain JSON arrays OR wrapped as `{"@c": N, "@a": [...]}`.
fn unwrap_array(val: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    val.as_array().or_else(|| val.get("@a").and_then(|a| a.as_array()))
}

/// Mutable version of unwrap_array.
fn unwrap_array_mut(val: &mut serde_json::Value) -> Option<&mut Vec<serde_json::Value>> {
    if val.is_array() {
        val.as_array_mut()
    } else {
        val.get_mut("@a").and_then(|a| a.as_array_mut())
    }
}

impl RpgMakerMvPlugin {
    /// Find the save directory (www/save/ for MV, save/ for MZ)
    fn find_save_dir(game_dir: &Path) -> Result<PathBuf, String> {
        let candidates = [
            game_dir.join("www").join("save"),
            game_dir.join("save"),
        ];
        for candidate in &candidates {
            if candidate.is_dir() {
                return Ok(candidate.clone());
            }
        }
        Err("Could not find save directory".into())
    }

    /// Decode .rpgsave: base64 → LZString decompress → JSON
    fn decode_save(content: &str) -> Result<serde_json::Value, String> {
        let decompressed = lz_str::decompress_from_base64(content)
            .ok_or("Failed to LZString decompress save data")?;
        let json_str = String::from_utf16(&decompressed)
            .map_err(|e| format!("Failed to convert decompressed data to string: {e}"))?;
        serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse decompressed save JSON: {e}"))
    }

    /// Encode save data: JSON → LZString compress → base64
    fn encode_save(data: &serde_json::Value) -> Result<String, String> {
        let json_str = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize save data: {e}"))?;
        let utf16: Vec<u16> = json_str.encode_utf16().collect();
        Ok(lz_str::compress_to_base64(&utf16))
    }

    /// Extract structured data from raw RPG Maker MV save JSON
    #[allow(clippy::type_complexity)]
    fn extract_structured(
        raw: &serde_json::Value,
        name_map: &NameMap,
    ) -> (
        Option<Vec<Character>>,
        Option<Inventory>,
        Option<CurrencyInfo>,
        Option<Vec<Variable>>,
        Option<Vec<Switch>>,
    ) {
        let party = Self::extract_party(raw, name_map);
        let inventory = Self::extract_inventory(raw, name_map);
        let currency = Self::extract_currency(raw);
        let variables = Self::extract_variables(raw, name_map);
        let switches = Self::extract_switches(raw, name_map);
        (party, inventory, currency, variables, switches)
    }

    fn extract_party(raw: &serde_json::Value, names: &NameMap) -> Option<Vec<Character>> {
        let actors = unwrap_array(raw.get("actors")?.get("_data")?)?;
        let mut characters = Vec::new();

        for actor_val in actors.iter() {
            if actor_val.is_null() {
                continue;
            }

            let id = match actor_val.get("_actorId").and_then(|v| v.as_u64()) {
                Some(id) => id as u32,
                None => continue,
            };
            let name = actor_val
                .get("_name")
                .and_then(|v| v.as_str())
                .or_else(|| names.actors.get(&id).map(|s| s.as_str()))
                .unwrap_or("Unknown")
                .to_string();

            let class_id = actor_val.get("_classId").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let class_name = names.classes.get(&class_id).cloned();

            let level = actor_val.get("_level").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

            let exp = actor_val
                .get("_exp")
                .and_then(|v| v.as_object())
                .and_then(|obj| {
                    // _exp is an object keyed by class ID, skip @c metadata
                    obj.iter()
                        .find(|(k, _)| !k.starts_with('@'))
                        .and_then(|(_, v)| v.as_u64())
                })
                .unwrap_or(0);

            let hp = actor_val.get("_hp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mp = actor_val.get("_mp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let tp = actor_val.get("_tp").and_then(|v| v.as_f64()).unwrap_or(0.0);

            // Parameter base values
            let param_base = actor_val.get("_paramPlus").and_then(|v| unwrap_array(v));
            let param_labels = ["Max HP", "Max MP", "ATK", "DEF", "MAT", "MDF", "AGI", "LUK"];
            let param_keys = ["mhp", "mmp", "atk", "def", "mat", "mdf", "agi", "luk"];

            let mut stats = vec![
                Stat { key: "hp".into(), label: "HP".into(), current: hp, max: None },
                Stat { key: "mp".into(), label: "MP".into(), current: mp, max: None },
                Stat { key: "tp".into(), label: "TP".into(), current: tp, max: None },
            ];

            if let Some(params) = param_base {
                for (i, param) in params.iter().enumerate() {
                    if i < param_keys.len() {
                        stats.push(Stat {
                            key: param_keys[i].to_string(),
                            label: param_labels[i].to_string(),
                            current: param.as_f64().unwrap_or(0.0),
                            max: None,
                        });
                    }
                }
            }

            // Equipment
            let mut equipment = Vec::new();
            let slot_names = ["Weapon", "Shield", "Head", "Body", "Accessory"];
            if let Some(equips) = actor_val.get("_equips").and_then(|v| unwrap_array(v)) {
                for (i, equip) in equips.iter().enumerate() {
                    let slot_name = slot_names.get(i).unwrap_or(&"Slot").to_string();
                    let item_id = equip.get("_itemId").and_then(|v| v.as_u64()).map(|v| v as u32);
                    let data_class = equip.get("_dataClass").and_then(|v| v.as_str()).unwrap_or("");

                    let item_name = item_id.and_then(|id| {
                        if id == 0 {
                            return None;
                        }
                        match data_class {
                            "weapon" => names.weapons.get(&id),
                            "armor" => names.armors.get(&id),
                            _ => None,
                        }
                        .cloned()
                    });

                    // Default data_class based on slot if empty
                    let data_class_str = if data_class.is_empty() {
                        if i == 0 { "weapon" } else { "armor" }
                    } else {
                        data_class
                    };

                    equipment.push(EquipSlot {
                        slot_name,
                        item_id: item_id.filter(|&id| id != 0),
                        item_name,
                        data_class: data_class_str.to_string(),
                    });
                }
            }

            // Skills
            let skills = actor_val
                .get("_skills")
                .and_then(|v| unwrap_array(v))
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_u64())
                        .map(|id| {
                            let id = id as u32;
                            NamedId {
                                id,
                                name: names
                                    .skills
                                    .get(&id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("Skill #{id}")),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            characters.push(Character {
                id,
                name,
                class_name,
                level,
                exp,
                stats,
                equipment,
                skills,
                states: Vec::new(),
            });
        }

        if characters.is_empty() {
            None
        } else {
            Some(characters)
        }
    }

    fn extract_inventory(raw: &serde_json::Value, names: &NameMap) -> Option<Inventory> {
        let party = raw.get("party")?;

        let extract_items = |key: &str, name_map: &std::collections::HashMap<u32, String>| -> Vec<InventoryItem> {
            party
                .get(key)
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(id_str, qty)| {
                            let id = id_str.parse::<u32>().ok()?;
                            let quantity = qty.as_u64()? as u32;
                            if quantity == 0 {
                                return None;
                            }
                            Some(InventoryItem {
                                id,
                                name: name_map
                                    .get(&id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("#{id}")),
                                quantity,
                                description: None,
                                category: None,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        let items = extract_items("_items", &names.items);
        let weapons = extract_items("_weapons", &names.weapons);
        let armors = extract_items("_armors", &names.armors);

        if items.is_empty() && weapons.is_empty() && armors.is_empty() {
            None
        } else {
            Some(Inventory {
                items,
                weapons,
                armors,
            })
        }
    }

    fn extract_currency(raw: &serde_json::Value) -> Option<CurrencyInfo> {
        let gold = raw.get("party")?.get("_gold")?.as_i64()?;
        Some(CurrencyInfo {
            label: "Gold".into(),
            amount: gold,
        })
    }

    fn extract_variables(raw: &serde_json::Value, names: &NameMap) -> Option<Vec<Variable>> {
        let data = unwrap_array(raw.get("variables")?.get("_data")?)?;
        let mut vars = Vec::new();

        for (i, val) in data.iter().enumerate() {
            if i == 0 {
                continue; // index 0 is unused
            }
            // Skip default/zero values to reduce noise
            let is_default = val.is_null() || (val.as_i64() == Some(0)) || (val.as_f64() == Some(0.0));
            let has_name = names.variables.contains_key(&(i as u32));

            if !is_default || has_name {
                vars.push(Variable {
                    id: i as u32,
                    name: names.variables.get(&(i as u32)).cloned(),
                    value: val.clone(),
                    group: None,
                });
            }
        }

        if vars.is_empty() {
            None
        } else {
            Some(vars)
        }
    }

    fn extract_switches(raw: &serde_json::Value, names: &NameMap) -> Option<Vec<Switch>> {
        let data = unwrap_array(raw.get("switches")?.get("_data")?)?;
        let mut switches = Vec::new();

        for (i, val) in data.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let value = val.as_bool().unwrap_or(false);
            let has_name = names.switches.contains_key(&(i as u32));

            if value || has_name {
                switches.push(Switch {
                    id: i as u32,
                    name: names.switches.get(&(i as u32)).cloned(),
                    value,
                });
            }
        }

        if switches.is_empty() {
            None
        } else {
            Some(switches)
        }
    }

    /// Apply structured edits back to the raw JSON
    fn apply_edits(raw: &mut serde_json::Value, data: &SaveData) {
        // Apply currency
        if let Some(ref currency) = data.currency {
            if let Some(party) = raw.get_mut("party") {
                party["_gold"] = serde_json::json!(currency.amount);
            }
        }

        // Apply variables
        if let Some(ref variables) = data.variables {
            if let Some(var_data) = raw
                .get_mut("variables")
                .and_then(|v| v.get_mut("_data"))
                .and_then(|v| unwrap_array_mut(v))
            {
                for var in variables {
                    if (var.id as usize) < var_data.len() {
                        var_data[var.id as usize] = var.value.clone();
                    }
                }
            }
        }

        // Apply switches
        if let Some(ref switches) = data.switches {
            if let Some(sw_data) = raw
                .get_mut("switches")
                .and_then(|v| v.get_mut("_data"))
                .and_then(|v| unwrap_array_mut(v))
            {
                for sw in switches {
                    if (sw.id as usize) < sw_data.len() {
                        sw_data[sw.id as usize] = serde_json::json!(sw.value);
                    }
                }
            }
        }

        // Apply inventory
        if let Some(ref inventory) = data.inventory {
            if let Some(party) = raw.get_mut("party") {
                let apply_inv = |key: &str, items: &[InventoryItem], party: &mut serde_json::Value| {
                    let mut obj = serde_json::Map::new();
                    for item in items {
                        if item.quantity > 0 {
                            obj.insert(item.id.to_string(), serde_json::json!(item.quantity));
                        }
                    }
                    party[key] = serde_json::Value::Object(obj);
                };
                apply_inv("_items", &inventory.items, party);
                apply_inv("_weapons", &inventory.weapons, party);
                apply_inv("_armors", &inventory.armors, party);
            }
        }

        // Apply party/actor edits
        if let Some(ref characters) = data.party {
            if let Some(actors_data) = raw
                .get_mut("actors")
                .and_then(|v| v.get_mut("_data"))
                .and_then(|v| unwrap_array_mut(v))
            {
                for character in characters {
                    // Find the matching actor in the raw data
                    for actor_val in actors_data.iter_mut() {
                        if actor_val.is_null() {
                            continue;
                        }
                        let actor_id = actor_val
                            .get("_actorId")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                        if actor_id != character.id {
                            continue;
                        }

                        actor_val["_level"] = serde_json::json!(character.level);
                        actor_val["_name"] = serde_json::json!(character.name);

                        // Apply stats
                        for stat in &character.stats {
                            match stat.key.as_str() {
                                "hp" => actor_val["_hp"] = serde_json::json!(stat.current),
                                "mp" => actor_val["_mp"] = serde_json::json!(stat.current),
                                "tp" => actor_val["_tp"] = serde_json::json!(stat.current),
                                _ => {
                                    // _paramPlus values
                                    let idx = match stat.key.as_str() {
                                        "mhp" => Some(0),
                                        "mmp" => Some(1),
                                        "atk" => Some(2),
                                        "def" => Some(3),
                                        "mat" => Some(4),
                                        "mdf" => Some(5),
                                        "agi" => Some(6),
                                        "luk" => Some(7),
                                        _ => None,
                                    };
                                    if let Some(idx) = idx {
                                        if let Some(params) = actor_val
                                            .get_mut("_paramPlus")
                                            .and_then(|v| unwrap_array_mut(v))
                                        {
                                            if idx < params.len() {
                                                params[idx] = serde_json::json!(stat.current);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Apply equipment
                        if let Some(equips) = actor_val
                            .get_mut("_equips")
                            .and_then(|v| unwrap_array_mut(v))
                        {
                            for (i, eq) in character.equipment.iter().enumerate() {
                                if i < equips.len() {
                                    equips[i]["_itemId"] =
                                        serde_json::json!(eq.item_id.unwrap_or(0));
                                    equips[i]["_dataClass"] = serde_json::json!(
                                        if eq.item_id.unwrap_or(0) == 0 { "" } else { &eq.data_class }
                                    );
                                }
                            }
                        }

                        break;
                    }
                }
            }
        }
    }
}

impl EnginePlugin for RpgMakerMvPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "rpg-maker-mv".into(),
            name: "RPG Maker MV/MZ".into(),
            icon: "rpg-maker-mv".into(),
            supports_debug: true,
            save_extensions: vec!["rpgsave".into(), "rmmzsave".into()],
            description: "RPG Maker MV and MZ game saves".into(),
            save_dir_hint: None,
            pick_mode: "folder".into(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        // MV: www/js/rpg_managers.js
        // MZ: js/rmmz_managers.js
        game_dir.join("www").join("js").join("rpg_managers.js").exists()
            || game_dir.join("js").join("rmmz_managers.js").exists()
            || game_dir.join("www").join("js").join("rmmz_managers.js").exists()
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let save_dir = Self::find_save_dir(game_dir)?;
        let mut saves = Vec::new();

        let entries = fs::read_dir(&save_dir)
            .map_err(|e| format!("Failed to read save directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if name.ends_with(".rpgsave") || name.ends_with(".rmmzsave") {
                // Skip config saves
                if name == "config.rpgsave" || name == "global.rpgsave" {
                    continue;
                }

                let meta = entry.metadata().map_err(|e| format!("Failed to get metadata: {e}"))?;
                let modified = meta
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<Local>::from(t).to_rfc3339())
                    .unwrap_or_default();

                saves.push(SaveFile {
                    path: path.to_string_lossy().to_string(),
                    name: name.replace(".rpgsave", "").replace(".rmmzsave", ""),
                    modified,
                    size: meta.len(),
                });
            }
        }

        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, game_dir: &Path) -> Result<SaveData, String> {
        let content = fs::read_to_string(save_path)
            .map_err(|e| format!("Failed to read save file: {e}"))?;

        let raw = Self::decode_save(content.trim())?;
        let name_map = self.resolve_names(game_dir).unwrap_or_default();
        let (party, inventory, currency, variables, switches) =
            Self::extract_structured(&raw, &name_map);

        Ok(SaveData {
            raw,
            party,
            inventory,
            currency,
            variables,
            switches,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        let mut raw = data.raw.clone();
        Self::apply_edits(&mut raw, data);

        let encoded = Self::encode_save(&raw)?;
        fs::write(save_path, encoded)
            .map_err(|e| format!("Failed to write save file: {e}"))?;
        Ok(())
    }

    fn resolve_names(&self, game_dir: &Path) -> Result<NameMap, String> {
        names::resolve_names(game_dir)
    }

    fn supports_debug_patch(&self) -> bool {
        true
    }

    fn apply_debug_patch(&self, game_dir: &Path) -> Result<PatchInfo, String> {
        // Find the js/plugins directory
        let plugins_dir = if game_dir.join("www").join("js").join("plugins").is_dir() {
            game_dir.join("www").join("js").join("plugins")
        } else if game_dir.join("js").join("plugins").is_dir() {
            game_dir.join("js").join("plugins")
        } else {
            return Err("Could not find plugins directory".into());
        };

        let plugin_path = plugins_dir.join("SaveEditorDebug.js");

        // Write the debug plugin
        let plugin_content = r#"/*:
 * @plugindesc Enable debug mode for Prismatic
 * @author Patrick
 *
 * @help Enables F9 (switch/variable editor) and F8 (console) in-game.
 * Remove this plugin to disable debug mode.
 */
(function() {
    // Enable F9 debug menu
    var _orig_isPlaytest = Utils.isOptionValid;
    Utils.isOptionValid = function(name) {
        if (name === 'test') return true;
        return _orig_isPlaytest.call(this, name);
    };

    // Also enable via the newer method if available
    if (typeof Utils.isNwjs === 'function') {
        var _Scene_Map_update = Scene_Map.prototype.update;
        Scene_Map.prototype.update = function() {
            _Scene_Map_update.call(this);
            if (Input.isTriggered('debug')) {
                SceneManager.push(Scene_Debug);
            }
        };
    }

    console.log('[SaveEditorDebug] Debug mode enabled. Press F9 for variable editor.');
})();
"#;

        fs::write(&plugin_path, plugin_content)
            .map_err(|e| format!("Failed to write debug plugin: {e}"))?;

        // Add to plugins.js
        let plugins_js_path = plugins_dir.parent().unwrap().join("plugins.js");
        if plugins_js_path.exists() {
            let content = fs::read_to_string(&plugins_js_path)
                .map_err(|e| format!("Failed to read plugins.js: {e}"))?;

            if !content.contains("SaveEditorDebug") {
                // Insert before the closing bracket
                let new_entry = r#"{"name":"SaveEditorDebug","status":true,"description":"Debug mode for Prismatic","parameters":{}}"#;
                let new_content = if content.trim().ends_with("];") {
                    let trimmed = content.trim_end();
                    let without_end = &trimmed[..trimmed.len() - 2];
                    if without_end.trim_end().ends_with('}') {
                        format!("{},\n{}\n];", without_end, new_entry)
                    } else {
                        format!("{}\n{}\n];", without_end, new_entry)
                    }
                } else {
                    content
                };
                fs::write(&plugins_js_path, new_content)
                    .map_err(|e| format!("Failed to update plugins.js: {e}"))?;
            }
        }

        Ok(PatchInfo {
            engine: "rpg-maker-mv".into(),
            game_dir: game_dir.to_string_lossy().to_string(),
            patches: vec![PatchEntry {
                file_path: plugin_path.to_string_lossy().to_string(),
                action: PatchAction::Created,
                original_hash: None,
            }],
            applied_at: Local::now().to_rfc3339(),
        })
    }

    fn revert_debug_patch(&self, game_dir: &Path, patch: &PatchInfo) -> Result<(), String> {
        for entry in &patch.patches {
            match &entry.action {
                PatchAction::Created => {
                    let path = Path::new(&entry.file_path);
                    if path.exists() {
                        fs::remove_file(path)
                            .map_err(|e| format!("Failed to remove {}: {e}", entry.file_path))?;
                    }
                }
                PatchAction::Modified { original } => {
                    fs::write(&entry.file_path, original)
                        .map_err(|e| format!("Failed to restore {}: {e}", entry.file_path))?;
                }
            }
        }

        // Remove from plugins.js
        let plugins_dir = if game_dir.join("www").join("js").join("plugins").is_dir() {
            game_dir.join("www").join("js")
        } else {
            game_dir.join("js")
        };

        let plugins_js_path = plugins_dir.join("plugins.js");
        if plugins_js_path.exists() {
            let content = fs::read_to_string(&plugins_js_path)
                .map_err(|e| format!("Failed to read plugins.js: {e}"))?;
            if content.contains("SaveEditorDebug") {
                let new_content = content
                    .replace(r#",{"name":"SaveEditorDebug","status":true,"description":"Debug mode for Prismatic","parameters":{}}"#, "")
                    .replace(r#"{"name":"SaveEditorDebug","status":true,"description":"Debug mode for Prismatic","parameters":{}},"#, "")
                    .replace(r#"{"name":"SaveEditorDebug","status":true,"description":"Debug mode for Prismatic","parameters":{}}"#, "");
                fs::write(&plugins_js_path, new_content)
                    .map_err(|e| format!("Failed to update plugins.js: {e}"))?;
            }
        }

        Ok(())
    }
}
