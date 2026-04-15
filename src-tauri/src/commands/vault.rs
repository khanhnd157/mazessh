use tauri::State;
use zeroize::Zeroize;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::models::vault::*;
use crate::services::{audit_service, migration_service, policy_service, profile_service};
use crate::state::AppState;

use maze_vault::{
    ExportPolicy, GenerateKeyInput, ImportKeyInput, SshKeyItem, SshKeyItemSummary, SshKeyVault,
    UpdateKeyInput,
};

// ── Agent info ───────────────────────────────────────────────────

#[tauri::command]
pub fn get_agent_pipe_path() -> String {
    crate::services::agent_service::PIPE_NAME.to_string()
}

// ── Vault lifecycle ──────────────────────────────────────────────

#[tauri::command]
pub fn vault_get_state(state: State<'_, AppState>) -> Result<VaultStateResponse, MazeSshError> {
    let initialized = SshKeyVault::is_initialized(&state.vault_dir);
    let unlocked = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?
        .is_some();
    let key_count = if initialized {
        SshKeyVault::list_keys(&state.vault_dir)
            .map(|keys| keys.len())
            .unwrap_or(0)
    } else {
        0
    };
    Ok(VaultStateResponse {
        initialized,
        unlocked,
        key_count,
    })
}

#[tauri::command]
pub fn vault_init(
    mut passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let result = (|| {
        ensure_unlocked(&state)?;
        SshKeyVault::init(&passphrase, &state.vault_dir)?;
        // Immediately unlock after init
        let session = SshKeyVault::unlock(&passphrase, &state.vault_dir)?;
        let mut guard = state
            .vault_session
            .lock()
            .map_err(|_| MazeSshError::StateLockError)?;
        *guard = Some(session);
        audit_service::log_action("vault_init", None, "success");
        Ok(())
    })();
    passphrase.zeroize();
    result
}

#[tauri::command]
pub fn vault_unlock(
    mut passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let result = (|| {
        ensure_unlocked(&state)?;
        let session = SshKeyVault::unlock(&passphrase, &state.vault_dir)?;
        let mut guard = state
            .vault_session
            .lock()
            .map_err(|_| MazeSshError::StateLockError)?;
        *guard = Some(session);
        audit_service::log_action("vault_unlock", None, "success");
        Ok(())
    })();
    passphrase.zeroize();
    result
}

#[tauri::command]
pub fn vault_lock(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let mut guard = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    // Dropping the old Some(VaultSession) triggers ZeroizeOnDrop on the VEK
    *guard = None;
    audit_service::log_action("vault_lock", None, "success");
    Ok(())
}

#[tauri::command]
pub fn vault_change_passphrase(
    mut old_passphrase: String,
    mut new_passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let result = (|| {
        ensure_unlocked(&state)?;
        SshKeyVault::change_passphrase(&old_passphrase, &new_passphrase, &state.vault_dir)?;
        // Re-unlock with new passphrase
        let session = SshKeyVault::unlock(&new_passphrase, &state.vault_dir)?;
        let mut guard = state
            .vault_session
            .lock()
            .map_err(|_| MazeSshError::StateLockError)?;
        *guard = Some(session);
        audit_service::log_action("vault_change_passphrase", None, "success");
        Ok(())
    })();
    old_passphrase.zeroize();
    new_passphrase.zeroize();
    result
}

// ── Key CRUD ─────────────────────────────────────────────────────

#[tauri::command]
pub fn vault_generate_key(
    request: GenerateKeyRequest,
    state: State<'_, AppState>,
) -> Result<SshKeyItem, MazeSshError> {
    ensure_unlocked(&state)?;
    let guard = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    let session = guard.as_ref().ok_or(MazeSshError::VaultLocked)?;

    let input = GenerateKeyInput {
        name: request.name,
        algorithm: request.algorithm,
        comment: request.comment,
        export_policy: request.allow_private_export.map(|allow| ExportPolicy {
            allow_private_export: allow,
        }),
    };

    let item = SshKeyVault::generate_key(session, input, &state.vault_dir)?;
    audit_service::log_action("vault_generate_key", Some(&item.name), "success");
    Ok(item)
}

