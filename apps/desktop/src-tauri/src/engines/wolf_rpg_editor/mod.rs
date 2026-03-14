pub mod crypto;
pub mod reader;
pub mod vardb;

use crate::engines::types::*;
use crate::engines::EnginePlugin;
use chrono::Local;
use std::fs;
use std::path::Path;

pub struct WolfRpgEditorPlugin;

impl EnginePlugin for WolfRpgEditorPlugin {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            id: "wolf-rpg-editor".into(),
            name: "Wolf RPG Editor".into(),
            icon: "wolf-rpg-editor".into(),
            supports_debug: true,
            save_extensions: vec!["sav".into()],
            description: "Wolf RPG Editor game saves".into(),
            save_dir_hint: Some(
                "Select the game folder containing your .sav save files.\n\
                 Save files are usually in the game's root directory."
                    .to_string(),
            ),
            pick_mode: "folder".into(),
        }
    }

    fn detect(&self, game_dir: &Path) -> bool {
        // Must have Data/*.wolf files AND must NOT be an RPG Maker game
        let has_wolf = game_dir
            .join("Data")
            .read_dir()
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "wolf-rpg-editor")
                })
            })
            .unwrap_or(false);

        let not_rpgmaker = !game_dir.join("Data").join("Actors.rvdata2").exists()
            && !game_dir.join("www").join("js").exists();

        has_wolf && not_rpgmaker
    }

    fn list_saves(&self, game_dir: &Path) -> Result<Vec<SaveFile>, String> {
        let save_dir = game_dir.join("Save");
        if !save_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut saves = Vec::new();
        let entries = save_dir
            .read_dir()
            .map_err(|e| format!("Failed to read Save directory: {}", e))?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Only include SaveData*.sav files, skip System.sav and others
            if !name.starts_with("SaveData") || !name.ends_with(".sav") {
                continue;
            }

            let meta = fs::metadata(&path).map_err(|e| format!("metadata error: {}", e))?;
            let modified = meta
                .modified()
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();

            saves.push(SaveFile {
                path: path.to_string_lossy().to_string(),
                name,
                modified,
                size: meta.len(),
            });
        }

        saves.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(saves)
    }

    fn parse_save(&self, save_path: &Path, _game_dir: &Path) -> Result<SaveData, String> {
        let mut buf = fs::read(save_path).map_err(|e| format!("Failed to read: {e}"))?;
        crypto::decrypt(&mut buf);

        let mut walker = reader::FileWalker::new(&buf);

        // Header
        walker.skip(20)?; // 20-byte header
        let marker = walker.read_u8()?;
        if marker != 0x19 {
            return Err(format!(
                "Invalid save: expected 0x19 marker, got 0x{:02X}",
                marker
            ));
        }
        walker.skip_memdata_u16()?; // game name
        let file_version = walker.read_u16_le()?;
        walker.set_file_version(file_version);

        // Skip SaveParts 1-5
        reader::skip_save_part_1(&mut walker)?;
        reader::skip_save_part_2(&mut walker)?;
        reader::skip_save_part_3(&mut walker)?;
        reader::skip_save_part_4(&mut walker)?;
        reader::skip_save_part_5(&mut walker)?;

        // Parse VariableDatabase
        let vardb = vardb::VariableDatabase::parse(&mut walker)?;

        // Convert to Variables for the UI
        let variables = vardb_to_variables(&vardb);

        // Raw view
        let raw = serde_json::json!({
            "file_version": file_version,
            "vardb_types": vardb.types.len(),
            "total_entries": vardb.types.iter().map(|t| t.entries.len()).sum::<usize>(),
        });

        Ok(SaveData {
            raw,
            party: None,
            inventory: None,
            currency: None,
            variables: Some(variables),
            switches: None,
            custom_sections: Vec::new(),
        })
    }

    fn write_save(&self, save_path: &Path, data: &SaveData) -> Result<(), String> {
        let mut buf = fs::read(save_path).map_err(|e| format!("Failed to read: {e}"))?;
        crypto::decrypt(&mut buf);

        let mut walker = reader::FileWalker::new(&buf);

        // Skip header
        walker.skip(20)?;
        let marker = walker.read_u8()?;
        if marker != 0x19 {
            return Err(format!(
                "Invalid save: expected 0x19 marker, got 0x{:02X}",
                marker
            ));
        }
        walker.skip_memdata_u16()?; // game name
        let ver = walker.read_u16_le()?;
        walker.set_file_version(ver);

        // Skip SaveParts 1-5
        reader::skip_save_part_1(&mut walker)?;
        reader::skip_save_part_2(&mut walker)?;
        reader::skip_save_part_3(&mut walker)?;
        reader::skip_save_part_4(&mut walker)?;
        reader::skip_save_part_5(&mut walker)?;

        let vardb_start = walker.pos();
        let mut vardb = vardb::VariableDatabase::parse(&mut walker)?;
        let vardb_end = walker.pos();

        // Apply edits from SaveData.variables back into the VarDB structs
        if let Some(ref variables) = data.variables {
            for var in variables {
                let type_idx = (var.id / 100000) as usize;
                let entry_idx = ((var.id % 100000) / 100) as usize;
                let field_idx = (var.id % 100) as usize;

                if let Some(vtype) = vardb.types.get_mut(type_idx) {
                    if let Some(entry) = vtype.entries.get_mut(entry_idx) {
                        if let Some(field) = entry.fields.get_mut(field_idx) {
                            match field {
                                vardb::VarField::Int(ref mut n) => {
                                    *n = var.value.as_i64().unwrap_or(0) as i32;
                                }
                                vardb::VarField::Str(ref mut bytes) => {
                                    if let Some(s) = var.value.as_str() {
                                        let mut new_bytes = s.as_bytes().to_vec();
                                        new_bytes.push(0); // null terminator
                                        *bytes = new_bytes;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Serialize modified VarDB and splice into the buffer
        let new_vardb = vardb.to_bytes();

        let mut out = Vec::with_capacity(buf.len());
        out.extend_from_slice(&buf[..vardb_start]);
        out.extend_from_slice(&new_vardb);
        out.extend_from_slice(&buf[vardb_end..]);

        // Recompute checksum (stored at byte 0x02, computed over data region)
        out[0x02] = crypto::checksum(&out);

        // Re-encrypt
        crypto::encrypt(&mut out);

        fs::write(save_path, &out).map_err(|e| format!("Failed to write: {e}"))?;
        Ok(())
    }

    fn resolve_names(&self, _game_dir: &Path) -> Result<NameMap, String> {
        Ok(NameMap::default())
    }

    fn supports_debug_patch(&self) -> bool {
        true
    }

    fn apply_debug_patch(&self, game_dir: &Path) -> Result<PatchInfo, String> {
        // Find Game.exe or GamePro.exe (some Wolf RPG Editor games rename the exe)
        let game_exe_name = if game_dir.join("Game.exe").exists() {
            "Game.exe"
        } else if game_dir.join("GamePro.exe").exists() {
            "GamePro.exe"
        } else {
            "Game.exe"
        };

        if cfg!(target_os = "windows") {
            let lnk_path = game_dir.join("Debug Mode.lnk");
            let game_exe = game_dir.join(game_exe_name);

            let ps_script = format!(
                "$ws = New-Object -ComObject WScript.Shell; \
                 $s = $ws.CreateShortcut('{}'); \
                 $s.TargetPath = '{}'; \
                 $s.Arguments = 'Test_Of_Main Use_Debug_Window'; \
                 $s.WorkingDirectory = '{}'; \
                 $s.Description = 'Wolf RPG Editor debug mode (F9=variables, F3=debug window, F10=pause)'; \
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
                engine: "wolf-rpg-editor".into(),
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

            let script = format!(
                "#!/bin/bash\n\
                 cd \"$(dirname \"$0\")\"\n\
                 if ! command -v wine &>/dev/null; then\n\
                     echo \"Error: wine not found. Install Wine or adjust PATH.\" >&2\n\
                     exit 1\n\
                 fi\n\
                 wine {game_exe_name} Test_Of_Main Use_Debug_Window\n"
            );

            fs::write(&sh_path, &script)
                .map_err(|e| format!("Failed to write debug script: {e}"))?;

            // Make script executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&sh_path, fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("Failed to set script permissions: {e}"))?;
            }

            Ok(PatchInfo {
                engine: "wolf-rpg-editor".into(),
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

fn vardb_to_variables(vardb: &vardb::VariableDatabase) -> Vec<Variable> {
    let mut variables = Vec::new();
    for (ti, vtype) in vardb.types.iter().enumerate() {
        let group = format!("Type {}", ti);
        for (ei, entry) in vtype.entries.iter().enumerate() {
            for (fi, field) in entry.fields.iter().enumerate() {
                let id = (ti as u32) * 100000 + (ei as u32) * 100 + (fi as u32);
                let value = match field {
                    vardb::VarField::Int(n) => serde_json::json!(*n),
                    vardb::VarField::Str(bytes) => {
                        let s = String::from_utf8_lossy(
                            bytes.strip_suffix(&[0]).unwrap_or(bytes),
                        );
                        serde_json::json!(s)
                    }
                };
                // Skip zero/empty/default values to reduce noise
                let is_default = match field {
                    vardb::VarField::Int(0) => true,
                    vardb::VarField::Str(b) => b.is_empty() || b == &[0],
                    _ => false,
                };
                if !is_default {
                    variables.push(Variable {
                        id,
                        name: None,
                        value,
                        group: Some(group.clone()),
                    });
                }
            }
        }
    }
    variables
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // requires real save file on disk
    fn real_save_parse_save() {
        let save_path = std::path::Path::new(
            r"D:\Personalisation\Avatar\saves\Lilia The Fallen Flower in the Prison City-v1.03\Save\SaveData01.sav",
        );
        let game_dir = std::path::Path::new(
            r"D:\Personalisation\Avatar\saves\Lilia The Fallen Flower in the Prison City-v1.03",
        );

        let plugin = WolfRpgEditorPlugin;
        let save_data = plugin.parse_save(save_path, game_dir).unwrap();

        println!("Raw: {}", serde_json::to_string_pretty(&save_data.raw).unwrap());

        let vars = save_data.variables.as_ref().unwrap();
        println!("Total non-default variables: {}", vars.len());

        // Print first 10 variables as samples
        for v in vars.iter().take(10) {
            println!(
                "  id={}, group={:?}, name={:?}, value={}",
                v.id, v.group, v.name, v.value
            );
        }

        assert!(!vars.is_empty(), "Expected at least some non-default variables");
    }

    #[test]
    #[ignore] // requires real save file on disk
    fn real_save_write_roundtrip() {
        let save_path = std::path::Path::new(
            r"D:\Personalisation\Avatar\saves\Lilia The Fallen Flower in the Prison City-v1.03\Save\SaveData01.sav",
        );
        let game_dir = std::path::Path::new(
            r"D:\Personalisation\Avatar\saves\Lilia The Fallen Flower in the Prison City-v1.03",
        );

        // Copy the save to a temp file so we don't modify the original
        let tmp_path = std::env::temp_dir().join("wolf_write_test.sav");
        std::fs::copy(save_path, &tmp_path).expect("Failed to copy save to temp");

        let plugin = WolfRpgEditorPlugin;

        // 1. Parse original
        let original_data = plugin.parse_save(&tmp_path, game_dir).unwrap();
        let original_vars = original_data.variables.as_ref().unwrap();
        assert!(!original_vars.is_empty(), "Need at least one variable to test");

        // Pick the first variable and modify its value
        let target = &original_vars[0];
        let target_id = target.id;
        let original_value = target.value.clone();

        println!(
            "Target variable: id={}, value={}",
            target_id, original_value
        );

        // Use a distinctive non-zero/non-empty value so it survives the
        // default-value filter on re-parse
        let new_value = match original_value {
            serde_json::Value::Number(_) => serde_json::json!(12345),
            serde_json::Value::String(_) => serde_json::json!("TEST_WRITE"),
            _ => serde_json::json!(12345),
        };

        // 2. Build modified SaveData with just the one changed variable
        let modified_vars = vec![Variable {
            id: target_id,
            name: None,
            value: new_value.clone(),
            group: None,
        }];
        let modified_data = SaveData {
            raw: original_data.raw.clone(),
            party: None,
            inventory: None,
            currency: None,
            variables: Some(modified_vars),
            switches: None,
            custom_sections: Vec::new(),
        };

        // 3. Write
        plugin.write_save(&tmp_path, &modified_data).unwrap();

        // 4. Re-parse and verify the change persisted
        let reloaded = plugin.parse_save(&tmp_path, game_dir).unwrap();
        let reloaded_vars = reloaded.variables.as_ref().unwrap();
        let found = reloaded_vars.iter().find(|v| v.id == target_id);
        assert!(found.is_some(), "Modified variable id={} not found after reload", target_id);
        assert_eq!(
            found.unwrap().value, new_value,
            "Variable id={} value mismatch after write+reload",
            target_id
        );

        // 5. Also verify the decrypted file has correct checksum
        let mut raw = std::fs::read(&tmp_path).unwrap();
        super::crypto::decrypt(&mut raw);
        let cs = super::crypto::checksum(&raw);
        assert_eq!(cs, raw[0x02], "Checksum mismatch after write");

        // 6. Cleanup
        let _ = std::fs::remove_file(&tmp_path);

        println!(
            "write_roundtrip OK: id={}, original={}, new={}",
            target_id, original_value, new_value
        );
    }
}
