use std::sync::Mutex;
use std::time::Instant;

use crate::models::profile::SshProfile;
use crate::models::repo_mapping::RepoMapping;
use crate::models::security::SecuritySettings;

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
    pub security: Mutex<SecurityState>,
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
}

impl AppState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
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
            }),
        }
    }

    pub fn from_persisted(
        profiles: Vec<SshProfile>,
        active_profile_id: Option<String>,
        repo_mappings: Vec<RepoMapping>,
        settings: SecuritySettings,
        pin_is_set: bool,
    ) -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                profiles,
                active_profile_id,
                repo_mappings,
            }),
            security: Mutex::new(SecurityState {
                is_locked: pin_is_set, // Start locked if PIN is configured
                pin_is_set,
                last_activity: Instant::now(),
                agent_activated_at: None,
                settings,
                failed_pin_attempts: 0,
                last_failed_attempt: None,
            }),
        }
    }
}