#[tauri::command]
pub fn vault_import_key(
    mut request: ImportKeyRequest,
    state: State<'_, AppState>,
) -> Result<SshKeyItem, MazeSshError> {
    let result = (|| {
        ensure_unlocked(&state)?;
        let guard = state
            .vault_session
            .lock()
            .map_err(|_| MazeSshError::StateLockError)?;
        let session = guard.as_ref().ok_or(MazeSshError::VaultLocked)?;

        let input = maze_vault::ImportKeyInput {
            private_key_pem: request.private_key_pem.clone(),
            name: request.name.clone(),
            comment: request.comment.clone(),
            export_policy: request.allow_private_export.map(|allow| ExportPolicy {
                allow_private_export: allow,
            }),
            source_passphrase: request.source_passphrase.clone(),
        };

        let item = SshKeyVault::import_key(session, input, &state.vault_dir)?;
        audit_service::log_action("vault_import_key", Some(&item.name), "success");
        Ok(item)
    })();
    request.private_key_pem.zeroize();
    if let Some(ref mut p) = request.source_passphrase {
        p.zeroize();
    }
    result
}

#[tauri::command]
pub fn vault_list_keys(
    state: State<'_, AppState>,
) -> Result<Vec<SshKeyItemSummary>, MazeSshError> {
    ensure_unlocked(&state)?;
    let keys = SshKeyVault::list_keys(&state.vault_dir)?;
    Ok(keys)
}

#[tauri::command]
pub fn vault_get_key(id: String, state: State<'_, AppState>) -> Result<SshKeyItem, MazeSshError> {
    ensure_unlocked(&state)?;
    let item = SshKeyVault::get_key(&id, &state.vault_dir)?;
    Ok(item)
}

#[tauri::command]
pub fn vault_update_key(
    id: String,
    request: UpdateKeyRequest,
    state: State<'_, AppState>,
) -> Result<SshKeyItem, MazeSshError> {
    ensure_unlocked(&state)?;

    let input = UpdateKeyInput {
        name: request.name,
        comment: request.comment,
        export_policy: request.allow_private_export.map(|allow| ExportPolicy {
            allow_private_export: allow,
        }),
    };

    let item = SshKeyVault::update_key(&id, input, &state.vault_dir)?;
    audit_service::log_action("vault_update_key", Some(&item.name), "success");
    Ok(item)
}

#[tauri::command]
pub fn vault_delete_key(id: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let guard = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    let session = guard.as_ref().ok_or(MazeSshError::VaultLocked)?;

    // Get key name for audit before deleting
    let name = SshKeyVault::get_key(&id, &state.vault_dir)
        .map(|k| k.name)
        .unwrap_or_else(|_| id.clone());

    SshKeyVault::delete_key(session, &id, &state.vault_dir)?;
    audit_service::log_action("vault_delete_key", Some(&name), "success");
    Ok(())
}

#[tauri::command]
pub fn vault_archive_key(id: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    SshKeyVault::archive_key(&id, &state.vault_dir)?;
    audit_service::log_action("vault_archive_key", Some(&id), "success");
    Ok(())
}

// ── Export ────────────────────────────────────────────────────────

#[tauri::command]
pub fn vault_export_public_key(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let key = SshKeyVault::export_public_key(&id, &state.vault_dir)?;
    Ok(key)
}

#[tauri::command]
pub fn vault_export_private_key(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let guard = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    let session = guard.as_ref().ok_or(MazeSshError::VaultLocked)?;

    let pem = SshKeyVault::export_private_key(session, &id, &state.vault_dir)?;
    audit_service::log_action("vault_export_private_key", Some(&id), "success");
    Ok(pem)
}

// ── Migration ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_migration_preview(state: State<'_, AppState>) -> Result<MigrationPreview, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    Ok(migration_service::build_preview(&inner.profiles))
}

