use serde::{Deserialize, Serialize};

use super::profile::Provider;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub action: String,
    pub profile_name: String,
    pub provider: Provider,
    pub detail: String,
    pub timestamp: String,
}
