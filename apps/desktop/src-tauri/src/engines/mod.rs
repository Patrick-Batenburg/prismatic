pub mod pixel_game_maker_mv;
pub mod renpy;
pub mod rpg_maker_mv;
pub mod types;

use std::path::Path;
use types::*;

pub trait EnginePlugin: Send + Sync {
    fn info(&self) -> EngineInfo;
    fn detect(&self, game_dir: &Path) -> bool;
    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String>;
    fn parse_save(&self, save_path: &Path, game_dir: &Path) -> Result<SaveData, String>;
    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String>;
    fn resolve_names(&self, game_dir: &Path) -> Result<NameMap, String>;

    fn supports_debug_patch(&self) -> bool {
        false
    }
    fn apply_debug_patch(&self, _game_dir: &Path) -> Result<PatchInfo, String> {
        Err("Debug patching not supported for this engine".into())
    }
    fn revert_debug_patch(&self, _game_dir: &Path, _patch: &PatchInfo) -> Result<(), String> {
        Err("Debug patching not supported for this engine".into())
    }
}

pub struct EngineRegistry {
    engines: Vec<Box<dyn EnginePlugin>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    pub fn register(&mut self, engine: Box<dyn EnginePlugin>) {
        self.engines.push(engine);
    }

    pub fn list_engines(&self) -> Vec<EngineInfo> {
        self.engines.iter().map(|e| e.info()).collect()
    }

    pub fn detect_engine(&self, game_dir: &Path) -> Option<&dyn EnginePlugin> {
        self.engines
            .iter()
            .find(|e| e.detect(game_dir))
            .map(|e| e.as_ref())
    }

    pub fn get_engine(&self, id: &str) -> Option<&dyn EnginePlugin> {
        self.engines
            .iter()
            .find(|e| e.info().id == id)
            .map(|e| e.as_ref())
    }
}
