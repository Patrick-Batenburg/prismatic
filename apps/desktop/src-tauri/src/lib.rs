mod backup;
mod commands;
mod engines;
mod watcher;

use commands::AppState;
use engines::EngineRegistry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use watcher::FileWatcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut registry = EngineRegistry::new();
    registry.register(Box::new(engines::rpg_maker_mv::RpgMakerMvPlugin));
    registry.register(Box::new(engines::pixel_game_maker_mv::PgmmvPlugin));
    registry.register(Box::new(engines::renpy::RenpyPlugin));
    registry.register(Box::new(engines::rpg_maker_vx_ace::RpgMakerVxaPlugin));
    registry.register(Box::new(engines::wolf_rpg_editor::WolfRpgEditorPlugin));
    registry.register(Box::new(engines::flash::FlashSolPlugin));
    registry.register(Box::new(engines::unreal_engine::UnrealPlugin));
    registry.register(Box::new(engines::sugarcube::SugarCubePlugin));
    registry.register(Box::new(engines::sqlite::SqlitePlugin));
    registry.register(Box::new(engines::unity::UnityPlugin));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            registry,
            current_engine: Mutex::new(None),
            current_game_dir: Mutex::new(None),
            last_loaded_save: Mutex::new(None),
            scan_cache: Mutex::new(HashMap::new()),
        })
        .manage(Arc::new(Mutex::new(FileWatcher::new())) as watcher::SharedWatcher)
        .invoke_handler(tauri::generate_handler![
            commands::list_engines,
            commands::detect_engine,
            commands::set_game,
            commands::list_saves,
            commands::load_save,
            commands::save_file,
            commands::compare_save,
            commands::get_names,
            commands::get_diff,
            commands::list_backups,
            commands::restore_backup,
            commands::browse_save_dir,
            commands::deep_scan_dir,
            commands::apply_debug_patch,
            commands::revert_debug_patch,
            commands::query_table,
            commands::update_rows,
            commands::insert_row,
            commands::delete_rows,
            watcher::watch_save,
            watcher::unwatch_save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
