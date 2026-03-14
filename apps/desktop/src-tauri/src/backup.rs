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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// Creates a unique temp directory for a test and returns its path.
    /// The caller is responsible for removing it when done.
    fn make_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("backup_test_{}_{}", name, std::process::id()));
        fs::create_dir_all(&dir).expect("failed to create test dir");
        dir
    }

    /// Writes `content` to `path`, creating parent directories as needed.
    fn write_file(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        fs::write(path, content).expect("failed to write file");
    }

    // -------------------------------------------------------------------------
    // 1. create_backup — creates a backup file in the backup directory
    // -------------------------------------------------------------------------
    #[test]
    fn test_create_backup_creates_file() {
        let dir = make_test_dir("create_file");
        let save_path = dir.join("save.dat");
        write_file(&save_path, b"save data");

        let backup_path = BackupManager::create_backup(&save_path)
            .expect("create_backup should succeed");

        assert!(backup_path.exists(), "backup file should exist on disk");

        let backup_dir = dir.join("save_backups");
        assert!(
            backup_path.starts_with(&backup_dir),
            "backup should be inside the save_backups subdirectory"
        );

        let backup_name = backup_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            backup_name.starts_with("save.dat_"),
            "backup name should be prefixed with the original filename"
        );

        let contents = fs::read(&backup_path).expect("should be able to read backup");
        assert_eq!(contents, b"save data", "backup contents should match original");

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 2. create_backup creates directory — backup dir is created if it doesn't exist
    // -------------------------------------------------------------------------
    #[test]
    fn test_create_backup_creates_backup_dir() {
        let dir = make_test_dir("create_dir");
        let save_path = dir.join("save.dat");
        write_file(&save_path, b"hello");

        let backup_dir = dir.join("save_backups");
        assert!(!backup_dir.exists(), "backup dir should not exist yet");

        BackupManager::create_backup(&save_path).expect("create_backup should succeed");

        assert!(backup_dir.exists(), "backup dir should have been created");
        assert!(backup_dir.is_dir(), "backup dir should be a directory");

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 3. list_backups — returns backups sorted by date (newest first)
    // -------------------------------------------------------------------------
    #[test]
    fn test_list_backups_sorted_newest_first() {
        let dir = make_test_dir("list_sorted");
        let backup_dir = dir.join("save_backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create three backup files with artificially separated modified times
        // so that filesystem resolution is not an issue.
        let names = ["save.dat_20240101_120000", "save.dat_20240102_120000", "save.dat_20240103_120000"];
        let base_time = SystemTime::now() - Duration::from_secs(60);

        for (i, name) in names.iter().enumerate() {
            let path = backup_dir.join(name);
            fs::write(&path, b"data").unwrap();
            // Stagger modified times by 10 seconds each
            let mtime = base_time + Duration::from_secs(i as u64 * 10);
            filetime_set(&path, mtime);
        }

        let save_path = dir.join("save.dat");
        write_file(&save_path, b"current");

        let entries = BackupManager::list_backups(&save_path)
            .expect("list_backups should succeed");

        assert_eq!(entries.len(), 3, "should find all 3 backup files");

        // modified is rfc3339; sorted newest first means descending order
        for window in entries.windows(2) {
            assert!(
                window[0].modified >= window[1].modified,
                "entries should be sorted newest first: {} < {}",
                window[0].modified,
                window[1].modified
            );
        }

        // The most recent entry should correspond to the last-created file
        assert!(
            entries[0].name.contains("20240103"),
            "newest entry name should contain 20240103, got: {}",
            entries[0].name
        );

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 4. list_backups empty dir — returns empty vec when no backups exist
    // -------------------------------------------------------------------------
    #[test]
    fn test_list_backups_empty_when_no_backups() {
        let dir = make_test_dir("list_empty");
        let save_path = dir.join("save.dat");
        // No save file and no backup dir needed — list_backups checks existence.

        let entries = BackupManager::list_backups(&save_path)
            .expect("list_backups should succeed even when backup dir is absent");

        assert!(entries.is_empty(), "should return empty vec when no backups exist");

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 5. restore_backup — restores a backup file to the original path
    // -------------------------------------------------------------------------
    #[test]
    fn test_restore_backup_overwrites_save() {
        let dir = make_test_dir("restore");
        let save_path = dir.join("save.dat");
        let backup_dir = dir.join("save_backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create a "current" save and a backup with different content.
        write_file(&save_path, b"current save");
        let backup_path = backup_dir.join("save.dat_20240101_000000");
        write_file(&backup_path, b"old save data");

        BackupManager::restore_backup(&backup_path, &save_path)
            .expect("restore_backup should succeed");

        let restored = fs::read(&save_path).expect("save file should exist after restore");
        assert_eq!(restored, b"old save data", "save file should contain the backup's content");

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 6. prune_old_backups — keeps only MAX_BACKUPS most recent files
    // -------------------------------------------------------------------------
    #[test]
    fn test_prune_old_backups_removes_excess() {
        let dir = make_test_dir("prune_excess");
        let backup_dir = dir.join("save_backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create MAX_BACKUPS + 3 backup files, staggering modified times.
        let total = MAX_BACKUPS + 3;
        let base_time = SystemTime::now() - Duration::from_secs(total as u64 * 10);

        for i in 0..total {
            let name = format!("save.dat_backup_{:03}", i);
            let path = backup_dir.join(&name);
            fs::write(&path, b"x").unwrap();
            let mtime = base_time + Duration::from_secs(i as u64 * 10);
            filetime_set(&path, mtime);
        }

        // The test module is inside backup.rs so it can call the private method directly.
        BackupManager::prune_old_backups(&backup_dir, "save.dat")
            .expect("prune should succeed");

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(
            remaining.len(),
            MAX_BACKUPS,
            "prune should leave exactly MAX_BACKUPS ({}) files, found {}",
            MAX_BACKUPS,
            remaining.len()
        );

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // 7. prune with fewer than max — doesn't delete anything when under limit
    // -------------------------------------------------------------------------
    #[test]
    fn test_prune_does_not_delete_when_under_limit() {
        let dir = make_test_dir("prune_under");
        let backup_dir = dir.join("save_backups");
        fs::create_dir_all(&backup_dir).unwrap();

        let count = MAX_BACKUPS - 2;
        for i in 0..count {
            let name = format!("save.dat_backup_{:03}", i);
            let path = backup_dir.join(&name);
            fs::write(&path, b"x").unwrap();
        }

        BackupManager::prune_old_backups(&backup_dir, "save.dat")
            .expect("prune should succeed");

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(
            remaining.len(),
            count,
            "prune should not delete files when count ({}) is below MAX_BACKUPS ({})",
            count,
            MAX_BACKUPS
        );

        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------------------
    // Helper: set file modification time via std (platform-agnostic enough for tests)
    // -------------------------------------------------------------------------
    fn filetime_set(path: &Path, time: SystemTime) {
        // We use std::fs::File + set_modified where available.
        // On Windows this requires the `set_modified` stabilised in Rust 1.75.
        // Fall back gracefully: if it errors, we skip the ordering guarantee.
        let _ = fs::File::options().write(true).open(path).and_then(|f| {
            f.set_modified(time)
        });
    }
}
