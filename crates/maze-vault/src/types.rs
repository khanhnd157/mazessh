use chrono::{DateTime, Utc};
use maze_crypto::{EncryptedBlob, KdfParams};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

// ─── Key algorithm ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAlgorithm {
    Ed25519,
    Rsa4096,
}

impl std::fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAlgorithm::Ed25519 => write!(f, "ed25519"),
            KeyAlgorithm::Rsa4096 => write!(f, "rsa-4096"),
        }
    }
}

// ─── Key state ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyState {
    Active,
    Archived,
}

// ─── Export policy ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPolicy {
    pub allow_private_export: bool,
}

impl Default for ExportPolicy {
    fn default() -> Self {
        Self {
            allow_private_export: true,
        }
    }
}

// ─── Full SSH key item (metadata in vault-meta.json) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyItem {
    pub id: String,
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub fingerprint: String,
    pub public_key_openssh: String,
    pub state: KeyState,
    pub export_policy: ExportPolicy,
    pub comment: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── Summary for list views ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyItemSummary {
    pub id: String,
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub fingerprint: String,
    pub state: KeyState,
    pub created_at: DateTime<Utc>,
}

impl From<&SshKeyItem> for SshKeyItemSummary {
    fn from(item: &SshKeyItem) -> Self {
        Self {
            id: item.id.clone(),
            name: item.name.clone(),
            algorithm: item.algorithm,
            fingerprint: item.fingerprint.clone(),
            state: item.state,
            created_at: item.created_at,
        }
    }
}

// ─── Vault metadata file ─────────────────────────────────────────

/// On-disk vault metadata. Two-layer key hierarchy:
///   Passphrase → VMK (derived via Argon2id, never stored)
///   VMK → VEK (stored encrypted in this struct)
///   VEK → per-key private key (stored in keys/{id}.enc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub kdf_params: KdfParams,
    pub encrypted_vek: EncryptedBlob,
    pub keys: Vec<SshKeyItem>,
}

// ─── Vault session (in-memory, holds decrypted VEK) ──────────────

/// Holds the decrypted Vault Encryption Key in memory.
/// Zeroizes the VEK when dropped (lock, app exit, etc.).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct VaultSession {
    vek: [u8; 32],
}

impl VaultSession {
    pub(crate) fn new(vek: [u8; 32]) -> Self {
        Self { vek }
    }

    pub(crate) fn vek(&self) -> &[u8; 32] {
        &self.vek
    }
}

// ─── Command inputs ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateKeyInput {
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub comment: Option<String>,
    pub export_policy: Option<ExportPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportKeyInput {
    pub private_key_pem: String,
    pub name: String,
    pub comment: Option<String>,
    pub export_policy: Option<ExportPolicy>,
    /// Passphrase to decrypt the source PEM, if encrypted
    pub source_passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKeyInput {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub export_policy: Option<ExportPolicy>,
}
