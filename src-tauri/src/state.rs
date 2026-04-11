use std::sync::Mutex;

use crate::models::profile::SshProfile;

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

pub struct AppStateInner {
    pub profiles: Vec<SshProfile>,
    pub active_profile_id: Option<String>,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                profiles: Vec::new(),
                active_profile_id: None,
            }),
        }
    }

    pub fn from_persisted(profiles: Vec<SshProfile>, active_profile_id: Option<String>) -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                profiles,
                active_profile_id,
            }),
        }
    }
}
