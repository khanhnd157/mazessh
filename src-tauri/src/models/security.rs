use serde::{Deserialize, Serialize};

/// How the vault is unlocked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultUnlockMode {
    /// Use the app PIN to derive the vault master key (default)
    SameAsPin,
    /// Use a separate vault-specific passphrase
    SeparatePassphrase,
}

impl Default for VaultUnlockMode {
    fn default() -> Self {
        Self::SameAsPin
    }
}

/// Which SSH agent backend the app uses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Traditional: keys live on disk, loaded via ssh-add
    FileSystem,
    /// Vault: keys encrypted in vault, decrypted on demand
    Vault,
}

impl Default for AgentMode {
    fn default() -> Self {
        Self::FileSystem
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    #[serde(default)]
    pub auto_lock_timeout_minutes: Option<u16>,
    #[serde(default)]
    pub agent_key_timeout_minutes: Option<u16>,
    #[serde(default)]
    pub lock_on_minimize: bool,
    #[serde(default)]
    pub vault_unlock_mode: VaultUnlockMode,
    #[serde(default)]
    pub agent_mode: AgentMode,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout_minutes: None,
            agent_key_timeout_minutes: None,
            lock_on_minimize: false,
            vault_unlock_mode: VaultUnlockMode::default(),
            agent_mode: AgentMode::default(),
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
