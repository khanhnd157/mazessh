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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditEntry {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub result: String,
    /// WSL distro name (for bridge operations)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distro: Option<String>,
    /// Provider display name (for bridge operations)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStateResponse {
    pub is_locked: bool,
    pub pin_is_set: bool,
}
