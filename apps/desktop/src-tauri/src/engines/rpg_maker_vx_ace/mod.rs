pub mod names;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use chrono::Local;
use marshal_rs::{self, Get, Value, ValueType};
use std::fs;
use std::path::Path;

pub struct RpgMakerVxaPlugin;

// ---------------------------------------------------------------------------
// Helper: get a symbol key from a HashMap-type Value
// ---------------------------------------------------------------------------

fn get_sym<'a>(hash: &'a Value, key: &str) -> Option<&'a Value> {
    Get::<&Value>::get(hash, &Value::symbol(key))
}

fn get_sym_mut<'a>(hash: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    Get::<&Value>::get_mut(hash, &Value::symbol(key))
}

// ---------------------------------------------------------------------------
// Extraction helpers
// ---------------------------------------------------------------------------

const EQUIP_SLOT_NAMES: [&str; 5] = ["Weapon", "Shield", "Head", "Body", "Accessory"];
const PARAM_KEYS: [&str; 8] = ["mhp", "mmp", "atk", "def", "mat", "mdf", "agi", "luk"];
const PARAM_LABELS: [&str; 8] = [
    "Max HP",
    "Max MP",
    "Attack",
    "Defense",
    "M.Attack",
    "M.Defense",
    "Agility",
    "Luck",
];

