use crate::engines::types::*;
use crate::engines::EnginePlugin;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;

const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";
const SAVE_EXTENSIONS: &[&str] = &["db", "sqlite", "sqlite3"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    pub name: String,
    pub columns: Vec<ColumnMeta>,
    pub row_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub col_type: String,
}

pub struct SqlitePlugin;

impl SqlitePlugin {
    fn is_sqlite_file(path: &Path) -> bool {
        let Ok(mut file) = fs::File::open(path) else {
            return false;
        };
        let mut buf = [0u8; 16];
        if file.read_exact(&mut buf).is_err() {
            return false;
        }
        buf == SQLITE_MAGIC
    }

    fn has_matching_extension(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| SAVE_EXTENSIONS.contains(&e))
            .unwrap_or(false)
    }

    fn is_save_file(path: &Path) -> bool {
        Self::has_matching_extension(path) && Self::is_sqlite_file(path)
    }

    fn get_columns(conn: &Connection, table_name: &str) -> Result<Vec<ColumnMeta>, String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{}\")", table_name))
            .map_err(|e| format!("PRAGMA table_info failed for {table_name}: {e}"))?;

        let cols = stmt
            .query_map([], |row| {
                Ok(ColumnMeta {
                    name: row.get::<_, String>(1)?,
                    col_type: row.get::<_, String>(2)?,
                })
            })
            .map_err(|e| format!("failed to query columns for {table_name}: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(cols)
    }

    fn get_table_metadata(conn: &Connection) -> Result<Vec<TableMeta>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(|e| format!("failed to query sqlite_master: {e}"))?;

        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("failed to read table names: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        let mut tables = Vec::new();
        for name in table_names {
            let columns = Self::get_columns(conn, &name)?;

            let row_count: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM \"{}\"", name),
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| format!("failed to count rows in {name}: {e}"))?;

            tables.push(TableMeta {
                name,
                columns,
                row_count,
            });
        }

        Ok(tables)
    }
}

impl EnginePlugin for SqlitePlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "sqlite".to_string(),
            name: "SQLite".to_string(),
            icon: "sqlite".to_string(),
            supports_debug: false,
            save_extensions: SAVE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            description: "Generic SQLite database save files".to_string(),
            save_dir_hint: Some(
                "Select the folder containing your SQLite save database.\n\
                 Common extensions: .db, .sqlite, .sqlite3, .save"
                    .to_string(),
            ),
            pick_mode: "folder".to_string(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        let Ok(entries) = fs::read_dir(game_dir) else {
            return false;
        };
        entries
            .flatten()
            .any(|entry| Self::is_save_file(&entry.path()))
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let entries = fs::read_dir(game_dir)
            .map_err(|e| format!("failed to read dir {}: {e}", game_dir.display()))?;

        let mut saves = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if Self::is_save_file(&path) {
                let meta = fs::metadata(&path).ok();
                saves.push(SaveFile {
                    path: path.to_string_lossy().to_string(),
                    name: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    modified: meta
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(crate::engines::utils::format_modified_time)
                        .unwrap_or_default(),
                    size: meta.map(|m| m.len()).unwrap_or(0),
                });
            }
        }
        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, _game_dir: &Path) -> Result<SaveData, String> {
        let conn = Connection::open(save_path)
            .map_err(|e| format!("failed to open SQLite db {}: {e}", save_path.display()))?;

        let tables = Self::get_table_metadata(&conn)?;
        let raw = serde_json::to_value(&tables)
            .map_err(|e| format!("failed to serialize table metadata: {e}"))?;

        Ok(SaveData {
            raw,
            party: None,
            inventory: None,
            currency: None,
            variables: None,
            switches: None,
            custom_sections: vec![],
        })
    }

    fn write_save(&self, _save_path: &Path, _data: &SaveData) -> Result<(), String> {
        // Writing is handled by separate update_rows command
        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }
}
