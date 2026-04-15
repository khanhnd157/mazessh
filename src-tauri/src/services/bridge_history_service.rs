/// Per-distro health history ring buffer.
///
/// Each bridged distro gets a JSONL file at `~/.maze-ssh/bridge-history-<distro>.json`.
/// Entries are appended on every bridge lifecycle event. The file is compacted to
/// MAX_HISTORY_ENTRIES on every write so it never grows unboundedly.
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::models::bridge::{BridgeHistoryEvent, BridgeHistoryEventKind};
use crate::services::profile_service::data_dir;

const MAX_HISTORY_ENTRIES: usize = 200;

fn history_path(distro: &str) -> Option<PathBuf> {
    // Sanitise distro name to a safe filename component.
    let safe = distro.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_");
    data_dir().ok().map(|d| d.join(format!("bridge-history-{safe}.json")))
}

/// Append one event and compact the file to MAX_HISTORY_ENTRIES if needed.
/// Silently drops if the data directory is unavailable.
pub fn append_event(distro: &str, event: BridgeHistoryEventKind, detail: Option<String>) {
    let Some(path) = history_path(distro) else {
        return;
    };

    let entry = BridgeHistoryEvent {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event,
        detail,
    };

    let mut entries = read_raw(&path);
    entries.push(entry);
    if entries.len() > MAX_HISTORY_ENTRIES {
        let overflow = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..overflow);
    }
    write_all(&path, &entries);
}

/// Return up to `limit` most-recent events, newest-first.
pub fn read_events(distro: &str, limit: usize) -> Vec<BridgeHistoryEvent> {
    let Some(path) = history_path(distro) else {
        return Vec::new();
    };
    let mut events = read_raw(&path);
    events.reverse();
    events.truncate(limit);
    events
}

// ── Internal helpers ──

fn read_raw(path: &PathBuf) -> Vec<BridgeHistoryEvent> {
    if !path.exists() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .filter_map(|l| serde_json::from_str::<BridgeHistoryEvent>(&l).ok())
        .collect()
}

fn write_all(path: &PathBuf, entries: &[BridgeHistoryEvent]) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    let Ok(mut file) = fs::File::create(&tmp) else {
        return;
    };
    for e in entries {
        if let Ok(line) = serde_json::to_string(e) {
            let _ = writeln!(file, "{line}");
        }
    }
    let _ = fs::rename(&tmp, path);
}
