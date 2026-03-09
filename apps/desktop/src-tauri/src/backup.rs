use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_BACKUPS: usize = 10;

pub struct BackupManager;

impl BackupManager {
    pub fn create_backup(save_path: &Path) -> Result<PathBuf, String> {
        if !save_path.exists() {
            return Err("Save file does not exist".into());
        }

        let backup_dir = Self::backup_dir_for(save_path)?;
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup dir: {e}"))?;

        let file_name = save_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_string_lossy();
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("{file_name}_{timestamp}");
        let backup_path = backup_dir.join(&backup_name);

        fs::copy(save_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {e}"))?;

        Self::prune_old_backups(&backup_dir, &file_name)?;

        Ok(backup_path)
    }

    pub fn list_backups(save_path: &Path) -> Result<Vec<BackupEntry>, String> {
        let backup_dir = Self::backup_dir_for(save_path)?;
        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let file_name = save_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_string_lossy()
            .to_string();

        let mut entries: Vec<BackupEntry> = fs::read_dir(&backup_dir)
            .map_err(|e| format!("Failed to read backup dir: {e}"))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&file_name) && name != file_name {
                    let meta = entry.metadata().ok()?;
                    Some(BackupEntry {
                        path: entry.path().to_string_lossy().to_string(),
                        name: name.clone(),
                        size: meta.len(),
                        modified: meta
                            .modified()
                            .ok()
                            .map(|t| chrono::DateTime::<Local>::from(t).to_rfc3339())
                            .unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(entries)
    }

    pub fn restore_backup(backup_path: &Path, save_path: &Path) -> Result<(), String> {
        if save_path.exists() {
            Self::create_backup(save_path)?;
        }
        fs::copy(backup_path, save_path)
            .map_err(|e| format!("Failed to restore backup: {e}"))?;
        Ok(())
    }

    fn backup_dir_for(save_path: &Path) -> Result<PathBuf, String> {
        let parent = save_path.parent().ok_or("No parent directory")?;
        Ok(parent.join("save_backups"))
    }

    fn prune_old_backups(backup_dir: &Path, file_prefix: &str) -> Result<(), String> {
        let mut backups: Vec<_> = fs::read_dir(backup_dir)
            .map_err(|e| format!("Failed to read backup dir: {e}"))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(file_prefix))
            .collect();

        backups.sort_by(|a, b| {
            let ma = a.metadata().and_then(|m| m.modified()).ok();
            let mb = b.metadata().and_then(|m| m.modified()).ok();
            mb.cmp(&ma)
        });

        for old in backups.into_iter().skip(MAX_BACKUPS) {
            let _ = fs::remove_file(old.path());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: String,
}
