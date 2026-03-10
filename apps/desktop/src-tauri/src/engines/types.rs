use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub path: String,
    pub name: String,
    pub modified: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub raw: serde_json::Value,
    pub party: Option<Vec<Character>>,
    pub inventory: Option<Inventory>,
    pub currency: Option<CurrencyInfo>,
    pub variables: Option<Vec<Variable>>,
    pub switches: Option<Vec<Switch>>,
    pub custom_sections: Vec<CustomSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: u32,
    pub name: String,
    pub class_name: Option<String>,
    pub level: u32,
    pub exp: u64,
    pub stats: Vec<Stat>,
    pub equipment: Vec<EquipSlot>,
    pub skills: Vec<NamedId>,
    pub states: Vec<NamedId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stat {
    pub key: String,
    pub label: String,
    pub current: f64,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipSlot {
    pub slot_name: String,
    pub item_id: Option<u32>,
    pub item_name: Option<String>,
    /// "weapon" or "armor" — determines which name list to use
    pub data_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub weapons: Vec<InventoryItem>,
    pub armors: Vec<InventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: u32,
    pub name: String,
    pub quantity: u32,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyInfo {
    pub label: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: u32,
    pub name: Option<String>,
    pub value: serde_json::Value,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Switch {
    pub id: u32,
    pub name: Option<String>,
    pub value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedId {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSection {
    pub key: String,
    pub label: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NameMap {
    pub actors: HashMap<u32, String>,
    pub classes: HashMap<u32, String>,
    pub items: HashMap<u32, String>,
    pub weapons: HashMap<u32, String>,
    pub armors: HashMap<u32, String>,
    pub skills: HashMap<u32, String>,
    pub variables: HashMap<u32, String>,
    pub switches: HashMap<u32, String>,
    pub custom: HashMap<String, HashMap<u32, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInfo {
    pub engine: String,
    pub game_dir: String,
    pub patches: Vec<PatchEntry>,
    pub applied_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEntry {
    pub file_path: String,
    pub action: PatchAction,
    pub original_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchAction {
    Created,
    Modified { original: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub supports_debug: bool,
    pub save_extensions: Vec<String>,
    pub description: String,
    /// If set, the frontend should prompt the user to select a separate save directory
    /// after detection, using this string as a hint message.
    pub save_dir_hint: Option<String>,
}
