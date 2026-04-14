use tauri::State;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::models::bridge_provider::*;
use crate::models::security::AuditEntry;
use crate::services::{audit_service, bridge_service, provider_health, relay_bundler, wsl_service};
use crate::state::AppState;

fn log_bridge_audit(action: &str, distro: &str, provider: &str, result: &str) {
    audit_service::append_log(&AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: format!("bridge_{}", action),
        profile_name: None,
        result: result.to_string(),
        distro: Some(distro.to_string()),
        provider: Some(provider.to_string()),
    });
}

use super::security::ensure_unlocked;

/// Get full bridge overview: WSL availability, provider status, all distro statuses
#[tauri::command]
pub fn get_bridge_overview(
    state: State<'_, AppState>,
) -> Result<BridgeOverview, MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    Ok(bridge_service::get_bridge_overview(&config))
}

/// List detected WSL distributions (lightweight, no health checks)
#[tauri::command]
pub fn list_wsl_distros(
    state: State<'_, AppState>,
) -> Result<Vec<WslDistro>, MazeSshError> {
    ensure_unlocked(&state)?;
    wsl_service::list_distros()
}

/// Bootstrap bridge relay into a specific WSL distro
#[tauri::command]
pub fn bootstrap_bridge(
    distro: String,
    relay_mode: Option<String>,
    state: State<'_, AppState>,
) -> Result<DistroBridgeStatus, MazeSshError> {
    ensure_unlocked(&state)?;
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    let parsed_relay_mode = match relay_mode.as_deref() {
        Some("daemon") => RelayMode::Daemon,
        _ => RelayMode::Systemd,
    };

    // Ensure distro entry exists in config with default provider
    if !config.distros.iter().any(|d| d.distro_name == distro) {
        config.distros.push(DistroBridgeConfig {
            distro_name: distro.clone(),
            enabled: true,
            socket_path: None,
            provider: BridgeProvider::default(),
            allow_agent_forwarding: false,
            relay_mode: parsed_relay_mode,
        });
    } else {
        // Mark as enabled and update relay_mode
        if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
            d.enabled = true;
            d.relay_mode = parsed_relay_mode;
        }
    }

    let provider_name = config.distros.iter()
        .find(|d| d.distro_name == distro)
        .map(|d| d.provider.display_name().to_string())
        .unwrap_or_default();

    let result = bridge_service::bootstrap_distro(&distro, &config)?;
    bridge_service::save_bridge_config(&config)?;

    log_bridge_audit("bootstrap", &distro, &provider_name, "Bridge setup completed");

    Ok(result)
}

/// Remove bridge from a WSL distro
#[tauri::command]
pub fn teardown_bridge(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    let config_snap = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?.clone();
    let provider_name = config_snap.distros.iter()
        .find(|d| d.distro_name == distro)
        .map(|d| d.provider.display_name().to_string())
        .unwrap_or_default();

    bridge_service::teardown_distro(&distro, &config_snap)?;

    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;
    config.distros.retain(|d| d.distro_name != distro);
    bridge_service::save_bridge_config(&config)?;

    log_bridge_audit("teardown", &distro, &provider_name, "Bridge removed");

    Ok(())
}

/// Start the relay service in a WSL distro
#[tauri::command]
pub fn start_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    let relay_mode = config.distros.iter().find(|d| d.distro_name == distro).map(|d| d.relay_mode.clone()).unwrap_or_default();
    drop(config);
    bridge_service::start_relay(&distro, &relay_mode)?;
    log_bridge_audit("start", &distro, "", "Relay started");
    Ok(())
}

/// Stop the relay service in a WSL distro
#[tauri::command]
pub fn stop_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    let relay_mode = config.distros.iter().find(|d| d.distro_name == distro).map(|d| d.relay_mode.clone()).unwrap_or_default();
    drop(config);
    bridge_service::stop_relay(&distro, &relay_mode)?;
    log_bridge_audit("stop", &distro, "", "Relay stopped");
    Ok(())
}

