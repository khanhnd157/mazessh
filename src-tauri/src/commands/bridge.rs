use tauri::State;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::models::bridge_provider::*;
use crate::models::security::AuditEntry;
use crate::services::{audit_service, bridge_service, provider_health, relay_bundler, wsl_service};
use crate::services::provider_health::NamedPipeEntry;
use crate::state::AppState;

/// Result of bootstrapping a single distro in a batch operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct BootstrapAllResult {
    pub distro: String,
    pub success: bool,
    pub error: Option<String>,
}

// Validation helper for socket paths
fn validate_socket_path(path: &str) -> Result<(), MazeSshError> {
    if path.is_empty() {
        return Err(MazeSshError::BridgeError("Socket path cannot be empty".to_string()));
    }
    // Unix socket paths are limited to ~108 bytes (including null terminator); leave headroom
    if path.len() > 104 {
        return Err(MazeSshError::BridgeError(
            "Socket path too long (max 104 characters)".to_string(),
        ));
    }
    if !path.starts_with("/tmp/") && !path.starts_with("/run/user/") {
        return Err(MazeSshError::BridgeError(
            "Socket path must start with /tmp/ or /run/user/".to_string(),
        ));
    }
    // Strict character allowlist — no shell metacharacters, spaces, or special bytes
    if !path.bytes().all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.')) {
        return Err(MazeSshError::BridgeError(
            "Socket path contains invalid characters (allowed: a-z A-Z 0-9 / - _ .)".to_string(),
        ));
    }
    // Segment-level traversal check — catches '.' and '..' anywhere in the path
    for segment in path.split('/') {
        if segment == ".." || segment == "." {
            return Err(MazeSshError::BridgeError(
                "Socket path cannot contain '.' or '..' path components".to_string(),
            ));
        }
    }
    Ok(())
}

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
    let mut overview = bridge_service::get_bridge_overview(&config);
    // Overlay watchdog restart counts from in-memory state
    if let Ok(watchdog) = state.relay_watchdog_state.lock() {
        for distro_status in overview.distros.iter_mut() {
            if let Some(entry) = watchdog.get(&distro_status.distro_name) {
                distro_status.watchdog_restart_count = entry.restart_count;
            }
        }
    }
    Ok(overview)
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
            auto_restart: true,
            max_restarts: 5,
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
    let mut status = bridge_service::get_distro_status(&distro, &config);
    if let Ok(watchdog) = state.relay_watchdog_state.lock() {
        if let Some(entry) = watchdog.get(&distro) {
            status.watchdog_restart_count = entry.restart_count;
        }
    }
    Ok(status)
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
            auto_restart: true,
            max_restarts: 5,
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
            auto_restart: true,
            max_restarts: 5,
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

/// Enable or disable auto-restart watchdog for a distro's relay
#[tauri::command]
pub fn set_auto_restart(
    distro: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
        d.auto_restart = enabled;
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(())
}

/// Check for available updates for relay binaries against GitHub latest releases
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn check_relay_binary_updates(
    state: State<'_, AppState>,
) -> Result<Vec<BinaryUpdateStatus>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(relay_bundler::check_for_updates().await)
}

/// Override the Unix socket path for a distro's relay.
/// Validated: must start with /tmp/ or /run/user/, no spaces, no '..'.
/// The relay must be re-bootstrapped after changing this.
#[tauri::command]
pub fn set_distro_socket_path(
    distro: String,
    socket_path: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    validate_socket_path(&socket_path)?;

    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
        d.socket_path = Some(socket_path);
    } else {
        config.distros.push(DistroBridgeConfig {
            distro_name: distro,
            enabled: false,
            socket_path: Some(socket_path),
            provider: BridgeProvider::default(),
            allow_agent_forwarding: false,
            relay_mode: RelayMode::default(),
            auto_restart: true,
            max_restarts: 5,
        });
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(())
}

/// Reset watchdog restart counter for a distro (clears the "paused" state)
#[tauri::command]
pub fn reset_watchdog_restart_count(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut watchdog = state
        .relay_watchdog_state
        .lock()
        .map_err(|_| MazeSshError::StateLockError)?;
    if let Some(entry) = watchdog.get_mut(&distro) {
        entry.restart_count = 0;
        entry.was_active = false;
    }
    Ok(())
}

