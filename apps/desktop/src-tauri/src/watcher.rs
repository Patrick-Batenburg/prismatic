use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    watched_path: Option<PathBuf>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watcher: None,
            watched_path: None,
        }
    }

    pub fn watch(&mut self, path: PathBuf, app: AppHandle) -> Result<(), String> {
        // Stop any existing watcher
        self.unwatch();

        let watch_path = path.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    use notify::EventKind::*;
                    match event.kind {
                        Modify(_) | Create(_) => {
                            let _ = app.emit("save-file-changed", &watch_path.to_string_lossy().to_string());
                        }
                        _ => {}
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        // Watch the parent directory (to catch file replacements)
        let watch_dir = path.parent().ok_or("No parent directory")?;
        watcher
            .watch(watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch directory: {e}"))?;

        self.watcher = Some(watcher);
        self.watched_path = Some(path);
        Ok(())
    }

    pub fn unwatch(&mut self) {
        self.watcher = None;
        self.watched_path = None;
    }
}

pub type SharedWatcher = Arc<Mutex<FileWatcher>>;

#[tauri::command]
pub fn watch_save(
    app: AppHandle,
    watcher: tauri::State<SharedWatcher>,
    save_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(&save_path);
    watcher.lock().unwrap().watch(path, app)
}

#[tauri::command]
pub fn unwatch_save(watcher: tauri::State<SharedWatcher>) -> Result<(), String> {
    watcher.lock().unwrap().unwatch();
    Ok(())
}
