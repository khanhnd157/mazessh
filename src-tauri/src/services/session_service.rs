use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

use crate::models::security::AuditEntry;
use crate::services::{audit_service, ssh_engine};
use crate::state::AppState;

pub fn touch_activity(state: &AppState) {
    if let Ok(mut security) = state.security.lock() {
        security.last_activity = Instant::now();
    }
}

pub fn check_inactivity_and_lock(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let mut security = match state.security.lock() {
        Ok(s) => s,
        Err(_) => return,
    };

    if security.is_locked || !security.pin_is_set {
        return;
    }

    let timeout_minutes = match security.settings.auto_lock_timeout_minutes {
        Some(m) if m > 0 => m,
        _ => return,
    };

    let elapsed = security.last_activity.elapsed();
    if elapsed >= Duration::from_secs(timeout_minutes as u64 * 60) {
        security.is_locked = true;
        drop(security);

        // Clear agent keys
        let _ = ssh_engine::agent_clear_keys();

        // Emit event to frontend
        let _ = app.emit("lock-state-changed", serde_json::json!({ "is_locked": true }));

        // Audit log
        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "auto_lock".to_string(),
            profile_name: None,
            result: format!("Locked after {} minutes of inactivity", timeout_minutes),
        });
    }
}

pub fn check_agent_expiry(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let mut security = match state.security.lock() {
        Ok(s) => s,
        Err(_) => return,
    };

    let timeout_minutes = match security.settings.agent_key_timeout_minutes {
        Some(m) if m > 0 => m,
        _ => return,
    };

    let activated_at = match security.agent_activated_at {
        Some(t) => t,
        None => return,
    };

    let elapsed = activated_at.elapsed();
    if elapsed >= Duration::from_secs(timeout_minutes as u64 * 60) {
        security.agent_activated_at = None;
        drop(security);

        // Clear agent keys
        let _ = ssh_engine::agent_clear_keys();
        let _ = ssh_engine::clear_user_env_git_ssh_command();

        // Clear active profile
        {
            let mut inner = state.inner.lock().unwrap();
            inner.active_profile_id = None;
        }
        let _ = crate::services::profile_service::save_active_profile_id(None);

        // Emit events
        let _ = app.emit(
            "agent-expired",
            serde_json::json!({ "message": "Agent keys expired and cleared" }),
        );

        audit_service::append_log(&AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action: "agent_expired".to_string(),
            profile_name: None,
            result: format!("Keys cleared after {} minutes", timeout_minutes),
        });
    }
}

pub fn get_agent_time_remaining(state: &AppState) -> Option<u64> {
    let security = state.security.lock().ok()?;
    let timeout_minutes = security.settings.agent_key_timeout_minutes?;
    let activated_at = security.agent_activated_at?;
    let total = Duration::from_secs(timeout_minutes as u64 * 60);
    let elapsed = activated_at.elapsed();

    if elapsed >= total {
        Some(0)
    } else {
        Some((total - elapsed).as_secs())
    }
}
