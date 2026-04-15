use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::models::bridge_provider::*;
use crate::services::bridge_history_service as history;
use crate::services::profile_service;
use crate::services::provider_health;
use crate::services::wsl_service;

fn resolve_relay_mode(config: &BridgeConfig, distro: &str) -> RelayMode {
    config
        .distros
        .iter()
        .find(|d| d.distro_name == distro)
        .map(|d| d.relay_mode.clone())
        .unwrap_or_default()
}

// ── Config persistence ──

fn bridge_config_path() -> PathBuf {
    profile_service::data_dir()
        .unwrap_or_else(|_| PathBuf::from(".maze-ssh"))
        .join("bridge.json")
}

pub fn load_bridge_config() -> BridgeConfig {
    let path = bridge_config_path();
    if !path.exists() {
        return BridgeConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => BridgeConfig::default(),
    }
}

pub fn save_bridge_config(config: &BridgeConfig) -> Result<(), MazeSshError> {
    let path = bridge_config_path();
    let content = serde_json::to_string_pretty(config)?;
    profile_service::atomic_write(&path, &content)?;
    Ok(())
}

// ── Relay binary management ──

/// Path for a relay binary on the Windows filesystem (~/.maze-ssh/bin/{filename})
pub fn relay_binary_path(binary: RelayBinary) -> PathBuf {
    profile_service::data_dir()
        .unwrap_or_else(|_| PathBuf::from(".maze-ssh"))
        .join("bin")
        .join(binary.filename())
}

pub fn is_relay_binary_installed(binary: RelayBinary) -> bool {
    relay_binary_path(binary).exists()
}

/// Backward compat wrapper
pub fn npiperelay_path() -> PathBuf {
    relay_binary_path(RelayBinary::Npiperelay)
}

/// Backward compat wrapper
pub fn is_npiperelay_installed() -> bool {
    is_relay_binary_installed(RelayBinary::Npiperelay)
}

/// Convert a Windows path to the WSL /mnt/c/... equivalent
fn windows_path_to_wsl(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        let drive = (s.as_bytes()[0] as char).to_ascii_lowercase();
        let rest = s[2..].replace('\\', "/");
        format!("/mnt/{}{}", drive, rest)
    } else {
        s.replace('\\', "/")
    }
}

/// Get the WSL-visible path for a relay binary
fn relay_binary_wsl_path(binary: RelayBinary) -> String {
    windows_path_to_wsl(&relay_binary_path(binary))
}

/// Check all relay binaries and return their status
fn check_relay_binaries() -> Vec<RelayBinaryStatus> {
    RelayBinary::all()
        .iter()
        .map(|b| RelayBinaryStatus {
            binary: *b,
            installed: is_relay_binary_installed(*b),
            path: relay_binary_path(*b).to_string_lossy().to_string(),
        })
        .collect()
}

// ── Windows agent check (backward compat) ──

pub fn is_windows_agent_running() -> bool {
    provider_health::check_provider(&BridgeProvider::WindowsOpenSsh).available
}

// ── Bootstrap / teardown ──

fn resolve_provider(config: &BridgeConfig, distro: &str) -> BridgeProvider {
    config
        .distros
        .iter()
        .find(|d| d.distro_name == distro)
        .map(|d| d.provider.clone())
        .unwrap_or_default()
}

fn resolve_socket_path(config: &BridgeConfig, distro: &str) -> String {
    config
        .distros
        .iter()
        .find(|d| d.distro_name == distro)
        .and_then(|d| d.socket_path.clone())
        .unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string())
}

/// Generate the relay script, dispatched by provider type
fn generate_relay_script(
    provider: &BridgeProvider,
    relay_binary_wsl: &str,
    socket_path: &str,
) -> String {
    match provider {
        BridgeProvider::WindowsOpenSsh
        | BridgeProvider::OnePassword
        | BridgeProvider::Custom { .. } => {
            let pipe = provider.named_pipe().unwrap_or_default();
            format!(
                r#"#!/bin/bash
# Maze SSH Agent Relay ({name}) — DO NOT EDIT (managed by Maze SSH)
SOCKET="{socket_path}"
RELAY="{relay_binary_wsl}"

# Clean up stale socket
rm -f "$SOCKET"

# Bridge: socat listens on Unix socket, pipes to relay which talks to Windows named pipe
exec socat UNIX-LISTEN:"$SOCKET",fork,mode=0600 \
  EXEC:"$RELAY -ei -s {pipe}",nofork
"#,
                name = provider.display_name(),
            )
        }
        BridgeProvider::Pageant => {
            // wsl-ssh-pageant creates the socket itself, no socat needed
            format!(
                r#"#!/bin/bash
# Maze SSH Agent Relay ({name}) — DO NOT EDIT (managed by Maze SSH)
SOCKET="{socket_path}"
PAGEANT="{relay_binary_wsl}"

# Clean up stale socket
rm -f "$SOCKET"

# Bridge: wsl-ssh-pageant connects to Pageant and exposes a Unix socket
exec "$PAGEANT" --wsl "$SOCKET"
"#,
                name = provider.display_name(),
            )
        }
    }
}

fn generate_systemd_unit(provider: &BridgeProvider, socket_path: &str) -> String {
    format!(
        r#"[Unit]
Description={description}
After=default.target

[Service]
Type=simple
ExecStart=%h/{relay_script}
Restart=on-failure
RestartSec=3
Environment=MAZE_SSH_SOCKET={socket_path}

[Install]
WantedBy=default.target
"#,
        description = provider.service_description(),
        relay_script = RELAY_SCRIPT_PATH,
        socket_path = socket_path,
    )
}

fn generate_bashrc_block(socket_path: &str, relay_mode: &RelayMode) -> String {
    match relay_mode {
        RelayMode::Systemd => format!(
            "{begin}\nexport SSH_AUTH_SOCK=\"{socket_path}\"\n{end}\n",
            begin = BRIDGE_MARKER_BEGIN,
            socket_path = socket_path,
            end = BRIDGE_MARKER_END,
        ),
        RelayMode::Daemon => format!(
            r#"{begin}
export MAZE_SSH_SOCKET="{socket_path}"
if [ ! -S "$MAZE_SSH_SOCKET" ]; then
    nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &
    sleep 0.5
fi
export SSH_AUTH_SOCK="$MAZE_SSH_SOCKET"
{end}
"#,
            begin = BRIDGE_MARKER_BEGIN,
            socket_path = socket_path,
            end = BRIDGE_MARKER_END,
        ),
    }
}