impl RpgMakerVxaPlugin {
    fn extract_party(contents: &Value, name_map: &NameMap) -> Option<Vec<Character>> {
        let actors_obj = get_sym(contents, "actors")?;
        let data_arr = Get::<&str>::get(actors_obj, "@data")?.as_array()?;

        let mut characters = Vec::new();
        for actor in data_arr.iter() {
            if actor.is_null() {
                continue;
            }

            let actor_id = match Get::<&str>::get(actor, "@actor_id").and_then(|v| v.as_int()) {
                Some(id) => id as u32,
                None => continue,
            };

            let name = Get::<&str>::get(actor, "@name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| name_map.actors.get(&actor_id).cloned())
                .unwrap_or_else(|| format!("Actor {}", actor_id));

            let class_id = Get::<&str>::get(actor, "@class_id")
                .and_then(|v| v.as_int())
                .unwrap_or(0) as u32;
            let class_name = name_map.classes.get(&class_id).cloned();

            let level = Get::<&str>::get(actor, "@level")
                .and_then(|v| v.as_int())
                .unwrap_or(1) as u32;

            // Experience: @exp is a HashMap {class_id_int => total_exp_int}
            let exp = Get::<&str>::get(actor, "@exp")
                .and_then(|v| v.as_hashmap())
                .and_then(|hm| {
                    // Find entry matching current class_id
                    hm.iter().find_map(|(k, v)| {
                        if k.as_int() == Some(class_id as i32) {
                            v.as_int().map(|e| e as u64)
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(0);

            // Stats
            let hp = Get::<&str>::get(actor, "@hp")
                .and_then(|v| v.as_int())
                .unwrap_or(0);
            let mp = Get::<&str>::get(actor, "@mp")
                .and_then(|v| v.as_int())
                .unwrap_or(0);
            let tp = Get::<&str>::get(actor, "@tp")
                .and_then(|v| v.as_int())
                .unwrap_or(0);

            let mut stats = vec![
                Stat {
                    key: "hp".into(),
                    label: "HP".into(),
                    current: hp as f64,
                    max: None,
                },
                Stat {
                    key: "mp".into(),
                    label: "MP".into(),
                    current: mp as f64,
                    max: None,
                },
                Stat {
                    key: "tp".into(),
                    label: "TP".into(),
                    current: tp as f64,
                    max: None,
                },
            ];

            // @param_plus bonus stats
            if let Some(param_arr) = Get::<&str>::get(actor, "@param_plus").and_then(|v| v.as_array()) {
                for (i, val) in param_arr.iter().enumerate() {
                    if i < 8 {
                        let bonus = val.as_int().unwrap_or(0);
                        stats.push(Stat {
                            key: format!("{}_plus", PARAM_KEYS[i]),
                            label: format!("{} Bonus", PARAM_LABELS[i]),
                            current: bonus as f64,
                            max: None,
                        });
                    }
                }
            }

            // Equipment: @equips is array of Game_BaseItem objects
            let mut equipment = Vec::new();
            if let Some(equips_arr) = Get::<&str>::get(actor, "@equips").and_then(|v| v.as_array()) {
                for (i, equip) in equips_arr.iter().enumerate() {
                    let slot_name = EQUIP_SLOT_NAMES
                        .get(i)
                        .unwrap_or(&"Unknown")
                        .to_string();

                    let item_id = Get::<&str>::get(equip, "@item_id")
                        .and_then(|v| v.as_int())
                        .unwrap_or(0) as u32;

                    let is_weapon = Get::<&str>::get(equip, "@is_weapon")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let data_class = if is_weapon { "weapon" } else { "armor" };

                    let (opt_id, item_name) = if item_id == 0 {
                        (None, None)
                    } else {
                        let names = if is_weapon {
                            &name_map.weapons
                        } else {
                            &name_map.armors
                        };
                        (Some(item_id), names.get(&item_id).cloned())
                    };

                    equipment.push(EquipSlot {
                        slot_name,
                        item_id: opt_id,
                        item_name,
                        data_class: data_class.into(),
                    });
                }
            }

            // Skills: @skills is array of skill ID ints
            let skills = Get::<&str>::get(actor, "@skills")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            let id = v.as_int()? as u32;
                            let name = name_map
                                .skills
                                .get(&id)
                                .cloned()
                                .unwrap_or_else(|| format!("Skill {}", id));
                            Some(NamedId { id, name })
                        })
                        .collect()
                })
                .unwrap_or_default();

            characters.push(Character {
                id: actor_id,
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

    fn extract_inventory(contents: &Value, name_map: &NameMap) -> Option<Inventory> {
        let party = get_sym(contents, "party")?;

        let extract_category =
            |ivar: &str, names: &std::collections::HashMap<u32, String>, category: &str| -> Vec<InventoryItem> {
                Get::<&str>::get(party, ivar)
                    .and_then(|v| v.as_hashmap())
                    .map(|hm| {
                        hm.iter()
                            .filter_map(|(k, v)| {
                                let id = k.as_int()? as u32;
                                let qty = v.as_int()? as u32;
                                let name = names
                                    .get(&id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("{} {}", category, id));
                                Some(InventoryItem {
                                    id,
                                    name,
                                    quantity: qty,
                                    description: None,
                                    category: Some(category.to_string()),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            };

        let items = extract_category("@items", &name_map.items, "Item");
        let weapons = extract_category("@weapons", &name_map.weapons, "Weapon");
        let armors = extract_category("@armors", &name_map.armors, "Armor");

        Some(Inventory {
            items,
            weapons,
            armors,
        })
    }

    fn extract_currency(contents: &Value) -> Option<CurrencyInfo> {
        let party = get_sym(contents, "party")?;
        let gold = Get::<&str>::get(party, "@gold")?.as_int()?;
        Some(CurrencyInfo {
            label: "Gold".into(),
            amount: gold as i64,
        })
    }

    fn extract_variables(contents: &Value, name_map: &NameMap) -> Option<Vec<Variable>> {
        let variables_obj = get_sym(contents, "variables")?;
        let data = Get::<&str>::get(variables_obj, "@data")?.as_array()?;

        let mut result = Vec::new();
        for (i, val) in data.iter().enumerate() {
            if i == 0 {
                continue;
            }
            // Skip null and zero values
            if val.is_null() {
                continue;
            }
            if val.as_int() == Some(0) {
                continue;
            }

            let json_val = if let Some(n) = val.as_int() {
                serde_json::Value::Number(n.into())
            } else if let Some(s) = val.as_str() {
                serde_json::Value::String(s.to_string())
            } else if let Some(b) = val.as_bool() {
                serde_json::Value::Bool(b)
            } else {
                // For other types, serialize as string representation
                serde_json::Value::String(val.to_string())
            };

            let name = name_map.variables.get(&(i as u32)).cloned();

            result.push(Variable {
                id: i as u32,
                name,
                value: json_val,
                group: None,
            });
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn extract_switches(contents: &Value, name_map: &NameMap) -> Option<Vec<Switch>> {
        let switches_obj = get_sym(contents, "switches")?;
        let data = Get::<&str>::get(switches_obj, "@data")?.as_array()?;

        let mut result = Vec::new();
        for (i, val) in data.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let value = val.as_bool().unwrap_or(false);
            let name = name_map.switches.get(&(i as u32)).cloned();

            result.push(Switch {
                id: i as u32,
                name,
                value,
            });
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    // -----------------------------------------------------------------------
    // Write helpers: apply edits from SaveData back to marshal Value tree
    // -----------------------------------------------------------------------

    fn apply_currency(contents: &mut Value, data: &SaveData) {
        if let Some(ref currency) = data.currency {
            if let Some(party) = get_sym_mut(contents, "party") {
                if let Some(gold) = Get::<&str>::get_mut(party, "@gold") {
                    gold.set_value(ValueType::Integer(currency.amount as i32));
                }
            }
        }
    }

    fn apply_variables(contents: &mut Value, data: &SaveData) {
        if let Some(ref variables) = data.variables {
            if let Some(vars_obj) = get_sym_mut(contents, "variables") {
                if let Some(data_arr) = Get::<&str>::get_mut(vars_obj, "@data") {
                    for var in variables {
                        let idx = var.id as usize;
                        if let Some(slot) = data_arr.get_index_mut(idx) {
                            slot.set_value(json_to_value_type(&var.value));
                        }
                    }
                }
            }
        }
    }

    fn apply_switches(contents: &mut Value, data: &SaveData) {
        if let Some(ref switches) = data.switches {
            if let Some(sw_obj) = get_sym_mut(contents, "switches") {
                if let Some(data_arr) = Get::<&str>::get_mut(sw_obj, "@data") {
                    for sw in switches {
                        let idx = sw.id as usize;
                        if let Some(slot) = data_arr.get_index_mut(idx) {
                            slot.set_value(ValueType::Bool(sw.value));
                        }
                    }
                }
            }
        }
    }

    fn apply_party(contents: &mut Value, data: &SaveData) {
        let characters = match data.party {
            Some(ref c) => c,
            None => return,
        };

        let actors_obj = match get_sym_mut(contents, "actors") {
            Some(v) => v,
            None => return,
        };
        let data_arr = match Get::<&str>::get_mut(actors_obj, "@data") {
            Some(v) => v,
            None => return,
        };
        let arr = match data_arr.as_array_mut() {
            Some(a) => a,
            None => return,
        };

        for character in characters {
            // Find the actor in the array by @actor_id
            let actor = match arr.iter_mut().find(|a| {
                !a.is_null()
                    && Get::<&str>::get(&**a, "@actor_id")
                        .and_then(|v: &Value| v.as_int())
                        .map(|id| id as u32 == character.id)
                        .unwrap_or(false)
            }) {
                Some(a) => a,
                None => continue,
            };

            // Set @name
            if let Some(name_val) = Get::<&str>::get_mut(actor, "@name") {
                name_val.set_value(ValueType::String(character.name.clone()));
            }

            // Set @level
            if let Some(level_val) = Get::<&str>::get_mut(actor, "@level") {
                level_val.set_value(ValueType::Integer(character.level as i32));
            }

            // Apply stats
            for stat in &character.stats {
                match stat.key.as_str() {
                    "hp" => {
                        if let Some(v) = Get::<&str>::get_mut(actor, "@hp") {
                            v.set_value(ValueType::Integer(stat.current as i32));
                        }
                    }
                    "mp" => {
                        if let Some(v) = Get::<&str>::get_mut(actor, "@mp") {
                            v.set_value(ValueType::Integer(stat.current as i32));
                        }
                    }
                    "tp" => {
                        if let Some(v) = Get::<&str>::get_mut(actor, "@tp") {
                            v.set_value(ValueType::Integer(stat.current as i32));
                        }
                    }
                    key if key.ends_with("_plus") => {
                        // e.g. "mhp_plus" -> index 0, "mmp_plus" -> index 1, etc.
                        let param_name = &key[..key.len() - 5]; // strip "_plus"
                        if let Some(idx) = PARAM_KEYS.iter().position(|&k| k == param_name) {
                            if let Some(param_arr) =
                                Get::<&str>::get_mut(actor, "@param_plus")
                            {
                                if let Some(slot) = param_arr.get_index_mut(idx) {
                                    slot.set_value(ValueType::Integer(stat.current as i32));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Apply equipment
            if let Some(equips_val) = Get::<&str>::get_mut(actor, "@equips") {
                if let Some(equips_arr) = equips_val.as_array_mut() {
                    for (i, equip_slot) in character.equipment.iter().enumerate() {
                        if let Some(equip) = equips_arr.get_mut(i) {
                            let item_id = equip_slot.item_id.unwrap_or(0) as i32;
                            let is_weapon = equip_slot.data_class == "weapon";

                            if let Some(id_val) = Get::<&str>::get_mut(equip, "@item_id") {
                                id_val.set_value(ValueType::Integer(item_id));
                            }
                            if let Some(wep_val) = Get::<&str>::get_mut(equip, "@is_weapon") {
                                wep_val.set_value(ValueType::Bool(is_weapon));
                            }
                        }
                    }
                }
            }
        }
    }

    fn apply_inventory(contents: &mut Value, data: &SaveData) {
        let inventory = match data.inventory {
            Some(ref inv) => inv,
            None => return,
        };

        let party = match get_sym_mut(contents, "party") {
            Some(v) => v,
            None => return,
        };

        // Rebuild @items HashMap
        Self::rebuild_inventory_hash(party, "@items", &inventory.items);
        Self::rebuild_inventory_hash(party, "@weapons", &inventory.weapons);
        Self::rebuild_inventory_hash(party, "@armors", &inventory.armors);
    }

    fn rebuild_inventory_hash(party: &mut Value, ivar: &str, items: &[InventoryItem]) {
        if let Some(hash_val) = Get::<&str>::get_mut(party, ivar) {
            let mut new_map = marshal_rs::HashMap::new();
            for item in items {
                new_map.insert(
                    Value::int(item.id as i32),
                    Value::int(item.quantity as i32),
                );
            }
            hash_val.set_value(ValueType::HashMap(new_map));
        }
    }
}

/// Convert a serde_json::Value to a marshal_rs ValueType
fn json_to_value_type(json: &serde_json::Value) -> ValueType {
    match json {
        serde_json::Value::Null => ValueType::Null,
        serde_json::Value::Bool(b) => ValueType::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ValueType::Integer(i as i32)
            } else if let Some(f) = n.as_f64() {
                ValueType::Float(f.to_string())
            } else {
                ValueType::Null
            }
        }
        serde_json::Value::String(s) => ValueType::String(s.clone()),
        serde_json::Value::Array(arr) => {
            ValueType::Array(arr.iter().map(|v| Value::from(v.clone())).collect())
        }
        serde_json::Value::Object(_) => {
            // For complex objects, use the From<serde_json::Value> impl
            let v = Value::from(json.clone());
            // We need the inner ValueType, but since we just created it via From,
            // we can't easily extract it. Use set_value pattern instead.
            // Actually, the simplest approach: just return Object variant
            // But we don't have direct access to val field. Let's work around this.
            // The From<serde_json::Value> creates a Value::object(...), so we know it's Object.
            // We can match on the Deref to ValueType.
            match &*v {
                ValueType::Object(o) => ValueType::Object(o.clone()),
                _ => ValueType::Null,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Marshal stream splitter
// ---------------------------------------------------------------------------

/// Decode a Marshal fixnum from `buf`, returning (value, bytes_consumed).
fn read_fixnum(buf: &[u8]) -> Result<(i64, usize), String> {
    if buf.is_empty() {
        return Err("Unexpected end of data reading fixnum".into());
    }
    let b = buf[0] as i8;
    match b {
        0 => Ok((0, 1)),
        // 1..=4  => next N bytes, positive little-endian
        1..=4 => {
            let n = b as usize;
            if buf.len() < 1 + n {
                return Err("Unexpected end of data in positive fixnum".into());
            }
            let mut val: i64 = 0;
            for i in 0..n {
                val |= (buf[1 + i] as i64) << (8 * i);
            }
            Ok((val, 1 + n))
        }
        // -4..=-1  => next N bytes, negative little-endian
        -4..=-1 => {
            let n = (-b) as usize;
            if buf.len() < 1 + n {
                return Err("Unexpected end of data in negative fixnum".into());
            }
            // Start with all-ones to sign-extend
            let mut val: i64 = -1;
            for i in 0..n {
                // Clear the byte slot then OR in the actual byte
                val &= !(0xFF_i64 << (8 * i));
                val |= (buf[1 + i] as i64) << (8 * i);
            }
            Ok((val, 1 + n))
        }
        // 5..=127  => value - 5
        5..=127 => Ok(((b as i64) - 5, 1)),
        // -128..=-5  => value + 5
        _ => Ok(((b as i64) + 5, 1)),
    }
}

/// Walk one complete Marshal value starting at `buf`, returning bytes consumed.
/// This does NOT include the 2-byte version header (\x04\x08).
fn marshal_value_size(buf: &[u8]) -> Result<usize, String> {
    if buf.is_empty() {
        return Err("Unexpected end of data: expected type tag".into());
    }
    let tag = buf[0];
    let mut pos: usize = 1; // past the tag

    match tag {
        // nil, true, false  — no payload
        0x30 | 0x54 | 0x46 => Ok(pos),

        // fixnum
        0x69 => {
            let (_, consumed) = read_fixnum(&buf[pos..])?;
            Ok(pos + consumed)
        }

        // symbol (new) — fixnum length + that many bytes
        0x3a => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in symbol".into());
            }
            Ok(pos + len)
        }

        // symlink / objlink — just a fixnum index
        0x3b | 0x40 => {
            let (_, consumed) = read_fixnum(&buf[pos..])?;
            Ok(pos + consumed)
        }

        // raw string (\x22) — fixnum length + bytes
        0x22 => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in raw string".into());
            }
            Ok(pos + len)
        }

        // IVAR wrapper — one value + fixnum pair-count + (symbol, value) pairs
        0x49 => {
            let inner = marshal_value_size(&buf[pos..])?;
            pos += inner;
            let (pair_count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..pair_count {
                let k = marshal_value_size(&buf[pos..])?;
                pos += k;
                let v = marshal_value_size(&buf[pos..])?;
                pos += v;
            }
            Ok(pos)
        }

        // array
        0x5b => {
            let (count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..count {
                let sz = marshal_value_size(&buf[pos..])?;
                pos += sz;
            }
            Ok(pos)
        }

        // hash
        0x7b => {
            let (count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..count {
                let k = marshal_value_size(&buf[pos..])?;
                pos += k;
                let v = marshal_value_size(&buf[pos..])?;
                pos += v;
            }
            Ok(pos)
        }

        // hash with default — same as hash + one more value (the default)
        0x7c => {
            let (count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..count {
                let k = marshal_value_size(&buf[pos..])?;
                pos += k;
                let v = marshal_value_size(&buf[pos..])?;
                pos += v;
            }
            // default value
            let def = marshal_value_size(&buf[pos..])?;
            pos += def;
            Ok(pos)
        }

        // object (\x6f) — symbol (class name) + fixnum pair-count + pairs
        0x6f => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let (pair_count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..pair_count {
                let k = marshal_value_size(&buf[pos..])?;
                pos += k;
                let v = marshal_value_size(&buf[pos..])?;
                pos += v;
            }
            Ok(pos)
        }

        // uclass (\x43) — wraps another class around a value: symbol + value
        0x43 => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let inner = marshal_value_size(&buf[pos..])?;
            pos += inner;
            Ok(pos)
        }

        // extended (\x65) — symbol + value
        0x65 => {
            let mod_name = marshal_value_size(&buf[pos..])?;
            pos += mod_name;
            let inner = marshal_value_size(&buf[pos..])?;
            pos += inner;
            Ok(pos)
        }

        // userdef (\x75) — symbol + fixnum length + raw bytes
        0x75 => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in userdef".into());
            }
            pos += len;
            Ok(pos)
        }

        // usrmarshal (\x55) — symbol + value
        0x55 => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let inner = marshal_value_size(&buf[pos..])?;
            pos += inner;
            Ok(pos)
        }

        // data (\x64) — symbol + value
        0x64 => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let inner = marshal_value_size(&buf[pos..])?;
            pos += inner;
            Ok(pos)
        }

        // float (\x66) — fixnum length + that many ASCII bytes
        0x66 => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in float".into());
            }
            pos += len;
            Ok(pos)
        }

        // class (\x63) or module (\x6d) — fixnum length + that many bytes (class name)
        0x63 | 0x6d => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in class/module".into());
            }
            pos += len;
            Ok(pos)
        }

        // old module (\x4d) — fixnum length + that many bytes
        0x4d => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len {
                return Err("Unexpected end of data in old module".into());
            }
            pos += len;
            Ok(pos)
        }

        // regexp (\x2f) — fixnum length + bytes + 1 flags byte
        0x2f => {
            let (len, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let len = len as usize;
            if buf.len() < pos + len + 1 {
                return Err("Unexpected end of data in regexp".into());
            }
            pos += len + 1; // bytes + flags byte
            Ok(pos)
        }

        // struct (\x53) — symbol (class name) + fixnum pair-count + pairs
        0x53 => {
            let cls = marshal_value_size(&buf[pos..])?;
            pos += cls;
            let (pair_count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            for _ in 0..pair_count {
                let k = marshal_value_size(&buf[pos..])?;
                pos += k;
                let v = marshal_value_size(&buf[pos..])?;
                pos += v;
            }
            Ok(pos)
        }

        // bigint (\x6c) — sign byte + fixnum word-count + 2*word-count bytes
        0x6c => {
            if buf.len() < pos + 1 {
                return Err("Unexpected end of data in bigint sign".into());
            }
            pos += 1; // sign byte ('+' or '-')
            let (word_count, consumed) = read_fixnum(&buf[pos..])?;
            pos += consumed;
            let byte_count = (word_count as usize) * 2;
            if buf.len() < pos + byte_count {
                return Err("Unexpected end of data in bigint".into());
            }
            pos += byte_count;
            Ok(pos)
        }

        other => Err(format!(
            "Unknown Marshal type tag 0x{:02x} at stream position",
            other
        )),
    }
}

/// Split a buffer containing multiple sequential Marshal dumps.
/// Each dump starts with the 2-byte header `\x04\x08`.
pub fn split_marshal_dumps(buf: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut dumps = Vec::new();
    let mut offset = 0;

    while offset < buf.len() {
        // Each dump starts with version bytes 0x04 0x08
        if buf.len() < offset + 2 {
            return Err("Incomplete Marshal version header".into());
        }
        if buf[offset] != 0x04 || buf[offset + 1] != 0x08 {
            return Err(format!(
                "Expected Marshal header (\\x04\\x08) at offset {}, got \\x{:02x}\\x{:02x}",
                offset, buf[offset], buf[offset + 1]
            ));
        }

        let value_start = offset + 2;
        let value_size = marshal_value_size(&buf[value_start..])
            .map_err(|e| format!("Error at offset {}: {}", value_start, e))?;

        let dump_end = value_start + value_size;
        dumps.push(&buf[offset..dump_end]);
        offset = dump_end;
    }

    Ok(dumps)
}

// ---------------------------------------------------------------------------
// EnginePlugin implementation
// ---------------------------------------------------------------------------

impl EnginePlugin for RpgMakerVxaPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "rpg-maker-vx-ace".into(),
            name: "RPG Maker VX Ace".into(),
            icon: "rpg-maker-vx-ace".into(),
            supports_debug: true,
            save_extensions: vec!["rvdata2".into()],
            description: "RPG Maker VX Ace game saves".into(),
            save_dir_hint: Some(
                "Select the game folder containing your .rvdata2 save files.\n\
                 Save files are usually in the game's root directory."
                    .to_string(),
            ),
            pick_mode: "folder".into(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        // Must have Data/Actors.rvdata2 (VX Ace data file)
        let has_rvdata2 = game_dir.join("Data").join("Actors.rvdata2").exists();
        // Must NOT have www/js/ (that would be MV/MZ)
        let has_mv = game_dir.join("www").join("js").exists()
            || game_dir.join("js").join("rmmz_managers.js").exists();
        has_rvdata2 && !has_mv
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let mut saves = Vec::new();

        let entries = fs::read_dir(game_dir)
            .map_err(|e| format!("Failed to read game directory: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Match Save*.rvdata2 but skip Volume.rvdata2 and other non-save files
            if name.ends_with(".rvdata2") && name.starts_with("Save") {
                let meta = entry
                    .metadata()
                    .map_err(|e| format!("Failed to get metadata: {e}"))?;
                let modified = meta
                    .modified()
                    .ok()
                    .map(crate::engines::utils::format_modified_time)
                    .unwrap_or_default();

                saves.push(SaveFile {
                    path: path.to_string_lossy().to_string(),
                    name: name.replace(".rvdata2", ""),
                    modified,
                    size: meta.len(),
                });
            }
        }

        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, game_dir: &Path) -> Result<SaveData, String> {
        let buf = fs::read(save_path).map_err(|e| format!("Failed to read save: {e}"))?;
        let dumps = split_marshal_dumps(&buf)?;
        if dumps.len() < 2 {
            return Err("Invalid VX Ace save: expected at least 2 Marshal dumps".into());
        }

        let contents = marshal_rs::load(dumps[1], None)
            .map_err(|e| format!("Failed to parse save contents: {e}"))?;

        // Convert to serde_json for raw view
        let raw_json: serde_json::Value = serde_json::from_str(&contents.to_string())
            .map_err(|e| format!("Failed to convert to JSON: {e}"))?;

        let name_map = self.resolve_names(game_dir).unwrap_or_default();

        let party = Self::extract_party(&contents, &name_map);
        let inventory = Self::extract_inventory(&contents, &name_map);
        let currency = Self::extract_currency(&contents);
        let variables = Self::extract_variables(&contents, &name_map);
        let switches = Self::extract_switches(&contents, &name_map);

        Ok(SaveData {
            raw: raw_json,
            party,
            inventory,
            currency,
            variables,
            switches,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        // 1. Re-read the original file and split into Marshal dumps
        let buf = fs::read(save_path).map_err(|e| format!("Failed to read save: {e}"))?;
        let dumps = split_marshal_dumps(&buf)?;
        if dumps.len() < 2 {
            return Err("Invalid VX Ace save: expected at least 2 Marshal dumps".into());
        }

        // 2. Parse header and contents
        let header = marshal_rs::load(dumps[0], None)
            .map_err(|e| format!("Failed to parse header: {e}"))?;
        let mut contents = marshal_rs::load(dumps[1], None)
            .map_err(|e| format!("Failed to parse contents: {e}"))?;

        // 3. Apply edits
        Self::apply_currency(&mut contents, data);
        Self::apply_variables(&mut contents, data);
        Self::apply_switches(&mut contents, data);
        Self::apply_party(&mut contents, data);
        Self::apply_inventory(&mut contents, data);

        // 4. Re-dump
        let mut out = marshal_rs::dump(header, None);
        let contents_bytes = marshal_rs::dump(contents, None);

        // 5. Safety: verify the dumped contents can be loaded back
        //    This prevents writing a corrupted save file.
        if let Err(e) = std::panic::catch_unwind(|| marshal_rs::load(&contents_bytes, None)) {
            return Err(format!(
                "Marshal round-trip validation failed (dump produced invalid output): {:?}",
                e
            ));
        }

        out.extend(contents_bytes);
        fs::write(save_path, &out).map_err(|e| format!("Failed to write save: {e}"))?;

        Ok(())
    }

    fn resolve_names(&self, game_dir: &Path) -> Result<NameMap, String> {
        names::resolve_names(game_dir)
    }

    fn supports_debug_patch(&self) -> bool {
        true
    }

    fn apply_debug_patch(&self, game_dir: &Path) -> Result<PatchInfo, String> {
        if cfg!(target_os = "windows") {
            let lnk_path = game_dir.join("Debug Mode.lnk");
            let game_exe = game_dir.join("Game.exe");

            let ps_script = format!(
                "$ws = New-Object -ComObject WScript.Shell; \
                 $s = $ws.CreateShortcut('{}'); \
                 $s.TargetPath = '{}'; \
                 $s.Arguments = 'test console'; \
                 $s.WorkingDirectory = '{}'; \
                 $s.Description = 'Launch with debug mode (F9=variables, F8=console)'; \
                 $s.Save()",
                lnk_path.to_string_lossy().replace('\'', "''"),
                game_exe.to_string_lossy().replace('\'', "''"),
                game_dir.to_string_lossy().replace('\'', "''"),
            );

            let output = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to create shortcut: {stderr}"));
            }

            Ok(PatchInfo {
                engine: "rpg-maker-vx-ace".into(),
                game_dir: game_dir.to_string_lossy().to_string(),
                patches: vec![PatchEntry {
                    file_path: lnk_path.to_string_lossy().to_string(),
                    action: PatchAction::Created,
                    original_hash: None,
                }],
                applied_at: Local::now().to_rfc3339(),
            })
        } else {
            let sh_path = game_dir.join("debug-mode.sh");

            let script = "#!/bin/bash\n\
                 cd \"$(dirname \"$0\")\"\n\
                 if ! command -v wine &>/dev/null; then\n\
                     echo \"Error: wine not found. Install Wine or adjust PATH.\" >&2\n\
                     exit 1\n\
                 fi\n\
                 wine Game.exe test console\n";

            fs::write(&sh_path, script)
                .map_err(|e| format!("Failed to write debug script: {e}"))?;

            // Make script executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&sh_path, fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("Failed to set script permissions: {e}"))?;
            }

            Ok(PatchInfo {
                engine: "rpg-maker-vx-ace".into(),
                game_dir: game_dir.to_string_lossy().to_string(),
                patches: vec![PatchEntry {
                    file_path: sh_path.to_string_lossy().to_string(),
                    action: PatchAction::Created,
                    original_hash: None,
                }],
                applied_at: Local::now().to_rfc3339(),
            })
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_fixnum_zero() {
        let (val, consumed) = read_fixnum(&[0x00]).unwrap();
        assert_eq!(val, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_read_fixnum_small_positive() {
        // 0x06 => 6 - 5 = 1, 0x07 => 2, etc.
        let (val, consumed) = read_fixnum(&[0x06]).unwrap();
        assert_eq!(val, 1);
        assert_eq!(consumed, 1);

        let (val, _) = read_fixnum(&[0x7f]).unwrap();
        assert_eq!(val, 122); // 127 - 5
    }

    #[test]
    fn test_read_fixnum_small_negative() {
        // 0xfa => -6 + 5 = -1
        let (val, consumed) = read_fixnum(&[0xfa]).unwrap();
        assert_eq!(val, -1);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_read_fixnum_multibyte_positive() {
        // 0x01 means 1 byte follows
        let (val, consumed) = read_fixnum(&[0x01, 0xff]).unwrap();
        assert_eq!(val, 255);
        assert_eq!(consumed, 2);

        // 0x02 means 2 bytes follow, little-endian
        let (val, consumed) = read_fixnum(&[0x02, 0x00, 0x01]).unwrap();
        assert_eq!(val, 256);
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_split_single_nil_dump() {
        // \x04\x08 + nil (0x30)
        let buf = vec![0x04, 0x08, 0x30];
        let dumps = split_marshal_dumps(&buf).unwrap();
        assert_eq!(dumps.len(), 1);
        assert_eq!(dumps[0], &[0x04, 0x08, 0x30]);
    }

    #[test]
    fn test_split_two_dumps() {
        // Two nil dumps
        let buf = vec![0x04, 0x08, 0x30, 0x04, 0x08, 0x30];
        let dumps = split_marshal_dumps(&buf).unwrap();
        assert_eq!(dumps.len(), 2);
    }

    #[test]
    fn test_split_hash_then_nil() {
        // Empty hash {} then nil
        // hash: 0x7b + fixnum(0) = 0x7b 0x00
        let buf = vec![0x04, 0x08, 0x7b, 0x00, 0x04, 0x08, 0x30];
        let dumps = split_marshal_dumps(&buf).unwrap();
        assert_eq!(dumps.len(), 2);
        assert_eq!(dumps[0], &[0x04, 0x08, 0x7b, 0x00]);
        assert_eq!(dumps[1], &[0x04, 0x08, 0x30]);
    }

    #[test]
    #[ignore]
    fn test_real_save() {
        use marshal_rs::{load, Value, Get};

        let path = r"D:\Personalisation\Avatar\saves\Anera\Anera 1.31 Final\Save01.rvdata2";
        let buf = std::fs::read(path).expect("Failed to read save file");
        println!("File size: {} bytes", buf.len());

        let dumps = split_marshal_dumps(&buf).expect("Failed to split dumps");
        println!("Number of dumps: {}", dumps.len());
        for (i, d) in dumps.iter().enumerate() {
            println!("  dump[{}]: {} bytes", i, d.len());
        }

        // Parse the second dump (contents / game data)
        let contents = load(dumps[1], None).expect("Failed to parse dump[1]");

        // What type is the top-level?
        println!("\n=== TOP-LEVEL TYPE ===");
        println!("class_name: {:?}", contents.class_name());
        println!("is_object: {}", contents.is_object());
        println!("is_hash_map: {}", contents.is_hash_map());
        println!("is_array: {}", contents.is_array());

        // If it's a HashMap, list all keys
        if let Some(map) = contents.as_hashmap() {
            println!("\n=== TOP-LEVEL HASHMAP KEYS ({} entries) ===", map.len());
            for (k, v) in map.iter() {
                let key_str = if let Some(s) = k.as_str() {
                    s.to_string()
                } else {
                    format!("{:?}", k)
                };
                let val_summary = if v.is_object() {
                    format!("Object(class={})", v.class_name())
                } else if v.is_array() {
                    format!("Array(len={})", v.as_array().unwrap().len())
                } else if v.is_hash_map() {
                    format!("HashMap(len={})", v.as_hashmap().unwrap().len())
                } else if v.is_null() {
                    "Null".to_string()
                } else if v.is_integer() {
                    format!("Int({})", v.as_int().unwrap())
                } else if v.is_bool() {
                    format!("Bool({})", v.as_bool().unwrap())
                } else if v.is_string() {
                    format!("String({:?})", v.as_str().unwrap())
                } else {
                    format!("Other(class={})", v.class_name())
                };
                println!("  {} => {}", key_str, val_summary);
            }
        }

        // If it's an Object, list all ivars
        if let Some(obj) = contents.as_object() {
            println!("\n=== TOP-LEVEL OBJECT IVARS ({} entries, class={}) ===", obj.len(), contents.class_name());
            for (k, v) in obj.iter() {
                let val_summary = if v.is_object() {
                    format!("Object(class={})", v.class_name())
                } else if v.is_array() {
                    format!("Array(len={})", v.as_array().unwrap().len())
                } else if v.is_hash_map() {
                    format!("HashMap(len={})", v.as_hashmap().unwrap().len())
                } else if v.is_null() {
                    "Null".to_string()
                } else if v.is_integer() {
                    format!("Int({})", v.as_int().unwrap())
                } else if v.is_bool() {
                    format!("Bool({})", v.as_bool().unwrap())
                } else if v.is_string() {
                    format!("String({:?})", v.as_str().unwrap())
                } else {
                    format!("Other(class={}, type={})", v.class_name(), v.value_type())
                };
                println!("  {} => {}", k, val_summary);
            }
        }

        // Helper to explore an object's ivars
        let explore_object = |label: &str, val: &Value, max_depth: usize| {
            println!("\n=== {} (class={}) ===", label, val.class_name());
            if let Some(obj) = val.as_object() {
                for (k, v) in obj.iter() {
                    let val_summary = if v.is_object() {
                        format!("Object(class={})", v.class_name())
                    } else if v.is_array() {
                        let arr = v.as_array().unwrap();
                        let preview: Vec<String> = arr.iter().take(5).map(|e| {
                            if e.is_integer() { format!("{}", e.as_int().unwrap()) }
                            else if e.is_null() { "nil".into() }
                            else if e.is_string() { format!("{:?}", e.as_str().unwrap()) }
                            else if e.is_object() { format!("Obj({})", e.class_name()) }
                            else if e.is_bool() { format!("{}", e.as_bool().unwrap()) }
                            else { format!("?(type={})", e.value_type()) }
                        }).collect();
                        format!("Array(len={}, first_5={:?})", arr.len(), preview)
                    } else if v.is_hash_map() {
                        let hm = v.as_hashmap().unwrap();
                        let preview: Vec<String> = hm.iter().take(3).map(|(hk, hv)| {
                            let ks = if let Some(s) = hk.as_str() { s.to_string() }
                                     else if hk.is_integer() { format!("{}", hk.as_int().unwrap()) }
                                     else { format!("{:?}", hk) };
                            let vs = if hv.is_integer() { format!("{}", hv.as_int().unwrap()) }
                                     else if hv.is_null() { "nil".into() }
                                     else if hv.is_string() { format!("{:?}", hv.as_str().unwrap()) }
                                     else { format!("?(type={})", hv.value_type()) };
                            format!("{}=>{}", ks, vs)
                        }).collect();
                        format!("HashMap(len={}, sample={:?})", hm.len(), preview)
                    } else if v.is_null() {
                        "Null".to_string()
                    } else if v.is_integer() {
                        format!("Int({})", v.as_int().unwrap())
                    } else if v.is_float() {
                        format!("Float({})", v.as_str().unwrap_or("?"))
                    } else if v.is_bool() {
                        format!("Bool({})", v.as_bool().unwrap())
                    } else if v.is_string() {
                        format!("String({:?})", v.as_str().unwrap())
                    } else if v.is_bytes() {
                        format!("Bytes(len={})", v.as_byte_vec().unwrap().len())
                    } else {
                        format!("Other(class={}, type={})", v.class_name(), v.value_type())
                    };
                    println!("  {} => {}", k, val_summary);

                    // Go one level deeper for nested objects if max_depth > 0
                    if max_depth > 0 && v.is_object() {
                        if let Some(inner) = v.as_object() {
                            for (ik, iv) in inner.iter().take(10) {
                                let is = if iv.is_integer() { format!("Int({})", iv.as_int().unwrap()) }
                                    else if iv.is_string() { format!("String({:?})", iv.as_str().unwrap()) }
                                    else if iv.is_null() { "Null".into() }
                                    else if iv.is_array() { format!("Array(len={})", iv.as_array().unwrap().len()) }
                                    else if iv.is_hash_map() { format!("HashMap(len={})", iv.as_hashmap().unwrap().len()) }
                                    else if iv.is_object() { format!("Object(class={})", iv.class_name()) }
                                    else if iv.is_bool() { format!("Bool({})", iv.as_bool().unwrap()) }
                                    else { format!("?(type={})", iv.value_type()) };
                                println!("    {}.{} => {}", k, ik, is);
                            }
                        }
                    }
                }
            } else if let Some(hm) = val.as_hashmap() {
                println!("  (HashMap with {} entries)", hm.len());
                for (k, v) in hm.iter().take(10) {
                    let ks = if let Some(s) = k.as_str() { s.to_string() }
                             else if k.is_integer() { format!("{}", k.as_int().unwrap()) }
                             else { format!("{:?}", k) };
                    let vs = if v.is_integer() { format!("Int({})", v.as_int().unwrap()) }
                             else if v.is_null() { "nil".into() }
                             else if v.is_object() { format!("Object(class={})", v.class_name()) }
                             else if v.is_string() { format!("String({:?})", v.as_str().unwrap()) }
                             else { format!("?(type={})", v.value_type()) };
                    println!("  {}=>{}", ks, vs);
                }
            } else if let Some(arr) = val.as_array() {
                println!("  (Array with {} entries)", arr.len());
                for (i, v) in arr.iter().take(5).enumerate() {
                    let vs = if v.is_integer() { format!("Int({})", v.as_int().unwrap()) }
                             else if v.is_null() { "nil".into() }
                             else if v.is_object() { format!("Object(class={})", v.class_name()) }
                             else if v.is_string() { format!("String({:?})", v.as_str().unwrap()) }
                             else if v.is_bool() { format!("Bool({})", v.as_bool().unwrap()) }
                             else { format!("?(type={})", v.value_type()) };
                    println!("  [{}] => {}", i, vs);
                }
            }
        };

        // Try to find key game data objects
        // Attempt HashMap access with symbol keys
        let sym_actors = Value::symbol("actors");
        let sym_party = Value::symbol("party");
        let sym_variables = Value::symbol("variables");
        let sym_switches = Value::symbol("switches");
        let sym_system = Value::symbol("system");

        let try_hash_key = |sym: &Value, name: &str| -> Option<&Value> {
            let result: Option<&Value> = Get::<&Value>::get(&contents, sym);
            if result.is_some() {
                println!("\nFound :{} via HashMap symbol key", name);
            }
            result
        };

        if let Some(actors) = try_hash_key(&sym_actors, "actors") {
            explore_object("ACTORS", actors, 0);
            // Dive into the first non-nil actor - show ALL ivars deeply
            if let Some(data) = actors.as_object() {
                if let Some(data_val) = data.get("@data") {
                    if let Some(arr) = data_val.as_array() {
                        println!("\n=== ACTORS @data array (len={}) ===", arr.len());
                        for (i, actor) in arr.iter().enumerate() {
                            if !actor.is_null() {
                                explore_object(&format!("ACTOR @data[{}]", i), actor, 1);
                            }
                        }
                    }
                }
            }
        }

        if let Some(party) = try_hash_key(&sym_party, "party") {
            explore_object("PARTY", party, 1);
        }

        if let Some(variables) = try_hash_key(&sym_variables, "variables") {
            explore_object("VARIABLES", variables, 0);
            // Show @data contents
            if let Some(obj) = variables.as_object() {
                if let Some(data) = obj.get("@data") {
                    if let Some(arr) = data.as_array() {
                        println!("\n  VARIABLES @data (len={}):", arr.len());
                        for (i, v) in arr.iter().enumerate() {
                            if !v.is_null() && !(v.is_integer() && v.as_int() == Some(0)) {
                                let vs = if v.is_integer() { format!("{}", v.as_int().unwrap()) }
                                         else if v.is_string() { format!("{:?}", v.as_str().unwrap()) }
                                         else if v.is_bool() { format!("{}", v.as_bool().unwrap()) }
                                         else { format!("?(type={})", v.value_type()) };
                                println!("    [{}] = {}", i, vs);
                            }
                        }
                    }
                }
            }
        }

        if let Some(switches) = try_hash_key(&sym_switches, "switches") {
            explore_object("SWITCHES", switches, 0);
            // Show @data contents
            if let Some(obj) = switches.as_object() {
                if let Some(data) = obj.get("@data") {
                    if let Some(arr) = data.as_array() {
                        println!("\n  SWITCHES @data (len={}):", arr.len());
                        for (i, v) in arr.iter().enumerate() {
                            if !v.is_null() {
                                let vs = if v.is_bool() { format!("{}", v.as_bool().unwrap()) }
                                         else if v.is_integer() { format!("{}", v.as_int().unwrap()) }
                                         else { format!("?(type={})", v.value_type()) };
                                println!("    [{}] = {}", i, vs);
                            }
                        }
                    }
                }
            }
        }

        if let Some(system) = try_hash_key(&sym_system, "system") {
            explore_object("SYSTEM", system, 0);
        }

        // Also try Object ivar access with @-prefix
        let try_ivar = |key: &str| -> Option<&Value> {
            let result: Option<&Value> = Get::<&str>::get(&contents, key);
            if result.is_some() {
                println!("\nFound {} via Object ivar", key);
            }
            result
        };

        if let Some(actors) = try_ivar("@actors") {
            explore_object("@actors", actors, 0);
            if let Some(arr) = actors.as_array() {
                for (i, actor) in arr.iter().enumerate() {
                    if !actor.is_null() {
                        explore_object(&format!("@actors[{}]", i), actor, 1);
                        break;
                    }
                }
            }
        }

        if let Some(party) = try_ivar("@party") {
            explore_object("@party", party, 1);
        }

        if let Some(variables) = try_ivar("@variables") {
            explore_object("@variables", variables, 0);
        }

        if let Some(switches) = try_ivar("@switches") {
            explore_object("@switches", switches, 0);
        }

        // Also parse dump[0] (header) to see what it contains
        println!("\n\n========================================");
        println!("=== DUMP[0] (HEADER) ===");
        let header = load(dumps[0], None).expect("Failed to parse dump[0]");
        println!("class_name: {:?}", header.class_name());
        println!("is_object: {}", header.is_object());
        println!("is_hash_map: {}", header.is_hash_map());
        explore_object("HEADER", &header, 1);

        println!("\nTest complete - check output above for structure details");
    }
}
