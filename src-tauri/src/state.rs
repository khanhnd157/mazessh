use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::Instant;

use maze_vault::VaultSession;

use crate::models::bridge::BridgeConfig;
use crate::models::profile::SshProfile;
use crate::models::repo_mapping::RepoMapping;
use crate::models::security::SecuritySettings;

/// Per-distro watchdog tracking state (not persisted, reset on app restart)
pub struct WatchdogEntry {
    /// Was the relay active on the last poll?
    pub was_active: bool,
    /// Number of auto-restarts since last healthy observation
    pub restart_count: u8,
    /// When the last restart was attempted
    pub last_restart_at: Option<Instant>,
}

pub struct AppState {
    pub inner: RwLock<AppStateInner>,
    pub security: Mutex<SecurityState>,
    pub bridge: RwLock<BridgeConfig>,
    /// Watchdog state: distro_name → per-distro tracking entry.
    pub relay_watchdog_state: Mutex<HashMap<String, WatchdogEntry>>,

    // ── Vault ────────────────────────────────────────────────────
    /// The active vault session. None = vault locked / not initialized.
    pub vault_session: Mutex<Option<VaultSession>>,
    /// Vault directory path (set once at startup, typically ~/.maze-ssh/vault/)
    pub vault_dir: PathBuf,
}

pub struct AppStateInner {
    pub profiles: Vec<SshProfile>,
    pub active_profile_id: Option<String>,
    pub repo_mappings: Vec<RepoMapping>,
}

pub struct SecurityState {
    pub is_locked: bool,
    pub pin_is_set: bool,
    pub last_activity: Instant,
    pub agent_activated_at: Option<Instant>,
    pub settings: SecuritySettings,
    pub failed_pin_attempts: u32,
    pub last_failed_attempt: Option<Instant>,
    /// Monotonically increasing counter — incremented on every profile activation.
    pub activation_counter: u64,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let vault_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".maze-ssh")
            .join("vault");

        Self {
            inner: RwLock::new(AppStateInner {
                profiles: Vec::new(),
                active_profile_id: None,
                repo_mappings: Vec::new(),
            }),
            security: Mutex::new(SecurityState {
                is_locked: false,
                pin_is_set: false,
                last_activity: Instant::now(),
                agent_activated_at: None,
                settings: SecuritySettings::default(),
                failed_pin_attempts: 0,
                last_failed_attempt: None,
                activation_counter: 0,
            }),
            bridge: RwLock::new(BridgeConfig::default()),
            relay_watchdog_state: Mutex::new(HashMap::new()),
            vault_session: Mutex::new(None),
            vault_dir,
        }
    }

    pub fn from_persisted(
        profiles: Vec<SshProfile>,
        active_profile_id: Option<String>,
        repo_mappings: Vec<RepoMapping>,
        settings: SecuritySettings,
        pin_is_set: bool,
        bridge_config: BridgeConfig,
    ) -> Self {
        let vault_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".maze-ssh")
            .join("vault");

        Self {
            inner: RwLock::new(AppStateInner {
                profiles,
                active_profile_id,
                repo_mappings,
            }),
            security: Mutex::new(SecurityState {
                is_locked: pin_is_set,
                pin_is_set,
                last_activity: Instant::now(),
                agent_activated_at: None,
                settings,
                failed_pin_attempts: 0,
                last_failed_attempt: None,
                activation_counter: 0,
            }),
            bridge: RwLock::new(bridge_config),
            relay_watchdog_state: Mutex::new(HashMap::new()),
            vault_session: Mutex::new(None),
            vault_dir,
        }
    }
}
