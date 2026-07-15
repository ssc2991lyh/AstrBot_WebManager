use std::path::PathBuf;

/// Convert an archive entry path to a relative PathBuf, rejecting empty, traversal,
/// or control-character-containing paths.
pub(crate) fn parse_entry_rel_path(raw: &str) -> Option<PathBuf> {
    if raw.chars().any(|c| c.is_control()) {
        return None;
    }

    let normalized = raw.replace('\\', "/");
    let first = normalized.split('/').next()?;
    let bytes = normalized.as_bytes();
    let has_windows_drive_prefix =
        bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic();
    if first.is_empty() || has_windows_drive_prefix {
        return None;
    }

    let mut relative = PathBuf::new();

    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => relative.push(part),
        }
    }

    if relative.as_os_str().is_empty() {
        return None;
    }

    Some(relative)
}
