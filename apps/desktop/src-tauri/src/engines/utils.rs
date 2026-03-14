use std::path::Path;
use std::time::SystemTime;

pub fn format_modified_time(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(time).to_rfc3339()
}

pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::SystemTime;

    // --- format_modified_time ---

    #[test]
    fn format_modified_time_rfc3339_format() {
        // RFC3339 strings contain a "T" separator between date and time
        // and a timezone offset (e.g. "+00:00" or "-05:00" or "Z").
        let result = format_modified_time(SystemTime::UNIX_EPOCH);
        assert!(result.contains('T'), "expected RFC3339 'T' separator, got: {result}");
        // A timezone offset contains ':' (e.g. "+00:00") or ends with 'Z'
        let has_tz = result.ends_with('Z') || result.contains('+') || {
            // negative offsets appear as "-HH:MM" at the tail
            let tail = &result[result.len().saturating_sub(6)..];
            tail.starts_with('-') && tail.contains(':')
        };
        assert!(has_tz, "expected timezone offset in RFC3339 output, got: {result}");
    }

    #[test]
    fn format_modified_time_current_time_non_empty() {
        let result = format_modified_time(SystemTime::now());
        assert!(!result.is_empty(), "formatted time should not be empty");
    }

    #[test]
    fn format_modified_time_unix_epoch_valid_date() {
        let result = format_modified_time(SystemTime::UNIX_EPOCH);
        // The epoch is 1970-01-01; the date portion must start with "1970-01-01"
        // (local time may shift the clock, but the string must be a valid date)
        assert!(!result.is_empty(), "epoch should produce a non-empty string");
        // Ensure it parses back as a valid RFC3339 timestamp
        chrono::DateTime::parse_from_rfc3339(&result)
            .expect("epoch string should be valid RFC3339");
    }

    // --- has_extension ---

    #[test]
    fn has_extension_matching() {
        assert!(has_extension(Path::new("file.save"), "save"));
    }

    #[test]
    fn has_extension_non_matching() {
        assert!(!has_extension(Path::new("file.save"), "txt"));
    }

    #[test]
    fn has_extension_case_insensitive() {
        assert!(has_extension(Path::new("file.SAVE"), "save"));
        assert!(has_extension(Path::new("file.save"), "SAVE"));
    }

    #[test]
    fn has_extension_no_extension() {
        assert!(!has_extension(Path::new("file"), "save"));
    }

    #[test]
    fn has_extension_empty_extension_arg() {
        // A file with no extension should not match an empty string query
        assert!(!has_extension(Path::new("file"), ""));
    }

    #[test]
    fn has_extension_multiple_dots() {
        assert!(has_extension(Path::new("file.backup.save"), "save"));
    }

    #[test]
    fn has_extension_hidden_file() {
        // Rust's Path::extension() treats dotfiles like ".gitignore" as having
        // no extension (the entire name is considered the stem). Verify that
        // has_extension correctly returns false in this case.
        assert!(!has_extension(Path::new(".gitignore"), "gitignore"));
    }
}
