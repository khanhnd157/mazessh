use std::fs;
use std::path::Path;

use zeroize::Zeroizing;

use crate::models::profile::SshProfile;
use crate::models::vault::{MigrationEligible, MigrationFailed, MigrationPreview, MigrationReport, MigrationSkipped, MigrationSuccess};
use crate::services::security as security_service;

use maze_vault::{ImportKeyInput, SshKeyVault, VaultSession};

/// Build a preview of what migration would do for the given profiles.
pub fn build_preview(profiles: &[SshProfile]) -> MigrationPreview {
    let mut eligible = Vec::new();
    let mut skipped = Vec::new();

    for profile in profiles {
        // Skip profiles already migrated
        if profile.vault_key_id.is_some() {
            skipped.push(MigrationSkipped {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                reason: "Already migrated to vault".to_string(),
            });
            continue;
        }

        let key_path = &profile.private_key_path;
        if !key_path.exists() {
            skipped.push(MigrationSkipped {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                reason: format!("Key file not found: {}", key_path.display()),
            });
            continue;
        }

        // Detect algorithm from file content (rough heuristic)
        let algorithm = match fs::read_to_string(key_path) {
            Ok(content) => {
                if content.contains("ssh-ed25519") || content.contains("ED25519") {
                    "ed25519".to_string()
                } else if content.contains("RSA") {
                    "rsa".to_string()
                } else {
                    "unknown".to_string()
                }
            }
            Err(_) => "unknown".to_string(),
        };

        eligible.push(MigrationEligible {
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            key_path: key_path.display().to_string(),
            algorithm,
        });
    }

    MigrationPreview { eligible, skipped }
}

/// Migrate specific profiles into the vault. Returns a report.
/// Does NOT delete original key files.
/// Does NOT modify profiles — the caller must set vault_key_id.
pub fn migrate_profiles(
    session: &VaultSession,
    profiles: &[SshProfile],
    profile_ids: &[String],
    vault_dir: &Path,
) -> MigrationReport {
    let mut succeeded = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for id in profile_ids {
        let profile = match profiles.iter().find(|p| &p.id == id) {
            Some(p) => p,
            None => {
                failed.push(MigrationFailed {
                    profile_id: id.clone(),
                    profile_name: id.clone(),
                    error: "Profile not found".to_string(),
                });
                continue;
            }
        };

        if profile.vault_key_id.is_some() {
            skipped.push(MigrationSkipped {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                reason: "Already migrated".to_string(),
            });
            continue;
        }

        let key_path = &profile.private_key_path;
        if !key_path.exists() {
            failed.push(MigrationFailed {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                error: format!("Key file not found: {}", key_path.display()),
            });
            continue;
        }

        // Read private key PEM into a Zeroizing wrapper so the bytes are
        // cleared from memory as soon as the import completes or fails.
        let pem: Zeroizing<String> = match fs::read_to_string(key_path) {
            Ok(c) => Zeroizing::new(c),
            Err(e) => {
                failed.push(MigrationFailed {
                    profile_id: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    error: format!("Failed to read key file: {e}"),
                });
                continue;
            }
        };

        // Get passphrase from keyring if the key is encrypted.
        // get_passphrase already returns Option<Zeroizing<String>>.
        let passphrase: Option<Zeroizing<String>> = if profile.has_passphrase {
            match security_service::get_passphrase(&profile.id) {
                Ok(Some(p)) => Some(p),
                Ok(None) => {
                    failed.push(MigrationFailed {
                        profile_id: profile.id.clone(),
                        profile_name: profile.name.clone(),
                        error: "Key has passphrase but none stored in keyring".to_string(),
                    });
                    continue;
                }
                Err(e) => {
                    failed.push(MigrationFailed {
                        profile_id: profile.id.clone(),
                        profile_name: profile.name.clone(),
                        error: format!("Failed to get passphrase from keyring: {e}"),
                    });
                    continue;
                }
            }
        } else {
            None
        };

        // Build the import request. The vault library receives owned Strings;
        // pem and passphrase are Zeroizing wrappers that will wipe their
        // backing memory once they are dropped at the end of this block.
        let input = ImportKeyInput {
            private_key_pem: pem.to_string(),
            name: format!("{} (migrated)", profile.name),
            comment: Some(profile.email.clone()),
            export_policy: None,
            source_passphrase: passphrase.as_ref().map(|p| p.as_str().to_string()),
        };
        // pem and passphrase are dropped (and zeroized) here

        match SshKeyVault::import_key(session, input, vault_dir) {
            Ok(item) => {
                succeeded.push(MigrationSuccess {
                    profile_id: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    vault_key_id: item.id,
                });
            }
            Err(e) => {
                failed.push(MigrationFailed {
                    profile_id: profile.id.clone(),
                    profile_name: profile.name.clone(),
                    error: e.to_string(),
                });
            }
        }
    }

    MigrationReport { succeeded, skipped, failed }
}
