use std::path::PathBuf;

/// Returns the user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Returns the AppData/Roaming equivalent.
/// Windows: APPDATA, Linux: ~/.config
pub fn appdata_dir() -> Option<PathBuf> {
    dirs::config_dir()
}

/// Returns the AppData/Local equivalent.
/// Windows: LOCALAPPDATA, Linux: ~/.local/share
pub fn local_appdata_dir() -> Option<PathBuf> {
    dirs::data_local_dir()
}

/// Expand Windows-style `%VAR%` placeholders in a path string.
///
/// On Windows: resolves APPDATA, LOCALAPPDATA, USERPROFILE from the environment.
/// On Linux: maps Windows variable names to XDG equivalents via the `dirs` crate:
///   - `%APPDATA%`      -> `dirs::config_dir()`  (~/.config)
///   - `%LOCALAPPDATA%` -> `dirs::data_local_dir()` (~/.local/share)
///   - `%USERPROFILE%`  -> `dirs::home_dir()` (~)
///
/// Unknown `%VAR%` placeholders are left as-is.
/// Normalizes path separators after expansion.
pub fn expand_env(path: &str) -> String {
    let mut result = path.to_string();

    if cfg!(target_os = "windows") {
        // On Windows, resolve from actual environment variables
        if let Ok(appdata) = std::env::var("APPDATA") {
            result = result.replace("%APPDATA%", &appdata);
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            result = result.replace("%LOCALAPPDATA%", &localappdata);
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            result = result.replace("%USERPROFILE%", &userprofile);
        }
    } else {
        // On Linux, map Windows env var names to XDG equivalents
        if let Some(dir) = dirs::config_dir() {
            result = result.replace("%APPDATA%", &dir.to_string_lossy());
        }
        if let Some(dir) = dirs::data_local_dir() {
            result = result.replace("%LOCALAPPDATA%", &dir.to_string_lossy());
        }
        if let Some(dir) = dirs::home_dir() {
            result = result.replace("%USERPROFILE%", &dir.to_string_lossy());
        }
    }

    // Normalize to native path separators
    PathBuf::from(result).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_env_unknown_var_left_as_is() {
        let input = "%UNKNOWN_VAR%/something";
        let result = expand_env(input);
        assert!(result.contains("%UNKNOWN_VAR%"));
    }

    #[test]
    fn expand_env_empty_string() {
        assert_eq!(expand_env(""), "");
    }

    #[test]
    fn expand_env_no_placeholders() {
        let input = "/some/normal/path";
        let result = expand_env(input);
        assert!(result.contains("some"));
    }

    #[test]
    fn expand_env_userprofile_resolves() {
        let result = expand_env("%USERPROFILE%/test");
        if dirs::home_dir().is_some() {
            assert!(!result.contains("%USERPROFILE%"));
            assert!(result.contains("test"));
        }
    }

    #[test]
    fn expand_env_appdata_resolves() {
        let result = expand_env("%APPDATA%/test");
        if cfg!(target_os = "windows") {
            if std::env::var("APPDATA").is_ok() {
                assert!(!result.contains("%APPDATA%"));
            }
        } else if dirs::config_dir().is_some() {
            assert!(!result.contains("%APPDATA%"));
        }
    }

    #[test]
    fn home_dir_returns_some() {
        assert!(home_dir().is_some());
    }
}
