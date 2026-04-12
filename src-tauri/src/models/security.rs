use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    #[serde(default)]
    pub auto_lock_timeout_minutes: Option<u16>,
    #[serde(default)]
    pub agent_key_timeout_minutes: Option<u16>,
    #[serde(default)]
    pub lock_on_minimize: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout_minutes: None,
            agent_key_timeout_minutes: None,
            lock_on_minimize: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub profile_name: Option<String>,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStateResponse {
    pub is_locked: bool,
    pub pin_is_set: bool,
}
