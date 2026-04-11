use std::sync::Mutex;

use crate::models::profile::SshProfile;
use crate::models::repo_mapping::RepoMapping;

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

pub struct AppStateInner {
    pub profiles: Vec<SshProfile>,
    pub active_profile_id: Option<String>,
    pub repo_mappings: Vec<RepoMapping>,
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
        }
    }

    pub fn from_persisted(
        profiles: Vec<SshProfile>,
        active_profile_id: Option<String>,
        repo_mappings: Vec<RepoMapping>,
    ) -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                profiles,
                active_profile_id,
                repo_mappings,
            }),
        }
    }
}