#[tauri::command]
pub fn migrate_profiles_to_vault(
    profile_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<MigrationReport, MazeSshError> {
    ensure_unlocked(&state)?;
    let guard = state
        .vault_session
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    let session = guard.as_ref().ok_or(MazeSshError::VaultLocked)?;

    let profiles = {
        let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
        inner.profiles.clone()
    };

    let report = migration_service::migrate_profiles(&session, &profiles, &profile_ids, &state.vault_dir);

    // Update profiles with vault_key_id for successful migrations
    if !report.succeeded.is_empty() {
        let mut inner = state.inner.write().map_err(|_| MazeSshError::StateLockError)?;
        for success in &report.succeeded {
            if let Some(profile) = inner.profiles.iter_mut().find(|p| p.id == success.profile_id) {
                profile.vault_key_id = Some(success.vault_key_id.clone());
            }
        }
        let _ = profile_service::save_profiles(&inner.profiles);
    }

    let migrated_count = report.succeeded.len();
    audit_service::log_action(
        "migrate_profiles",
        None,
        &format!("{} migrated, {} skipped, {} failed", migrated_count, report.skipped.len(), report.failed.len()),
    );

    Ok(report)
}

#[tauri::command]
pub fn delete_original_key_file(
    key_path: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    let path = std::path::Path::new(&key_path);

    // Safety: only delete files under ~/.ssh/
    let ssh_dir = dirs::home_dir()
        .ok_or_else(|| MazeSshError::ConfigError("Home directory not found".to_string()))?
        .join(".ssh");

    let canonical_path = dunce::canonicalize(path)
        .map_err(|e| MazeSshError::ConfigError(format!("Cannot resolve path: {e}")))?;
    let canonical_ssh = dunce::canonicalize(&ssh_dir)
        .unwrap_or_else(|_| ssh_dir.clone());

    if !canonical_path.starts_with(&canonical_ssh) {
        return Err(MazeSshError::ValidationError(
            "Can only delete key files under ~/.ssh/".to_string(),
        ));
    }

    if canonical_path.exists() {
        std::fs::remove_file(&canonical_path)?;
    }

    // Also delete .pub file if it exists
    let pub_path = canonical_path.with_extension("pub");
    if pub_path.exists() {
        let _ = std::fs::remove_file(&pub_path);
    }

    audit_service::log_action("delete_original_key", Some(&key_path), "success");
    Ok(())
}

// ── Consent ──────────────────────────────────────────────────────

#[tauri::command]
pub fn respond_to_consent(
    consent_id: String,
    approved: bool,
    selected_key_id: String,
    allow_mode: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let mut consents = state
        .pending_consents
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;

    if let Some(pending) = consents.remove(&consent_id) {
        let decision = crate::state::ConsentDecision {
            approved,
            selected_key_id,
            allow_mode: allow_mode.clone(),
        };
        // If the receiver was dropped (timeout), this is a no-op
        let _ = pending.tx.send(decision);

        let action = if approved { "consent_approved" } else { "consent_denied" };
        audit_service::log_action(action, Some(&pending.key_name), &allow_mode);
    }

    Ok(())
}

#[tauri::command]
pub fn get_pending_consent(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, MazeSshError> {
    let consents = state
        .pending_consents
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;

    // Return the first pending consent (FIFO order not guaranteed with HashMap,
    // but there's typically only one at a time)
    if let Some((id, pending)) = consents.iter().next() {
        Ok(Some(serde_json::json!({
            "consent_id": id,
            "key_id": pending.key_id,
            "key_name": pending.key_name,
            "process_name": pending.process_name,
            "host": pending.host,
        })))
    } else {
        Ok(None)
    }
}

// ── Policy rules ─────────────────────────────────────────────────

#[tauri::command]
pub fn list_policy_rules(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, MazeSshError> {
    ensure_unlocked(&state)?;
    let rules = policy_service::load_rules(&state.vault_dir);
    let json_rules: Vec<serde_json::Value> = rules
        .iter()
        .map(|r| {
            serde_json::json!({
                "key_id": r.key_id,
                "key_name": r.key_name,
                "rule_type": "always",
                "created_at": r.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(json_rules)
}

#[tauri::command]
pub fn delete_policy_rule(
    key_id: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    policy_service::remove_rule(&state.vault_dir, &key_id)
        .map_err(|e| MazeSshError::VaultError(e.to_string()))?;
    audit_service::log_action("delete_policy_rule", Some(&key_id), "success");
    Ok(())
}

#[tauri::command]
pub fn clear_all_policy_rules(
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    policy_service::clear_all_rules(&state.vault_dir)
        .map_err(|e| MazeSshError::VaultError(e.to_string()))?;
    state.session_rules.clear();
    audit_service::log_action("clear_policy_rules", None, "success");
    Ok(())
}
