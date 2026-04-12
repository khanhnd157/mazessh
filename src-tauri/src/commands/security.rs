use tauri::{Emitter, State};

use crate::error::MazeSshError;
use crate::models::security::{AuditEntry, LockStateResponse, SecuritySettings};
use crate::services::{audit_service, lock_service, session_service, settings_service, ssh_engine};
use crate::state::AppState;

/// Helper: lock the app (used by command and by lib.rs on_window_event)
pub fn do_lock(app: &tauri::AppHandle) -> Result<(), MazeSshError> {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let mut security = state.security.lock().unwrap();
    if security.is_locked {
        return Ok(());
    }
    security.is_locked = true;
    drop(security);

    let _ = ssh_engine::agent_clear_keys();
    let _ = app.emit("lock-state-changed", serde_json::json!({ "is_locked": true }));

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "lock".to_string(),
        profile_name: None,
        result: "Manual lock".to_string(),
    });

    Ok(())
}

/// Helper: check if app is unlocked (used as guard in other commands)
pub fn ensure_unlocked(state: &AppState) -> Result<(), MazeSshError> {
    let security = state.security.lock().unwrap();
    if security.is_locked {
        return Err(MazeSshError::AppLocked);
    }
    // Touch activity on every successful command
    drop(security);
    session_service::touch_activity(state);
    Ok(())
}

#[tauri::command]
pub fn setup_pin(pin: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let security = state.security.lock().unwrap();
    if security.pin_is_set {
        return Err(MazeSshError::SecurityError("PIN already configured".to_string()));
    }
    drop(security);

    lock_service::set_pin(&pin)?;

    let mut security = state.security.lock().unwrap();
    security.pin_is_set = true;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "pin_set".to_string(),
        profile_name: None,
        result: "success".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub fn verify_pin(pin: String, state: State<'_, AppState>) -> Result<bool, MazeSshError> {
    let valid = lock_service::verify_pin(&pin)?;
    if valid {
        let mut security = state.security.lock().unwrap();
        security.is_locked = false;
        security.last_activity = std::time::Instant::now();

        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "unlock".to_string(),
            profile_name: None,
            result: "success".to_string(),
        });
    } else {
        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "unlock_failed".to_string(),
            profile_name: None,
            result: "Invalid PIN".to_string(),
        });
    }
    Ok(valid)
}

#[tauri::command]
pub fn change_pin(
    old_pin: String,
    new_pin: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

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
    });

    Ok(())
}

#[tauri::command]
pub fn remove_pin(pin: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    let valid = lock_service::verify_pin(&pin)?;
    if !valid {
        return Err(MazeSshError::SecurityError("PIN is incorrect".to_string()));
    }

    lock_service::remove_pin()?;

    let mut security = state.security.lock().unwrap();
    security.pin_is_set = false;
    security.is_locked = false;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "pin_removed".to_string(),
        profile_name: None,
        result: "success".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub fn lock_app(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), MazeSshError> {
    let security = state.security.lock().unwrap();
    if !security.pin_is_set {
        return Err(MazeSshError::PinNotSet);
    }
    drop(security);
    do_lock(&app)
}

#[tauri::command]
pub fn get_lock_state(state: State<'_, AppState>) -> Result<LockStateResponse, MazeSshError> {
    let security = state.security.lock().unwrap();
    Ok(LockStateResponse {
        is_locked: security.is_locked,
        pin_is_set: security.pin_is_set,
    })
}

#[tauri::command]
pub fn get_security_settings(state: State<'_, AppState>) -> Result<SecuritySettings, MazeSshError> {
    let security = state.security.lock().unwrap();
    Ok(security.settings.clone())
}

#[tauri::command]
pub fn update_security_settings(
    settings: SecuritySettings,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    settings_service::save_settings(&settings)?;

    let mut security = state.security.lock().unwrap();
    security.settings = settings;

    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "settings_changed".to_string(),
        profile_name: None,
        result: "success".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub fn get_audit_logs(
    limit: u32,
    offset: u32,
    action_filter: Option<String>,
) -> Result<Vec<AuditEntry>, MazeSshError> {
    Ok(audit_service::read_logs(
        limit as usize,
        offset as usize,
        action_filter.as_deref(),
    ))
}

#[tauri::command]
pub fn get_agent_time_remaining(state: State<'_, AppState>) -> Result<Option<u64>, MazeSshError> {
    Ok(session_service::get_agent_time_remaining(&state))
}

#[tauri::command]
pub fn touch_activity(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    session_service::touch_activity(&state);
    Ok(())
}