/// Validate a diagnostic fix command against the exact allowlist.
///
/// Uses exact-match for all fixed commands.  The only variable-content command
/// (`rm -f <socket_path>`) is parsed and the path is validated separately by
/// `validate_socket_path` so it cannot contain shell metacharacters or traversal.
fn validate_diagnostic_cmd(cmd: &str) -> Result<(), MazeSshError> {
    // Exact matches — no partial/prefix ambiguity
    const EXACT: &[&str] = &[
        "systemctl --user start maze-ssh-relay.service",
        "systemctl --user restart maze-ssh-relay.service",
        r#"nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &"#,
        "sudo apt install socat",
        "sudo apt install -y socat",
    ];

    if EXACT.contains(&cmd) {
        return Ok(());
    }

    // "rm -f <socket_path>" — validate the path strictly
    if let Some(path) = cmd.strip_prefix("rm -f ") {
        return validate_socket_path(path).map_err(|_| {
            MazeSshError::BridgeError(format!("Invalid socket path in rm command: {}", path))
        });
    }

    Err(MazeSshError::BridgeError(format!(
        "Command not in allowlist: {}",
        cmd
    )))
}

/// Map a validated diagnostic command to its argv form (no shell interpretation).
/// Returns `None` only for the nohup variant that requires shell features.
fn diagnostic_cmd_to_argv(cmd: &str) -> Vec<String> {
    match cmd {
        "systemctl --user start maze-ssh-relay.service" => {
            vec!["systemctl", "--user", "start", "maze-ssh-relay.service"]
                .into_iter().map(String::from).collect()
        }
        "systemctl --user restart maze-ssh-relay.service" => {
            vec!["systemctl", "--user", "restart", "maze-ssh-relay.service"]
                .into_iter().map(String::from).collect()
        }
        "sudo apt install socat" => {
            vec!["sudo", "apt", "install", "socat"]
                .into_iter().map(String::from).collect()
        }
        "sudo apt install -y socat" => {
            vec!["sudo", "apt", "install", "-y", "socat"]
                .into_iter().map(String::from).collect()
        }
        rm_cmd if rm_cmd.starts_with("rm -f ") => {
            let path = rm_cmd["rm -f ".len()..].to_string();
            vec!["rm".to_string(), "-f".to_string(), path]
        }
        _ => {
            vec!["bash".to_string(), "-c".to_string(), cmd.to_string()]
        }
    }
}

/// Run an allowlisted one-click fix command inside a WSL distro.
#[tauri::command]
pub fn run_diagnostic_fix(
    distro: String,
    cmd: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;

    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Err(MazeSshError::BridgeError("Command cannot be empty".to_string()));
    }
    validate_diagnostic_cmd(trimmed)?;

    let argv = diagnostic_cmd_to_argv(trimmed);
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let result = wsl_service::run_in_wsl(&distro, &argv_refs)
        .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;

    let output = if result.stderr.is_empty() {
        result.stdout
    } else {
        format!("{}\nstderr: {}", result.stdout, result.stderr)
    };
    Ok(output.trim().to_string())
}

/// Scan Windows named pipes and return those that look SSH/agent-related
#[tauri::command]
pub fn scan_windows_named_pipes(
    state: State<'_, AppState>,
) -> Result<Vec<NamedPipeEntry>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(provider_health::scan_named_pipes())
}

/// Serialize the current bridge config to pretty-printed JSON (no secrets)
#[tauri::command]
pub fn export_bridge_config(
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    serde_json::to_string_pretty(&*config)
        .map_err(|e| MazeSshError::BridgeError(format!("Serialization error: {}", e)))
}

/// Merge an imported bridge config JSON into the current config.
/// Returns the number of distros imported/updated. Does NOT restart relays.
#[tauri::command]
pub fn import_bridge_config(
    json: String,
    state: State<'_, AppState>,
) -> Result<usize, MazeSshError> {
    ensure_unlocked(&state)?;

    let imported: BridgeConfig = serde_json::from_str(&json)
        .map_err(|e| MazeSshError::BridgeError(format!("Invalid bridge config JSON: {}", e)))?;

    for d in &imported.distros {
        if d.distro_name.trim().is_empty() {
            return Err(MazeSshError::BridgeError(
                "Imported config contains a distro with an empty name".to_string(),
            ));
        }
    }

    let count = imported.distros.len();
    let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;

    for imported_distro in imported.distros {
        if let Some(existing) = config.distros.iter_mut().find(|d| d.distro_name == imported_distro.distro_name) {
            *existing = imported_distro;
        } else {
            config.distros.push(imported_distro);
        }
    }

    bridge_service::save_bridge_config(&config)?;
    Ok(count)
}

