use tauri::{Emitter, State};

use crate::error::MazeSshError;
use crate::models::security::{AuditEntry, LockStateResponse, SecuritySettings};
use crate::services::{audit_service, lock_service, session_service, settings_service, ssh_engine, validation};
use crate::state::AppState;
use zeroize::Zeroize;

/// Helper: lock the app (used by command and by lib.rs on_window_event)
pub fn do_lock(app: &tauri::AppHandle) -> Result<(), MazeSshError> {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    if security.is_locked {
        return Ok(());
    }
    security.is_locked = true;
    drop(security);

    // Lock vault session (zeroize VEK) and clear session rules
    {
        let mut vault_guard = state.vault_session.lock().map_err(|_| MazeSshError::StateLockError)?;
        *vault_guard = None; // Drop triggers ZeroizeOnDrop
    }
    state.session_rules.clear();

    // Emit IMMEDIATELY so UI locks instantly
    let _ = app.emit("lock-state-changed", serde_json::json!({ "is_locked": true }));

    // Heavy work in background — don't block UI
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let _ = ssh_engine::agent_clear_keys();

            audit_service::append_log(&AuditEntry {
                timestamp: chrono::Utc::now().to_rfc3339(),
                action: "lock".to_string(),
                profile_name: None,
                result: "Locked — agent keys cleared".to_string(),
                ..Default::default()
            });

            // Also emit agent status
            let _ = app_clone.emit(
                "agent-status",
                serde_json::json!({ "status": "Locked — keys cleared", "success": true }),
            );
        })
        .await
        .ok();
    });

    Ok(())
}

/// Helper: check if app is unlocked (used as guard in other commands)
pub fn ensure_unlocked(state: &AppState) -> Result<(), MazeSshError> {
    let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    if security.is_locked {
        return Err(MazeSshError::AppLocked);
    }
    // Touch activity on every successful command
    drop(security);
    session_service::touch_activity(state);
    Ok(())
}

#[tauri::command]
pub fn setup_pin(mut pin: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let result = (|| {
        validation::validate_pin(&pin)?;

        let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        if security.pin_is_set {
            return Err(MazeSshError::SecurityError("PIN already configured".to_string()));
        }
        drop(security);

        lock_service::set_pin(&pin)?;

        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.pin_is_set = true;

        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "pin_set".to_string(),
            profile_name: None,
            result: "success".to_string(),
            ..Default::default()
        });

        Ok(())
    })();
    pin.zeroize();
    result
}

const MAX_PIN_ATTEMPTS: u32 = 5;
const LOCKOUT_SECONDS: u64 = 60;

#[tauri::command]
pub fn verify_pin(mut pin: String, state: State<'_, AppState>) -> Result<bool, MazeSshError> {
    let result = (|| {
    // Rate limiting: check if locked out from too many attempts
    {
        let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        if security.failed_pin_attempts >= MAX_PIN_ATTEMPTS {
            if let Some(last) = security.last_failed_attempt {
                let elapsed = last.elapsed().as_secs();
                if elapsed < LOCKOUT_SECONDS {
                    let remaining = LOCKOUT_SECONDS - elapsed;
                    return Err(MazeSshError::SecurityError(format!(
                        "Too many failed attempts. Try again in {} seconds.",
                        remaining
                    )));
                }
            }
        }
    }

    let valid = lock_service::verify_pin(&pin)?;
    if valid {
        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.is_locked = false;
        security.last_activity = std::time::Instant::now();
        security.failed_pin_attempts = 0;
        security.last_failed_attempt = None;

        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "unlock".to_string(),
            profile_name: None,
            result: "success".to_string(),
            ..Default::default()
        });

        // Auto-unlock vault if SameAsPin mode
        {
            let vault_unlock_mode = security.settings.vault_unlock_mode;
            drop(security); // release before vault operation (Argon2 is slow)
            if vault_unlock_mode == crate::models::security::VaultUnlockMode::SameAsPin
                && maze_vault::SshKeyVault::is_initialized(&state.vault_dir)
            {
                if let Ok(session) = maze_vault::SshKeyVault::unlock(&pin, &state.vault_dir) {
                    if let Ok(mut guard) = state.vault_session.lock() {
                        *guard = Some(session);
                    }
                }
            }
        }
    } else {
        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.failed_pin_attempts += 1;
        security.last_failed_attempt = Some(std::time::Instant::now());

        let attempts_left = MAX_PIN_ATTEMPTS.saturating_sub(security.failed_pin_attempts);
        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "unlock_failed".to_string(),
            profile_name: None,
            result: format!("Invalid PIN ({} attempts remaining)", attempts_left),
            ..Default::default()
        });
    }
    Ok(valid)
    })();
    pin.zeroize();
    result
}

#[tauri::command]
pub fn change_pin(
    mut old_pin: String,
    mut new_pin: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let result = (|| {
    ensure_unlocked(&state)?;

    validation::validate_pin(&new_pin)?;

    let valid = lock_service::verify_pin(&old_pin)?;
    if !valid {
        return Err(MazeSshError::SecurityError("Current PIN is incorrect".to_string()));
    }

    lock_service::set_pin(&new_pin)?;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "pin_changed".to_string(),
        profile_name: None,
        result: "success".to_string(),
        ..Default::default()
    });

    Ok(())
    })();
    old_pin.zeroize();
    new_pin.zeroize();
    result
}

#[tauri::command]
pub fn remove_pin(mut pin: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let result = (|| {
    ensure_unlocked(&state)?;

    let valid = lock_service::verify_pin(&pin)?;
    if !valid {
        return Err(MazeSshError::SecurityError("PIN is incorrect".to_string()));
    }

    lock_service::remove_pin()?;

    let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    security.pin_is_set = false;
    security.is_locked = false;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "pin_removed".to_string(),
        profile_name: None,
        result: "success".to_string(),
        ..Default::default()
    });

    Ok(())
    })();
    pin.zeroize();
    result
}

#[tauri::command]
pub fn lock_app(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), MazeSshError> {
    let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    if !security.pin_is_set {
        return Err(MazeSshError::PinNotSet);
    }
    drop(security);
    do_lock(&app)
}

#[tauri::command]
pub fn get_lock_state(state: State<'_, AppState>) -> Result<LockStateResponse, MazeSshError> {
    let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    Ok(LockStateResponse {
        is_locked: security.is_locked,
        pin_is_set: security.pin_is_set,
    })
}

#[tauri::command]
pub fn get_security_settings(state: State<'_, AppState>) -> Result<SecuritySettings, MazeSshError> {
    ensure_unlocked(&state)?;
    let security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    Ok(security.settings.clone())
}

#[tauri::command]
pub fn update_security_settings(
    settings: SecuritySettings,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    settings_service::save_settings(&settings)?;

    let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
    security.settings = settings;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "settings_changed".to_string(),
        profile_name: None,
        result: "success".to_string(),
        ..Default::default()
    });

    Ok(())
}

#[tauri::command]
pub fn get_audit_logs(
    limit: u32,
    offset: u32,
    action_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AuditEntry>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(audit_service::read_logs(
        limit as usize,
        offset as usize,
        action_filter.as_deref(),
    ))
}

#[tauri::command]
pub fn get_agent_time_remaining(state: State<'_, AppState>) -> Result<Option<u64>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(session_service::get_agent_time_remaining(&state))
}

#[tauri::command]
pub fn touch_activity(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    session_service::touch_activity(&state);
    Ok(())
}
