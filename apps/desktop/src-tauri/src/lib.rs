mod backup;
mod commands;
mod engines;

use commands::AppState;
use engines::EngineRegistry;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut registry = EngineRegistry::new();
    registry.register(Box::new(engines::rpg_maker_mv::RpgMakerMvPlugin));
    registry.register(Box::new(engines::pixel_game_maker_mv::PgmmvPlugin));

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
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_engines,
            commands::detect_engine,
            commands::set_game,
            commands::list_saves,
            commands::load_save,
            commands::save_file,
            commands::get_names,
            commands::get_diff,
            commands::list_backups,
            commands::restore_backup,
            commands::apply_debug_patch,
            commands::revert_debug_patch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