/// Restart the relay service in a WSL distro
#[tauri::command]
pub fn restart_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    let relay_mode = config.distros.iter().find(|d| d.distro_name == distro).map(|d| d.relay_mode.clone()).unwrap_or_default();
    drop(config);
    bridge_service::restart_relay(&distro, &relay_mode)?;
    log_bridge_audit("restart", &distro, "", "Relay restarted");
    Ok(())
}

/// Get detailed bridge status for one distro
#[tauri::command]
pub fn get_distro_bridge_status(
    distro: String,
    state: State<'_, AppState>,
) -> Result<DistroBridgeStatus, MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    Ok(bridge_service::get_distro_status(&distro, &config))
}

/// Enable or disable bridge for a distro in config
#[tauri::command]
pub fn set_bridge_enabled(
    distro: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
        d.enabled = enabled;
    } else if enabled {
        config.distros.push(DistroBridgeConfig {
            distro_name: distro,
            enabled: true,
            socket_path: None,
            provider: BridgeProvider::default(),
            allow_agent_forwarding: false,
            relay_mode: RelayMode::default(),
        });
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(())
}

/// List all known providers and their Windows-side availability
#[tauri::command]
pub fn list_bridge_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderStatus>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(provider_health::check_all_providers())
}

/// Change the provider for a specific distro.
/// Requires teardown first if the bridge is actively running.
#[tauri::command]
pub fn set_distro_provider(
    distro: String,
    provider: BridgeProvider,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
        if d.provider != provider && d.enabled {
            // Check if relay is actually installed — if so, require teardown first
            if wsl_service::wsl_file_exists(&distro, &format!("~/{}", crate::models::bridge::RELAY_SCRIPT_PATH)) {
                return Err(MazeSshError::BridgeError(
                    "Teardown the current bridge before switching providers".to_string(),
                ));
            }
        }
        d.provider = provider;
    } else {
        config.distros.push(DistroBridgeConfig {
            distro_name: distro,
            enabled: false,
            socket_path: None,
            provider,
            allow_agent_forwarding: false,
            relay_mode: RelayMode::default(),
        });
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(())
}

/// Get the recommended provider based on availability and security scoring
#[tauri::command]
pub fn get_recommended_provider(
    state: State<'_, AppState>,
) -> Result<Option<BridgeProvider>, MazeSshError> {
    ensure_unlocked(&state)?;
    let statuses = provider_health::check_all_providers();
    Ok(provider_health::recommend_provider(&statuses))
}

/// Run step-by-step bridge diagnostics for a distro
#[tauri::command]
pub fn run_bridge_diagnostics(
    distro: String,
    state: State<'_, AppState>,
) -> Result<DiagnosticsResult, MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    Ok(bridge_service::run_diagnostics(&distro, &config))
}

/// Fetch recent relay service journal logs from a WSL distro
#[tauri::command]
pub fn get_relay_logs(
    distro: String,
    lines: u32,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    bridge_service::get_relay_logs(&distro, lines)
}

/// Get installed relay binary versions
#[tauri::command]
pub fn get_relay_binary_versions(
    state: State<'_, AppState>,
) -> Result<BinaryVersion, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(relay_bundler::get_installed_versions())
}

/// Download a relay binary from GitHub releases (emits binary-download-progress events)
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn download_relay_binary(
    binary: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let relay_binary = RelayBinary::from_key(&binary)
        .ok_or_else(|| MazeSshError::BridgeError(format!("Unknown binary: {binary}")))?;
    relay_bundler::download_binary(relay_binary, &app).await
}

/// Enable or disable SSH agent forwarding for a distro
#[tauri::command]
pub fn set_agent_forwarding(
    distro: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
        d.allow_agent_forwarding = enabled;
    }

    bridge_service::save_bridge_config(&config)?;

    // Apply live if distro has a bridge installed
    if wsl_service::wsl_file_exists(&distro, &format!("~/{}", crate::models::bridge::RELAY_SCRIPT_PATH)) {
        bridge_service::configure_agent_forwarding(&distro, enabled)?;
    }

    log_bridge_audit(
        "forwarding",
        &distro,
        "",
        if enabled { "Agent forwarding enabled" } else { "Agent forwarding disabled" },
    );

    Ok(())
}
