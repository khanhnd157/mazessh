use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::models::security::AuditEntry;

fn data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".maze-ssh"))
}

fn audit_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("audit.log"))
}

/// Maximum audit log size before rotation (1 MB)
const MAX_LOG_SIZE: u64 = 1_048_576;

pub fn append_log(entry: &AuditEntry) {
    let (Some(dir), Some(path)) = (data_dir(), audit_path()) else {
        return; // silently skip if home directory is unavailable
    };

    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }

    // Rotate if log exceeds max size
    if let Ok(metadata) = fs::metadata(&path) {
        if metadata.len() > MAX_LOG_SIZE {
            let _ = rotate_log(&path);
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        // Restrict audit log to owner-only on Unix (prevent other local users from reading it)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
        if let Ok(line) = serde_json::to_string(entry) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

/// Rotate the audit log: rename current to .1, discard older rotations
fn rotate_log(path: &std::path::Path) -> Result<(), std::io::Error> {
    let rotated = path.with_extension("log.1");
    // Remove previous rotation if exists
    if rotated.exists() {
        fs::remove_file(&rotated)?;
    }
    fs::rename(path, &rotated)?;
    Ok(())
}

/// Convenience wrapper for vault/key operations
pub fn log_action(action: &str, key_name: Option<&str>, result: &str) {
    append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: action.to_string(),
        profile_name: key_name.map(String::from),
        result: result.to_string(),
        ..Default::default()
    });
}

pub fn read_logs(
    limit: usize,
    offset: usize,
    action_filter: Option<&str>,
) -> Vec<AuditEntry> {
    let path = match audit_path() {
        Some(p) => p,
        None => return Vec::new(),
    };
    if !path.exists() {
        return Vec::new();
    }

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut entries: Vec<AuditEntry> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str::<AuditEntry>(&line).ok())
        .filter(|entry| {
            action_filter
                .map(|f| entry.action == f || entry.action.starts_with(f))
                .unwrap_or(true)
        })
        .collect();

    // Newest first
    entries.reverse();

    entries.into_iter().skip(offset).take(limit).collect()
}