/// Bootstrap the bridge relay into a WSL distro.
pub fn bootstrap_distro(
    distro: &str,
    config: &BridgeConfig,
) -> Result<DistroBridgeStatus, MazeSshError> {
    let provider = resolve_provider(config, distro);
    let relay_binary = provider.relay_binary();
    let relay_mode = resolve_relay_mode(config, distro);

    // 1. Verify relay binary exists
    if !is_relay_binary_installed(relay_binary) {
        return Err(MazeSshError::BridgeError(format!(
            "{} not found at {}. Place the binary there.",
            relay_binary.filename(),
            relay_binary_path(relay_binary).display()
        )));
    }

    // 2. Verify provider is available on Windows
    let provider_status = provider_health::check_provider(&provider);
    if !provider_status.available {
        return Err(MazeSshError::BridgeError(format!(
            "{} agent is not available: {}",
            provider.display_name(),
            provider_status.error.unwrap_or_default()
        )));
    }

    // 3. Verify distro is WSL2 and running
    let distros = wsl_service::list_distros()?;
    let wsl_distro = distros
        .iter()
        .find(|d| d.name == distro)
        .ok_or_else(|| MazeSshError::BridgeError(format!("WSL distro '{}' not found", distro)))?;

    if wsl_distro.version != 2 {
        return Err(MazeSshError::BridgeError(format!(
            "Only WSL2 distros are supported. '{}' is WSL{}. Convert with: wsl --set-version {} 2",
            distro, wsl_distro.version, distro
        )));
    }

    // Wake the distro if stopped
    if wsl_distro.state != "Running" {
        let _ = wsl_service::run_in_wsl(distro, &["echo", "ok"]);
    }

    // 4. Check socat (only for pipe-based providers)
    if provider.needs_socat() && !wsl_service::has_socat(distro) {
        return Err(MazeSshError::BridgeError(
            "socat is not installed in this distro. Install with: sudo apt install socat".to_string(),
        ));
    }

    let socket_path = resolve_socket_path(config, distro);
    let relay_wsl = relay_binary_wsl_path(relay_binary);

    match relay_mode {
        RelayMode::Systemd => {
            // 5. Check systemd
            if !wsl_service::has_systemd(distro) {
                return Err(MazeSshError::BridgeError(
                    "systemd is required but not available. Add [boot]\\nsystemd=true to /etc/wsl.conf and restart WSL.".to_string(),
                ));
            }

            // 6. Create directories
            let _ = wsl_service::run_in_wsl(distro, &["mkdir", "-p", "~/.local/bin", "~/.config/systemd/user"]);

            // 7. Write relay script
            let relay_content = generate_relay_script(&provider, &relay_wsl, &socket_path);
            wsl_service::wsl_write_file(distro, &format!("~/{}", RELAY_SCRIPT_PATH), &relay_content)?;
            let _ = wsl_service::run_in_wsl(distro, &["chmod", "+x", &format!("~/{}", RELAY_SCRIPT_PATH)]);

            // 8. Write systemd unit
            let unit_content = generate_systemd_unit(&provider, &socket_path);
            wsl_service::wsl_write_file(distro, &format!("~/{}", SYSTEMD_UNIT_PATH), &unit_content)?;

            // 9. Reload + enable + start
            let _ = wsl_service::run_in_wsl(distro, &["systemctl", "--user", "daemon-reload"]);
            let enable_result = wsl_service::run_in_wsl(
                distro,
                &["systemctl", "--user", "enable", "--now", "maze-ssh-relay.service"],
            )?;

            if !enable_result.success {
                return Err(MazeSshError::BridgeError(format!(
                    "Failed to enable/start service: {}",
                    enable_result.stderr.trim()
                )));
            }
        }
        RelayMode::Daemon => {
            // No systemd required — relay starts from .bashrc

            // 5. Create only ~/.local/bin
            let _ = wsl_service::run_in_wsl(distro, &["mkdir", "-p", "~/.local/bin"]);

            // 6. Write relay script
            let relay_content = generate_relay_script(&provider, &relay_wsl, &socket_path);
            wsl_service::wsl_write_file(distro, &format!("~/{}", RELAY_SCRIPT_PATH), &relay_content)?;
            let _ = wsl_service::run_in_wsl(distro, &["chmod", "+x", &format!("~/{}", RELAY_SCRIPT_PATH)]);

            // 7. Launch relay immediately in background
            let _ = wsl_service::run_in_wsl(
                distro,
                &["bash", "-c", r#"nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null & sleep 0.5"#],
            );
        }
    }

    // Configure SSH_AUTH_SOCK in bashrc (idempotent)
    configure_shell_env(distro, &socket_path, &relay_mode)?;

    // Brief pause for service to create socket
    std::thread::sleep(std::time::Duration::from_millis(500));

    history::append_event(distro, BridgeHistoryEventKind::BridgeBootstrapped, Some(provider.display_name().to_string()));
    Ok(get_distro_status(distro, config))
}

/// Remove the bridge from a WSL distro
pub fn teardown_distro(distro: &str, config: &BridgeConfig) -> Result<(), MazeSshError> {
    let relay_mode = resolve_relay_mode(config, distro);
    match relay_mode {
        RelayMode::Systemd => {
            let _ = wsl_service::run_in_wsl(
                distro,
                &["systemctl", "--user", "disable", "--now", "maze-ssh-relay.service"],
            );
            let _ = wsl_service::run_in_wsl(distro, &["systemctl", "--user", "daemon-reload"]);
        }
        RelayMode::Daemon => {
            // Kill the background relay process if running
            let _ = wsl_service::run_in_wsl(
                distro,
                &["bash", "-c", "pkill -f maze-ssh-relay.sh || true"],
            );
        }
    }
    let _ = wsl_service::run_in_wsl(
        distro,
        &["rm", "-f", &format!("~/{}", RELAY_SCRIPT_PATH), &format!("~/{}", SYSTEMD_UNIT_PATH)],
    );
    remove_shell_env(distro)?;
    history::append_event(distro, BridgeHistoryEventKind::BridgeTeardown, None);
    Ok(())
}

// ── Service lifecycle ──

pub fn start_relay(distro: &str, relay_mode: &RelayMode) -> Result<(), MazeSshError> {
    match relay_mode {
        RelayMode::Systemd => {
            let result = wsl_service::run_in_wsl(
                distro,
                &["systemctl", "--user", "start", "maze-ssh-relay.service"],
            )?;
            if !result.success {
                return Err(MazeSshError::BridgeError(format!(
                    "Failed to start relay: {}",
                    result.stderr.trim()
                )));
            }
        }
        RelayMode::Daemon => {
            // Kill any stale process first, then launch fresh
            let _ = wsl_service::run_in_wsl(
                distro,
                &["bash", "-c", "pkill -f maze-ssh-relay.sh || true"],
            );
            std::thread::sleep(std::time::Duration::from_millis(200));
            let result = wsl_service::run_in_wsl(
                distro,
                &["bash", "-c", r#"nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &"#],
            )?;
            if !result.success {
                return Err(MazeSshError::BridgeError(format!(
                    "Failed to start relay daemon: {}",
                    result.stderr.trim()
                )));
            }
        }
    }
    history::append_event(distro, BridgeHistoryEventKind::BridgeStarted, None);
    Ok(())
}

pub fn stop_relay(distro: &str, relay_mode: &RelayMode) -> Result<(), MazeSshError> {
    match relay_mode {
        RelayMode::Systemd => {
            let result = wsl_service::run_in_wsl(
                distro,
                &["systemctl", "--user", "stop", "maze-ssh-relay.service"],
            )?;
            if !result.success {
                return Err(MazeSshError::BridgeError(format!(
                    "Failed to stop relay: {}",
                    result.stderr.trim()
                )));
            }
        }
        RelayMode::Daemon => {
            let _ = wsl_service::run_in_wsl(
                distro,
                &["bash", "-c", "pkill -f maze-ssh-relay.sh || true"],
            );
        }
    }
    history::append_event(distro, BridgeHistoryEventKind::BridgeStopped, None);
    Ok(())
}

pub fn restart_relay(distro: &str, relay_mode: &RelayMode) -> Result<(), MazeSshError> {
    stop_relay(distro, relay_mode)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    start_relay(distro, relay_mode)?;
    Ok(())
}

// ── Health checks ──

/// Get full bridge status for a single distro
pub fn get_distro_status(distro: &str, config: &BridgeConfig) -> DistroBridgeStatus {
    let socket_path = resolve_socket_path(config, distro);
    let provider = resolve_provider(config, distro);
    let relay_mode = resolve_relay_mode(config, distro);

    let (wsl_version, distro_running) = match wsl_service::list_distros() {
        Ok(distros) => match distros.iter().find(|d| d.name == distro) {
            Some(d) => (d.version, d.state == "Running"),
            None => (0, false),
        },
        Err(_) => (0, false),
    };

    let enabled = config.distros.iter().any(|d| d.distro_name == distro && d.enabled);

    let distro_config = config.distros.iter().find(|d| d.distro_name == distro);
    let allow_agent_forwarding = distro_config.map(|d| d.allow_agent_forwarding).unwrap_or(false);
    let auto_restart = distro_config.map(|d| d.auto_restart).unwrap_or(true);
    let max_restarts = distro_config.map(|d| d.max_restarts).unwrap_or(5);
    // watchdog_restart_count comes from in-memory watchdog state; we pass 0 here since
    // get_distro_status doesn't have access to AppState. Commands that call this function
    // can optionally overlay this value from state.relay_watchdog_state.
    let watchdog_restart_count = 0u8;

    if !distro_running {
        return DistroBridgeStatus {
            distro_name: distro.to_string(),
            wsl_version,
            distro_running: false,
            enabled,
            provider: provider.clone(),
            relay_installed: false,
            service_active: false,
            socket_exists: false,
            agent_reachable: false,
            allow_agent_forwarding,
            socat_installed: false,
            systemd_available: false,
            relay_mode,
            auto_restart,
            watchdog_restart_count,
            relay_script_stale: false,
            max_restarts,
            detected_shells: Vec::new(),
            socket_path: socket_path.clone(),
            error: Some("Distro is not running".to_string()),
        };
    }

    let socat_installed = wsl_service::has_socat(distro);
    let systemd_available = wsl_service::has_systemd(distro);
    let detected_shells = wsl_service::detect_shells(distro);

    // relay_installed: script exists + (systemd unit exists OR daemon mode)
    let script_installed = wsl_service::wsl_file_exists(distro, &format!("~/{}", RELAY_SCRIPT_PATH));
    let relay_installed = match relay_mode {
        RelayMode::Systemd => script_installed && wsl_service::wsl_file_exists(distro, &format!("~/{}", SYSTEMD_UNIT_PATH)),
        RelayMode::Daemon => script_installed,
    };

    // service_active: systemd check OR socket exists (daemon mode)
    let service_active = match relay_mode {
        RelayMode::Systemd => wsl_service::run_in_wsl(
            distro,
            &["systemctl", "--user", "is-active", "maze-ssh-relay.service"],
        )
        .map(|o| o.stdout.trim().to_string() == "active")
        .unwrap_or(false),
        RelayMode::Daemon => {
            // In daemon mode, "active" means the relay process is running (socket present)
            wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
                .map(|o| o.success)
                .unwrap_or(false)
        }
    };

    let socket_exists = wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
        .map(|o| o.success)
        .unwrap_or(false);

    let agent_reachable = if socket_exists {
        wsl_service::run_in_wsl(
            distro,
            &["env", &format!("SSH_AUTH_SOCK={}", socket_path), "ssh-add", "-l"],
        )
        .map(|o| {
            o.success
                || o.stderr.contains("no identities")
                || o.stdout.contains("no identities")
                || !o.stderr.contains("Error connecting")
                    && !o.stderr.contains("Could not open")
        })
        .unwrap_or(false)
    } else {
        false
    };

    let error = if provider.needs_socat() && !socat_installed {
        Some("socat not installed".to_string())
    } else if relay_mode == RelayMode::Systemd && !systemd_available {
        Some("systemd not available".to_string())
    } else if relay_installed && !service_active {
        Some("Service installed but not active".to_string())
    } else if service_active && !socket_exists {
        Some("Service active but socket not found".to_string())
    } else if socket_exists && !agent_reachable {
        Some("Socket exists but agent unreachable — agent may be stopped".to_string())
    } else {
        None
    };

    let relay_script_stale = if relay_installed {
        is_relay_script_stale(distro, config)
    } else {
        false
    };

    DistroBridgeStatus {
        distro_name: distro.to_string(),
        wsl_version,
        distro_running,
        enabled,
        provider,
        relay_installed,
        service_active,
        socket_exists,
        agent_reachable,
        allow_agent_forwarding,
        socat_installed,
        systemd_available,
        relay_mode,
        auto_restart,
        watchdog_restart_count,
        relay_script_stale,
        max_restarts,
        detected_shells,
        socket_path,
        error,
    }
}

/// Get full bridge overview across all WSL2 distros
pub fn get_bridge_overview(config: &BridgeConfig) -> BridgeOverview {
    let wsl_available = wsl_service::is_wsl_available();
    let npiperelay_installed = is_npiperelay_installed();
    let windows_agent_running = is_windows_agent_running();
    let provider_statuses = provider_health::check_all_providers();
    let relay_binaries = check_relay_binaries();

    let distros = if wsl_available {
        match wsl_service::list_distros() {
            Ok(all) => all
                .iter()
                .filter(|d| d.version == 2)
                .map(|d| get_distro_status(&d.name, config))
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    BridgeOverview {
        wsl_available,
        npiperelay_installed,
        windows_agent_running,
        provider_statuses,
        relay_binaries,
        distros,
    }
}

// ── Shell env management ──

fn generate_fish_block(socket_path: &str, relay_mode: &RelayMode) -> String {
    match relay_mode {
        RelayMode::Systemd => format!(
            "{begin}\nset -x SSH_AUTH_SOCK \"{socket_path}\"\n{end}\n",
            begin = BRIDGE_MARKER_BEGIN,
            socket_path = socket_path,
            end = BRIDGE_MARKER_END,
        ),
        RelayMode::Daemon => format!(
            r#"{begin}
set -x MAZE_SSH_SOCKET "{socket_path}"
if not test -S $MAZE_SSH_SOCKET
    nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &
    sleep 0.5
end
set -x SSH_AUTH_SOCK $MAZE_SSH_SOCKET
{end}
"#,
            begin = BRIDGE_MARKER_BEGIN,
            socket_path = socket_path,
            end = BRIDGE_MARKER_END,
        ),
    }
}

fn configure_shell_env(distro: &str, socket_path: &str, relay_mode: &RelayMode) -> Result<(), MazeSshError> {
    let bash_block = generate_bashrc_block(socket_path, relay_mode);
    let fish_block = generate_fish_block(socket_path, relay_mode);

    // Detect installed shells
    let shells = wsl_service::detect_shells(distro);

    for shell_profile in &shells {
        if !shell_profile.is_installed {
            continue;
        }

        let (rc_file, block) = if shell_profile.shell == "fish" {
            (shell_profile.rc_file.as_str(), fish_block.as_str())
        } else {
            (shell_profile.rc_file.as_str(), bash_block.as_str())
        };

        // Ensure parent directory exists for fish
        if shell_profile.shell == "fish" {
            let _ = wsl_service::run_in_wsl(distro, &["mkdir", "-p", "~/.config/fish"]);
        }

        let current = wsl_service::run_in_wsl(distro, &["cat", rc_file])
            .map(|o| o.stdout)
            .unwrap_or_default();
        let cleaned = remove_marker_block(&current);
        let new_content = format!("{}\n{}", cleaned.trim_end(), block);
        wsl_service::wsl_write_file(distro, rc_file, &new_content)?;
    }

    // Always write ~/.profile for login shells regardless of detected shells
    {
        let current = wsl_service::run_in_wsl(distro, &["cat", "~/.profile"])
            .map(|o| o.stdout)
            .unwrap_or_default();
        let cleaned = remove_marker_block(&current);
        let new_content = format!("{}\n{}", cleaned.trim_end(), bash_block);
        wsl_service::wsl_write_file(distro, "~/.profile", &new_content)?;
    }

    Ok(())
}

fn remove_shell_env(distro: &str) -> Result<(), MazeSshError> {
    let shells = wsl_service::detect_shells(distro);

    let mut rc_files: Vec<&str> = shells
        .iter()
        .filter(|s| s.is_installed)
        .map(|s| s.rc_file.as_str())
        .collect();

    // Always clean ~/.profile too
    rc_files.push("~/.profile");

    for rc_file in rc_files {
        let current = wsl_service::run_in_wsl(distro, &["cat", rc_file])
            .map(|o| o.stdout)
            .unwrap_or_default();

        if current.contains(BRIDGE_MARKER_BEGIN) {
            let cleaned = remove_marker_block(&current);
            wsl_service::wsl_write_file(distro, rc_file, &cleaned)?;
        }
    }
    Ok(())
}

// ── Agent forwarding management ──

/// Configure or remove ForwardAgent in ~/.ssh/config inside WSL (marker-based)
pub fn configure_agent_forwarding(distro: &str, enable: bool) -> Result<(), MazeSshError> {
    // Ensure ~/.ssh exists
    let _ = wsl_service::run_in_wsl(distro, &["mkdir", "-p", "~/.ssh"]);

    let current = wsl_service::run_in_wsl(distro, &["cat", "~/.ssh/config"])
        .map(|o| o.stdout)
        .unwrap_or_default();

    // Remove existing forwarding block
    let cleaned = remove_block_between(&current, FORWARD_MARKER_BEGIN, FORWARD_MARKER_END);

    if enable {
        let block = format!(
            "{}\nHost *\n  ForwardAgent yes\n{}\n",
            FORWARD_MARKER_BEGIN, FORWARD_MARKER_END
        );
        let new_content = format!("{}\n{}", cleaned.trim_end(), block);
        wsl_service::wsl_write_file(distro, "~/.ssh/config", &new_content)?;
    } else if current.contains(FORWARD_MARKER_BEGIN) {
        wsl_service::wsl_write_file(distro, "~/.ssh/config", &cleaned)?;
    }

    Ok(())
}

// ── Diagnostics ──

static STEP_SUGGESTIONS: &[&str] = &[
    "Start or enable your SSH agent provider (e.g. Windows OpenSSH service).",
    "Download the relay binary using the Download button in Prerequisites.",
    "Start your WSL distro: open a terminal and run `wsl -d <distro>`.",
    "Start the relay service: click Start, or re-run bootstrap.",
    "The Unix socket wasn't created. Check relay logs for errors.",
    "The agent socket exists but no keys are loaded. Add keys with `ssh-add` on Windows.",
];

/// Run a step-by-step bridge connectivity test for a distro.
pub fn run_diagnostics(distro: &str, config: &BridgeConfig) -> DiagnosticsResult {
    let provider = resolve_provider(config, distro);
    let relay_mode = resolve_relay_mode(config, distro);
    let socket_path = resolve_socket_path(config, distro);
    let relay_binary = provider.relay_binary();
    let relay_installed = wsl_service::wsl_file_exists(distro, &format!("~/{}", crate::models::bridge::RELAY_SCRIPT_PATH));

    let mut steps: Vec<DiagnosticsStep> = Vec::new();
    let mut first_fail: Option<usize> = None;

    let mut add_step = |name: &str, passed: bool, detail: Option<String>, remediation: Option<String>| {
        if !passed && first_fail.is_none() {
            first_fail = Some(steps.len());
        }
        steps.push(DiagnosticsStep {
            name: name.to_string(),
            passed,
            detail,
            remediation_cmd: if passed { None } else { remediation },
        });
    };

    // Step 1: Provider reachable
    let ps = provider_health::check_provider(&provider);
    add_step("Provider reachable", ps.available, ps.error.clone(), None);

    // Step 2: Relay binary installed
    let binary_ok = is_relay_binary_installed(relay_binary);
    let binary_detail = if !binary_ok {
        Some(format!("Expected at: {}", relay_binary_path(relay_binary).display()))
    } else {
        None
    };
    add_step("Relay binary installed", binary_ok, binary_detail, None);

    // Step 3: Distro running
    let distro_running = wsl_service::run_in_wsl(distro, &["echo", "ok"])
        .map(|o| o.stdout.trim() == "ok")
        .unwrap_or(false);
    add_step("Distro running", distro_running, None, None);

    // Step 4: Service / relay active
    let service_active = match relay_mode {
        RelayMode::Systemd => wsl_service::run_in_wsl(
            distro,
            &["systemctl", "--user", "is-active", "maze-ssh-relay.service"],
        )
        .map(|o| o.stdout.trim() == "active")
        .unwrap_or(false),
        RelayMode::Daemon => wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
            .map(|o| o.success)
            .unwrap_or(false),
    };
    let mode_label = match relay_mode {
        RelayMode::Systemd => "Relay service (systemd)",
        RelayMode::Daemon => "Relay process (daemon)",
    };
    let service_remediation = match relay_mode {
        RelayMode::Systemd => Some("systemctl --user start maze-ssh-relay.service".to_string()),
        RelayMode::Daemon => Some(r#"nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &"#.to_string()),
    };
    add_step(mode_label, service_active, None, service_remediation);

    // Step 5: Socket exists
    let socket_ok = wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
        .map(|o| o.success)
        .unwrap_or(false);
    // If socket missing but service is installed, suggest removing stale socket and restarting
    let socket_remediation = if relay_installed && !socket_ok {
        Some(format!("rm -f {} && systemctl --user restart maze-ssh-relay.service", socket_path))
    } else {
        None
    };
    add_step("Unix socket exists", socket_ok, Some(socket_path.clone()), socket_remediation);

    // Step 6: Keys visible
    let keys_visible: Vec<String> = if socket_ok {
        wsl_service::run_in_wsl(
            distro,
            &["env", &format!("SSH_AUTH_SOCK={}", socket_path), "ssh-add", "-l"],
        )
        .map(|o| {
            let out = if o.stdout.trim().is_empty() { &o.stderr } else { &o.stdout };
            out.lines()
                .filter(|l| !l.trim().is_empty() && !l.contains("The agent has no identities"))
                .map(|l| l.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    let keys_count = keys_visible.len();
    add_step(
        "Keys visible through bridge",
        keys_count > 0,
        Some(format!("{} key(s) accessible", keys_count)),
        None,
    );

    let suggestions = match first_fail {
        Some(i) => vec![STEP_SUGGESTIONS[i.min(STEP_SUGGESTIONS.len() - 1)].to_string()],
        None => Vec::new(),
    };

    DiagnosticsResult {
        distro: distro.to_string(),
        steps,
        keys_visible,
        suggestions,
    }
}

/// Fetch the last N lines of the relay service journal inside WSL.
pub fn get_relay_logs(distro: &str, lines: u32) -> Result<String, MazeSshError> {
    let n = lines.min(200).to_string();
    let result = wsl_service::run_in_wsl(
        distro,
        &["bash", "-c", &format!("journalctl --user -u maze-ssh-relay --no-pager -n {} 2>&1 || echo '(journalctl not available)'", n)],
    )?;
    Ok(if result.stdout.trim().is_empty() {
        result.stderr
    } else {
        result.stdout
    })
}

// ── Relay watchdog ──

/// Poll all enabled distros and restart any relay that unexpectedly stopped.
/// Called every 30 seconds from the background timer in lib.rs.
///
/// Safety: On first poll (distro not in watchdog_state) we record the current state
/// WITHOUT restarting — this prevents false restarts right after app launch.
#[cfg(feature = "desktop")]
pub async fn poll_and_restart_relays(app: &tauri::AppHandle) {
    use tauri::Emitter;
    use tauri::Manager;

    let state = app.state::<crate::state::AppState>();

    // Skip while app is locked
    if let Ok(security) = state.security.lock() {
        if security.is_locked {
            return;
        }
    }

    let config = match state.bridge.read() {
        Ok(c) => c.clone(),
        Err(_) => return,
    };

    for distro_cfg in &config.distros {
        if !distro_cfg.enabled || !distro_cfg.auto_restart {
            continue;
        }

        let distro = distro_cfg.distro_name.clone();

        // get_distro_status is blocking — run in blocking thread
        let distro_clone = distro.clone();
        let config_clone = config.clone();
        let status = tokio::task::spawn_blocking(move || {
            get_distro_status(&distro_clone, &config_clone)
        })
        .await;

        let status = match status {
            Ok(s) => s,
            Err(_) => continue,
        };

        let currently_active = status.service_active;

        // Resolve max_restarts from config (default 5 if not found)
        let max_restarts = config.distros.iter()
            .find(|d| d.distro_name == distro)
            .map(|d| d.max_restarts)
            .unwrap_or(5);

        // Determine action without holding the guard across any await.
        // Drop the guard before any .await to avoid Send issues with MutexGuard.
        enum WatchdogAction {
            Init,          // First poll — record state, do nothing
            AlreadyPaused, // Hit max_restarts, skip and notify
            Restart,       // Was active, now dead, under retry cap
            Update,  // Just update was_active (no restart needed)
        }

        let action = {
            use crate::state::WatchdogEntry;
            let mut watchdog = match state.relay_watchdog_state.lock() {
                Ok(w) => w,
                Err(_) => continue,
            };

            match watchdog.get_mut(&distro) {
                None => {
                    // First poll — record current state, no restart
                    watchdog.insert(distro.clone(), WatchdogEntry {
                        was_active: currently_active,
                        restart_count: 0,
                        last_restart_at: None,
                    });
                    WatchdogAction::Init
                }
                Some(entry) => {
                    // Reset restart count when relay comes back healthy
                    if currently_active {
                        entry.restart_count = 0;
                    }

                    if entry.was_active && !currently_active {
                        if entry.restart_count >= max_restarts {
                            WatchdogAction::AlreadyPaused
                        } else {
                            entry.restart_count += 1;
                            entry.last_restart_at = Some(std::time::Instant::now());
                            WatchdogAction::Restart
                        }
                    } else {
                        entry.was_active = currently_active;
                        WatchdogAction::Update
                    }
                }
            }
            // Guard dropped here — before any await
        };

        match action {
            WatchdogAction::Init | WatchdogAction::Update => {
                // Nothing to do
            }
            WatchdogAction::AlreadyPaused => {
                history::append_event(
                    &distro,
                    BridgeHistoryEventKind::WatchdogPaused,
                    Some(format!("after {} attempts", max_restarts)),
                );
                // Emit paused event so in-app UI can show the badge
                let _ = app.emit("relay-restart-failed", serde_json::json!({
                    "distro": distro,
                    "count": max_restarts,
                }));
                // Emit native OS notification for users with the app minimized to tray
                #[cfg(feature = "desktop")]
                {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app.notification()
                        .builder()
                        .title("Maze SSH: Bridge Relay Stopped")
                        .body(&format!(
                            "Auto-restart paused for {} after {} attempts. Open Maze SSH to investigate.",
                            distro, max_restarts
                        ))
                        .show();
                }
            }
            WatchdogAction::Restart => {
                let distro_clone = distro.clone();
                let config_clone = config.clone();

                let relay_mode = config_clone
                    .distros
                    .iter()
                    .find(|d| d.distro_name == distro_clone)
                    .map(|d| d.relay_mode.clone())
                    .unwrap_or_default();

                let restarted = tokio::task::spawn_blocking(move || {
                    restart_relay(&distro_clone, &relay_mode)
                })
                .await;

                if restarted.map(|r| r.is_ok()).unwrap_or(false) {
                    let count = {
                        state.relay_watchdog_state.lock()
                            .ok()
                            .and_then(|w| w.get(&distro).map(|e| e.restart_count))
                            .unwrap_or(0)
                    };
                    history::append_event(
                        &distro,
                        BridgeHistoryEventKind::WatchdogRestart,
                        Some(format!("attempt {}", count)),
                    );
                    let _ = app.emit("relay-restarted", distro.clone());
                    // Update was_active to true on success
                    if let Ok(mut w) = state.relay_watchdog_state.lock() {
                        if let Some(entry) = w.get_mut(&distro) {
                            entry.was_active = true;
                        }
                    }
                }
            }
        }
    }
}

/// Generic marker-block removal helper.
///
/// Scans forward from each begin marker to confirm a matching end marker exists
/// before removing anything.  If no matching end is found the begin line is kept
/// as-is so that stray or hand-edited markers never silently discard user content.
fn remove_block_between(content: &str, begin: &str, end: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim() == begin {
            // Only strip the block when we can confirm a matching end exists
            if let Some(offset) = lines[i + 1..].iter().position(|l| l.trim() == end) {
                // Skip begin line, inner content, and end line
                i += offset + 2;
                continue;
            }
            // No matching end — preserve the begin line unchanged
        }
        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }

    result
}

// ── Phase 7: relay script drift, RC viewer, SSH host test ──

/// Compare the installed relay script against what the current config would generate.
/// Returns true if they differ (i.e. the script is stale).
fn is_relay_script_stale(distro: &str, config: &BridgeConfig) -> bool {
    let provider = resolve_provider(config, distro);
    let socket_path = resolve_socket_path(config, distro);
    let relay_binary = provider.relay_binary();
    let relay_wsl = relay_binary_wsl_path(relay_binary);

    let expected = generate_relay_script(&provider, &relay_wsl, &socket_path);

    let installed = wsl_service::run_in_wsl(distro, &["cat", &format!("~/{}", RELAY_SCRIPT_PATH)])
        .map(|o| if o.success { o.stdout } else { String::new() })
        .unwrap_or_default();

    // Normalize: trim each line, compare content
    let normalize = |s: &str| -> String {
        s.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n")
    };
    normalize(&expected) != normalize(&installed)
}

/// Rewrite the relay script from the current config and restart the service — no teardown needed.
pub fn refresh_relay_script(distro: &str, config: &BridgeConfig) -> Result<(), MazeSshError> {
    let provider = resolve_provider(config, distro);
    let socket_path = resolve_socket_path(config, distro);
    let relay_mode = resolve_relay_mode(config, distro);
    let relay_binary = provider.relay_binary();
    let relay_wsl = relay_binary_wsl_path(relay_binary);

    let script = generate_relay_script(&provider, &relay_wsl, &socket_path);
    let script_path = format!("~/{}", RELAY_SCRIPT_PATH);

    wsl_service::wsl_write_file(distro, &script_path, &script)?;
    wsl_service::run_in_wsl(distro, &["chmod", "+x", &script_path])
        .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;

    match relay_mode {
        RelayMode::Systemd => {
            wsl_service::run_in_wsl(distro, &["systemctl", "--user", "daemon-reload"])
                .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;
            wsl_service::run_in_wsl(distro, &["systemctl", "--user", "restart", "maze-ssh-relay.service"])
                .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;
        }
        RelayMode::Daemon => {
            // Kill existing process (ignore error if not running)
            let _ = wsl_service::run_in_wsl(distro, &["pkill", "-f", "maze-ssh-relay.sh"]);
            // Re-launch
            wsl_service::run_in_wsl(distro, &["bash", "-c",
                r#"nohup "$HOME"/.local/bin/maze-ssh-relay.sh &>/dev/null &"#])
                .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;
        }
    }

    history::append_event(distro, BridgeHistoryEventKind::RelayRefreshed, None);
    Ok(())
}

/// Return the current Maze SSH injection block for each known shell RC file.
pub fn get_shell_injections(distro: &str) -> Vec<crate::models::bridge::ShellInjection> {
    use crate::models::bridge::ShellInjection;

    let rc_files: &[(&str, &str)] = &[
        ("bash",    "~/.bashrc"),
        ("zsh",     "~/.zshrc"),
        ("fish",    "~/.config/fish/config.fish"),
        ("profile", "~/.profile"),
    ];

    let mut result = Vec::new();

    for (shell, rc_file) in rc_files {
        let content = wsl_service::run_in_wsl(distro, &["cat", rc_file])
            .map(|o| if o.success { Some(o.stdout) } else { None })
            .unwrap_or(None);

        let injected_block = content.as_deref().and_then(|s| {
            extract_marker_block(s, BRIDGE_MARKER_BEGIN, BRIDGE_MARKER_END)
        });

        result.push(ShellInjection {
            shell: shell.to_string(),
            rc_file: rc_file.to_string(),
            injected_block,
            has_forward_block: false,
        });
    }

    // Check ~/.ssh/config for ForwardAgent block
    let ssh_config_content = wsl_service::run_in_wsl(distro, &["cat", "~/.ssh/config"])
        .map(|o| if o.success { Some(o.stdout) } else { None })
        .unwrap_or(None);
    let has_forward_block = ssh_config_content
        .as_deref()
        .map(|s| s.contains(FORWARD_MARKER_BEGIN))
        .unwrap_or(false);

    // Attach has_forward_block to the profile entry (most relevant location)
    if let Some(entry) = result.iter_mut().find(|e| e.shell == "profile") {
        entry.has_forward_block = has_forward_block;
    }

    result
}

/// Extract the text between two markers (inclusive of the markers).
fn extract_marker_block(content: &str, begin: &str, end: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim() == begin {
            in_block = true;
        }
        if in_block {
            lines.push(line);
        }
        if in_block && line.trim() == end {
            return Some(lines.join("\n"));
        }
    }
    None
}

/// Remove the injection block from one RC file (allowlisted paths only).
pub fn remove_single_shell_injection(distro: &str, rc_file: &str) -> Result<(), MazeSshError> {
    const ALLOWED: &[&str] = &[
        "~/.bashrc",
        "~/.zshrc",
        "~/.profile",
        "~/.config/fish/config.fish",
    ];
    if !ALLOWED.contains(&rc_file) {
        return Err(MazeSshError::BridgeError(format!(
            "RC file not in allowlist: {}",
            rc_file
        )));
    }

    let content = wsl_service::run_in_wsl(distro, &["cat", rc_file])
        .map_err(|e| MazeSshError::BridgeError(e.to_string()))?
        .stdout;

    let cleaned = remove_marker_block(&content);
    wsl_service::wsl_write_file(distro, rc_file, &cleaned)?;
    Ok(())
}

/// Run a real SSH connection test through the bridged socket.
pub fn test_ssh_via_bridge(
    distro: &str,
    config: &BridgeConfig,
    host: &str,
    user: &str,
    port: u16,
) -> Result<crate::models::bridge::SshHostTestResult, MazeSshError> {
    use crate::models::bridge::SshHostTestResult;

    // Validate host: non-empty, only RFC-valid hostname/IP chars (no whitespace, no shell metacharacters)
    if host.is_empty() {
        return Err(MazeSshError::BridgeError("Host cannot be empty".to_string()));
    }
    if !host.chars().all(|c| c.is_alphanumeric() || "-._[]".contains(c)) {
        return Err(MazeSshError::BridgeError(
            "Host contains invalid characters (allowed: a-z A-Z 0-9 - . _ [ ])".to_string(),
        ));
    }

    // Validate user: alphanumeric + - . _ @ only, max 64 chars
    if user.is_empty() {
        return Err(MazeSshError::BridgeError("User cannot be empty".to_string()));
    }
    if user.len() > 64 {
        return Err(MazeSshError::BridgeError("User too long (max 64 chars)".to_string()));
    }
    if !user.chars().all(|c| c.is_alphanumeric() || "-._@".contains(c)) {
        return Err(MazeSshError::BridgeError(
            "User contains invalid characters".to_string(),
        ));
    }

    // port is u16 so 1–65535 is guaranteed by the type

    let socket_path = resolve_socket_path(config, distro);

    // Build argv — never use bash -c with user-supplied values to prevent shell injection
    let socket_env = format!("SSH_AUTH_SOCK={}", socket_path);
    let port_str = port.to_string();
    let destination = format!("{}@{}", user, host);
    let argv: Vec<&str> = vec![
        "env", &socket_env,
        "ssh", "-T",
        "-o", "StrictHostKeyChecking=accept-new",
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=8",
        "-p", &port_str,
        &destination,
    ];

    let output = wsl_service::run_in_wsl(distro, &argv)
        .map_err(|e| MazeSshError::BridgeError(e.to_string()))?;

    let combined = format!("{}{}", output.stdout, output.stderr);
    let lower = combined.to_lowercase();

    let connected = !lower.contains("connection refused")
        && !lower.contains("connection timed out")
        && !lower.contains("timed out")
        && !lower.contains("no route to host")
        && !lower.contains("could not resolve hostname");

    let authenticated = output.success
        || lower.contains("successfully authenticated")
        || lower.contains("pty allocation request failed")
        || lower.contains("welcome to")
        || lower.contains("you've successfully authenticated");

    let exit_code = if output.success { 0i32 } else { 1i32 };
    let display_cmd = format!(
        "env SSH_AUTH_SOCK=<socket> ssh -T -o StrictHostKeyChecking=accept-new -o BatchMode=yes -o ConnectTimeout=8 -p {} {}@{}",
        port, user, host
    );

    Ok(SshHostTestResult {
        command: display_cmd,
        output: combined.trim().to_string(),
        connected,
        authenticated,
        exit_code,
    })
}

fn remove_marker_block(content: &str) -> String {
    remove_block_between(content, BRIDGE_MARKER_BEGIN, BRIDGE_MARKER_END)
}

// ── Phase 8: Windows SSH config auto-population ──

/// Generate the Host block text for the Windows-side `~/.ssh/config`.
///
/// The alias is `maze-wsl-<distro>` (lowercase, spaces → hyphens).
/// The `ProxyCommand` uses `wsl -d <distro>` so Windows SSH clients can
/// connect through the WSL-hosted bridge socket.
fn generate_windows_wsl_host_block(distro: &str, config: &BridgeConfig) -> String {
    let socket_path = resolve_socket_path(config, distro);
    let alias = format!("maze-wsl-{}", distro.to_lowercase().replace(' ', "-"));
    let begin = format!("# >>> maze-ssh-wsl-{distro} >>>");
    let end = format!("# <<< maze-ssh-wsl-{distro} <<<");
    format!(
        "{begin}\nHost {alias}\n    ProxyCommand wsl -d {distro} -- socat - UNIX-CONNECT:{socket_path}\n    StrictHostKeyChecking accept-new\n{end}\n",
    )
}

/// Return a preview of the Windows SSH config Host block without writing it.
pub fn preview_windows_ssh_host(distro: &str, config: &BridgeConfig) -> String {
    generate_windows_wsl_host_block(distro, config)
}

/// Write (or refresh) the Host block in `~/.ssh/config` on the Windows host.
/// Uses marker-based idempotent injection — safe to call multiple times.
pub fn upsert_windows_ssh_host(distro: &str, config: &BridgeConfig) -> Result<(), MazeSshError> {
    let config_path = windows_ssh_config_path()?;
    if let Some(dir) = config_path.parent() {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let current = std::fs::read_to_string(&config_path).unwrap_or_default();
    let begin = format!("# >>> maze-ssh-wsl-{distro} >>>");
    let end = format!("# <<< maze-ssh-wsl-{distro} <<<");
    let cleaned = remove_block_between(&current, &begin, &end);
    let block = generate_windows_wsl_host_block(distro, config);
    let new_content = format!("{}\n{}", cleaned.trim_end(), block);
    profile_service::atomic_write(&config_path, &new_content)?;
    Ok(())
}

/// Remove the Host block from `~/.ssh/config` on the Windows host.
pub fn remove_windows_ssh_host(distro: &str) -> Result<(), MazeSshError> {
    let config_path = windows_ssh_config_path()?;
    if !config_path.exists() {
        return Ok(());
    }
    let current = std::fs::read_to_string(&config_path)?;
    let begin = format!("# >>> maze-ssh-wsl-{distro} >>>");
    let end = format!("# <<< maze-ssh-wsl-{distro} <<<");
    let cleaned = remove_block_between(&current, &begin, &end);
    profile_service::atomic_write(&config_path, &cleaned)?;
    Ok(())
}

fn windows_ssh_config_path() -> Result<std::path::PathBuf, MazeSshError> {
    let home = dirs::home_dir()
        .ok_or_else(|| MazeSshError::ConfigError("Home directory not found".to_string()))?;
    Ok(home.join(".ssh").join("config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_path_to_wsl() {
        assert_eq!(
            windows_path_to_wsl(std::path::Path::new(r"C:\Users\test\.maze-ssh\bin\npiperelay.exe")),
            "/mnt/c/Users/test/.maze-ssh/bin/npiperelay.exe"
        );
        assert_eq!(
            windows_path_to_wsl(std::path::Path::new(r"D:\some\path")),
            "/mnt/d/some/path"
        );
    }

    #[test]
    fn test_remove_marker_block() {
        let content = "line1\n# >>> maze-ssh-bridge >>>\nexport SSH_AUTH_SOCK=\"/tmp/test.sock\"\n# <<< maze-ssh-bridge <<<\nline2\n";
        let cleaned = remove_marker_block(content);
        assert!(!cleaned.contains("maze-ssh-bridge"));
        assert!(cleaned.contains("line1"));
        assert!(cleaned.contains("line2"));
    }

    #[test]
    fn test_remove_marker_block_no_markers() {
        let content = "line1\nline2\n";
        let cleaned = remove_marker_block(content);
        assert_eq!(cleaned, content);
    }

    #[test]
    fn test_remove_marker_block_orphaned_begin_preserved() {
        // A stray begin marker with no matching end must NOT silently eat the rest of the file
        let content = "line1\n# >>> maze-ssh-bridge >>>\nline2\nline3\n";
        let cleaned = remove_marker_block(content);
        // All content must be preserved — no data loss
        assert!(cleaned.contains("line1"));
        assert!(cleaned.contains("maze-ssh-bridge"));
        assert!(cleaned.contains("line2"));
        assert!(cleaned.contains("line3"));
    }

    #[test]
    fn test_remove_block_between_multiple_blocks() {
        // When two complete blocks exist, both should be removed
        let content = "a\n# >>> maze-ssh-bridge >>>\nblock1\n# <<< maze-ssh-bridge <<<\nb\n# >>> maze-ssh-bridge >>>\nblock2\n# <<< maze-ssh-bridge <<<\nc\n";
        let cleaned = remove_marker_block(content);
        assert!(cleaned.contains('a'));
        assert!(cleaned.contains('b'));
        assert!(cleaned.contains('c'));
        assert!(!cleaned.contains("block1"));
        assert!(!cleaned.contains("block2"));
    }

    #[test]
    fn test_generate_relay_script_openssh() {
        let script = generate_relay_script(
            &BridgeProvider::WindowsOpenSsh,
            "/mnt/c/Users/test/.maze-ssh/bin/npiperelay.exe",
            "/tmp/maze-ssh-agent.sock",
        );
        assert!(script.contains("socat UNIX-LISTEN"));
        assert!(script.contains("//./pipe/openssh-ssh-agent"));
        assert!(script.contains("/tmp/maze-ssh-agent.sock"));
    }

    #[test]
    fn test_generate_relay_script_onepassword() {
        let script = generate_relay_script(
            &BridgeProvider::OnePassword,
            "/mnt/c/Users/test/.maze-ssh/bin/npiperelay.exe",
            "/tmp/maze-ssh-agent.sock",
        );
        assert!(script.contains("socat UNIX-LISTEN"));
        assert!(script.contains("//./pipe/op-ssh-sign-pipe"));
        assert!(script.contains("1Password"));
    }

    #[test]
    fn test_generate_relay_script_pageant() {
        let script = generate_relay_script(
            &BridgeProvider::Pageant,
            "/mnt/c/Users/test/.maze-ssh/bin/wsl-ssh-pageant.exe",
            "/tmp/maze-ssh-agent.sock",
        );
        assert!(!script.contains("socat"));
        assert!(script.contains("wsl-ssh-pageant.exe"));
        assert!(script.contains("--wsl"));
        assert!(script.contains("Pageant"));
    }
}
