use maze_vault::KeyAlgorithm;
use serde::{Deserialize, Serialize};

/// Response for vault state queries from the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStateResponse {
    pub initialized: bool,
    pub unlocked: bool,
    pub key_count: usize,
}

/// Frontend-facing generate request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateKeyRequest {
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub comment: Option<String>,
    pub allow_private_export: Option<bool>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

/// Frontend-facing import request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportKeyRequest {
    pub name: String,
    pub private_key_pem: String,
    pub comment: Option<String>,
    pub source_passphrase: Option<String>,
    pub allow_private_export: Option<bool>,
}

/// Frontend-facing update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKeyRequest {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub allow_private_export: Option<bool>,
}

/// Migration preview: what will happen if we migrate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPreview {
    pub eligible: Vec<MigrationEligible>,
    pub skipped: Vec<MigrationSkipped>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEligible {
    pub profile_id: String,
    pub profile_name: String,
    pub key_path: String,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSkipped {
    pub profile_id: String,
    pub profile_name: String,
    pub reason: String,
}

/// Migration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub succeeded: Vec<MigrationSuccess>,
    pub skipped: Vec<MigrationSkipped>,
    pub failed: Vec<MigrationFailed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSuccess {
    pub profile_id: String,
    pub profile_name: String,
    pub vault_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationFailed {
    pub profile_id: String,
    pub profile_name: String,
    pub error: String,
}
