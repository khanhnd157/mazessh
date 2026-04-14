use tauri::State;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::models::bridge_provider::*;
use crate::services::{bridge_service, provider_health, wsl_service};
use crate::state::AppState;

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
    state: State<'_, AppState>,
) -> Result<DistroBridgeStatus, MazeSshError> {
    ensure_unlocked(&state)?;
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    // Ensure distro entry exists in config with default provider
    if !config.distros.iter().any(|d| d.distro_name == distro) {
        config.distros.push(DistroBridgeConfig {
            distro_name: distro.clone(),
            enabled: true,
            socket_path: None,
            provider: BridgeProvider::default(),
        });
    } else {
        // Mark as enabled
        if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
            d.enabled = true;
        }
    }

    let result = bridge_service::bootstrap_distro(&distro, &config)?;

    // Save config on success
    bridge_service::save_bridge_config(&config)?;

    Ok(result)
}

/// Remove bridge from a WSL distro
#[tauri::command]
pub fn teardown_bridge(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    bridge_service::teardown_distro(&distro)?;

    // Remove from config
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;
    config.distros.retain(|d| d.distro_name != distro);
    bridge_service::save_bridge_config(&config)?;

    Ok(())
}

/// Start the relay service in a WSL distro
#[tauri::command]
pub fn start_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    bridge_service::start_relay(&distro)
}

/// Stop the relay service in a WSL distro
#[tauri::command]
pub fn stop_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    bridge_service::stop_relay(&distro)
}

/// Restart the relay service in a WSL distro
#[tauri::command]
pub fn restart_bridge_relay(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    bridge_service::restart_relay(&distro)
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
        });
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(())
}