/// Bootstrap the relay into all running WSL2 distros that don't have it installed yet.
/// Continues on per-distro failures; saves config once at the end.
#[tauri::command]
pub fn bootstrap_all_distros(
    state: State<'_, AppState>,
) -> Result<Vec<BootstrapAllResult>, MazeSshError> {
    ensure_unlocked(&state)?;

    let all_distros = wsl_service::list_distros()?;
    let candidates: Vec<_> = all_distros
        .into_iter()
        .filter(|d| d.version == 2 && d.state.eq_ignore_ascii_case("Running"))
        .collect();

    let mut results: Vec<BootstrapAllResult> = Vec::new();

    for distro_info in candidates {
        let distro = distro_info.name.clone();

        let relay_installed = wsl_service::wsl_file_exists(
            &distro,
            &format!("~/{}", crate::models::bridge::RELAY_SCRIPT_PATH),
        );
        if relay_installed {
            continue;
        }

        {
            let mut config = state.bridge.write().map_err(|_| MazeSshError::StateLockError)?;
            if !config.distros.iter().any(|d| d.distro_name == distro) {
                config.distros.push(DistroBridgeConfig {
                    distro_name: distro.clone(),
                    enabled: true,
                    socket_path: None,
                    provider: BridgeProvider::default(),
                    allow_agent_forwarding: false,
                    relay_mode: RelayMode::default(),
                    auto_restart: true,
                    max_restarts: 5,
                });
            } else if let Some(d) = config.distros.iter_mut().find(|d| d.distro_name == distro) {
                d.enabled = true;
            }
        }

        let bootstrap_result = {
            let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
            bridge_service::bootstrap_distro(&distro, &config)
        };

        match bootstrap_result {
            Ok(_) => {
                log_bridge_audit("bootstrap_all", &distro, "", "Bridge setup completed");
                results.push(BootstrapAllResult { distro, success: true, error: None });
            }
            Err(e) => {
                results.push(BootstrapAllResult { distro, success: false, error: Some(e.to_string()) });
            }
        }
    }

    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    bridge_service::save_bridge_config(&config)?;

    Ok(results)
}

// ── Phase 7 commands ──

/// Rewrite the relay script from the current config and restart the relay service in-place.
/// No teardown required — use this after changing socket path without re-bootstrapping.
#[tauri::command]
pub fn refresh_relay_script(
    distro: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    bridge_service::refresh_relay_script(&distro, &config)?;
    log_bridge_audit("relay_refresh", &distro, "", "Relay script refreshed in-place");
    Ok(())
}

/// Return the current Maze SSH injection block for each known shell RC file.
#[tauri::command]
pub fn get_shell_injections(
    distro: String,
    state: State<'_, AppState>,
) -> Result<Vec<ShellInjection>, MazeSshError> {
    ensure_unlocked(&state)?;
    Ok(bridge_service::get_shell_injections(&distro))
}

/// Surgically remove the Maze SSH injection block from one shell RC file.
/// The rc_file must be one of the known allowlisted paths.
#[tauri::command]
pub fn remove_shell_injection(
    distro: String,
    rc_file: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    bridge_service::remove_single_shell_injection(&distro, &rc_file)?;
    log_bridge_audit("remove_shell_injection", &distro, "", &format!("Removed injection from {}", rc_file));
    Ok(())
}

/// Run an end-to-end SSH connectivity test through the bridge socket.
/// user and host are passed separately (split on '@' in the frontend).
#[tauri::command]
pub fn test_ssh_via_bridge(
    distro: String,
    host: String,
    user: String,
    port: u16,
    state: State<'_, AppState>,
) -> Result<SshHostTestResult, MazeSshError> {
    ensure_unlocked(&state)?;
    let config = state.bridge.read().map_err(|_| MazeSshError::StateLockError)?;
    bridge_service::test_ssh_via_bridge(&distro, &config, &host, &user, port)
}
