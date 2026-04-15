use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A persistent "always allow" policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub key_id: String,
    pub key_name: String,
    pub created_at: DateTime<Utc>,
}

/// In-memory session rules (cleared on lock/restart).
pub struct SessionRules {
    /// key_id → true (allowed for this session)
    pub allowed: Mutex<HashMap<String, bool>>,
}

impl SessionRules {
    pub fn new() -> Self {
        Self {
            allowed: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_allowed(&self, key_id: &str) -> bool {
        self.allowed
            .lock()
            .map(|m| m.get(key_id).copied().unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn allow(&self, key_id: &str) {
        if let Ok(mut m) = self.allowed.lock() {
            m.insert(key_id.to_string(), true);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut m) = self.allowed.lock() {
            m.clear();
        }
    }
}

const POLICY_FILE: &str = "policy-rules.json";

/// Load persistent "always allow" rules from disk.
pub fn load_rules(vault_dir: &Path) -> Vec<PolicyRule> {
    let path = vault_dir.join(POLICY_FILE);
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Save persistent "always allow" rules to disk.
pub fn save_rules(vault_dir: &Path, rules: &[PolicyRule]) -> Result<(), std::io::Error> {
    let path = vault_dir.join(POLICY_FILE);
    let content = serde_json::to_string_pretty(rules)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content)?;
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

/// Check if a key has an "always allow" persistent rule.
pub fn has_always_rule(vault_dir: &Path, key_id: &str) -> bool {
    load_rules(vault_dir).iter().any(|r| r.key_id == key_id)
}

/// Add an "always allow" persistent rule.
pub fn add_always_rule(vault_dir: &Path, key_id: &str, key_name: &str) -> Result<(), std::io::Error> {
    let mut rules = load_rules(vault_dir);
    // Don't duplicate
    if rules.iter().any(|r| r.key_id == key_id) {
        return Ok(());
    }
    rules.push(PolicyRule {
        key_id: key_id.to_string(),
        key_name: key_name.to_string(),
        created_at: Utc::now(),
    });
    save_rules(vault_dir, &rules)
}

/// Remove a persistent rule by key_id.
pub fn remove_rule(vault_dir: &Path, key_id: &str) -> Result<(), std::io::Error> {
    let mut rules = load_rules(vault_dir);
    rules.retain(|r| r.key_id != key_id);
    save_rules(vault_dir, &rules)
}

/// Remove all persistent rules.
pub fn clear_all_rules(vault_dir: &Path) -> Result<(), std::io::Error> {
    save_rules(vault_dir, &[])
}
