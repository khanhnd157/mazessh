use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::services::profile_service;
use crate::services::ssh_engine::hidden_cmd;
use crate::services::wsl_service;

// ── Config persistence ──

fn bridge_config_path() -> PathBuf {
    profile_service::data_dir().join("bridge.json")
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

// ── npiperelay management ──

/// Default path for npiperelay.exe on the Windows filesystem
pub fn npiperelay_path() -> PathBuf {
    profile_service::data_dir().join("bin").join("npiperelay.exe")
}

pub fn is_npiperelay_installed() -> bool {
    npiperelay_path().exists()
}

/// Convert a Windows path to the WSL /mnt/c/... equivalent
fn windows_path_to_wsl(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    // Convert  C:\Users\foo\...  →  /mnt/c/Users/foo/...
    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        let drive = (s.as_bytes()[0] as char).to_ascii_lowercase();
        let rest = s[2..].replace('\\', "/");
        format!("/mnt/{}{}", drive, rest)
    } else {
        s.replace('\\', "/")
    }
}

/// Get the WSL-visible path to npiperelay.exe
fn npiperelay_wsl_path() -> String {
    windows_path_to_wsl(&npiperelay_path())
}

// ── Windows agent check ──

/// Check if the Windows OpenSSH Authentication Agent service is running
pub fn is_windows_agent_running() -> bool {
    let output = hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", "(Get-Service ssh-agent).Status"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.trim().eq_ignore_ascii_case("Running")
        }
        Err(_) => false,
    }
}

// ── Bootstrap / teardown ──

fn resolve_socket_path(config: &BridgeConfig, distro: &str) -> String {
    config
        .distros
        .iter()
        .find(|d| d.distro_name == distro)
        .and_then(|d| d.socket_path.clone())
        .unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string())
}

fn generate_relay_script(npiperelay_wsl: &str, socket_path: &str) -> String {
    format!(
        r#"#!/bin/bash
# Maze SSH Agent Relay — DO NOT EDIT (managed by Maze SSH)
SOCKET="{socket_path}"
NPIPERELAY="{npiperelay_wsl}"

# Clean up stale socket
rm -f "$SOCKET"

# Bridge: socat listens on Unix socket, pipes to npiperelay which talks to Windows named pipe
exec socat UNIX-LISTEN:"$SOCKET",fork,mode=0600 \
  EXEC:"$NPIPERELAY -ei -s //./pipe/openssh-ssh-agent",nofork
"#
    )
}

fn generate_systemd_unit(socket_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Maze SSH Agent Relay (Windows OpenSSH -> WSL)
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
        relay_script = RELAY_SCRIPT_PATH,
        socket_path = socket_path,
    )
}

fn generate_bashrc_block(socket_path: &str) -> String {
    format!(
        "{begin}\nexport SSH_AUTH_SOCK=\"{socket_path}\"\n{end}\n",
        begin = BRIDGE_MARKER_BEGIN,
        socket_path = socket_path,
        end = BRIDGE_MARKER_END,
    )
}

/// Bootstrap the bridge relay into a WSL distro.
///
/// Steps:
/// 1. Verify prerequisites (npiperelay, WSL2, socat, systemd)
/// 2. Write relay script + systemd unit
/// 3. Enable + start the service
/// 4. Configure SSH_AUTH_SOCK in shell profiles
pub fn bootstrap_distro(
    distro: &str,
    config: &BridgeConfig,
) -> Result<DistroBridgeStatus, MazeSshError> {
    // 1. Verify npiperelay exists
    if !is_npiperelay_installed() {
        return Err(MazeSshError::BridgeError(format!(
            "npiperelay.exe not found at {}. Place the binary there or use the bundled copy.",
            npiperelay_path().display()
        )));
    }

    // 2. Verify distro is WSL2 and running
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

    // 3. Check socat
    if !wsl_service::has_socat(distro) {
        return Err(MazeSshError::BridgeError(
            "socat is not installed in this distro. Install with: sudo apt install socat".to_string(),
        ));
    }

    // 4. Check systemd
    if !wsl_service::has_systemd(distro) {
        return Err(MazeSshError::BridgeError(
            "systemd is required but not available. Add [boot]\\nsystemd=true to /etc/wsl.conf and restart WSL.".to_string(),
        ));
    }

    let socket_path = resolve_socket_path(config, distro);
    let npiperelay_wsl = npiperelay_wsl_path();

    // 5. Create directories
    let _ = wsl_service::run_in_wsl(distro, &["mkdir", "-p", "~/.local/bin", "~/.config/systemd/user"]);

    // 6. Write relay script
    let relay_content = generate_relay_script(&npiperelay_wsl, &socket_path);
    wsl_service::wsl_write_file(distro, &format!("~/{}", RELAY_SCRIPT_PATH), &relay_content)?;

    // Make executable
    let _ = wsl_service::run_in_wsl(distro, &["chmod", "+x", &format!("~/{}", RELAY_SCRIPT_PATH)]);

    // 7. Write systemd unit
    let unit_content = generate_systemd_unit(&socket_path);
    wsl_service::wsl_write_file(distro, &format!("~/{}", SYSTEMD_UNIT_PATH), &unit_content)?;

    // 8. Reload + enable + start
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

    // 9. Configure SSH_AUTH_SOCK in bashrc (idempotent)
    configure_shell_env(distro, &socket_path)?;

    // Brief pause for service to create socket
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Return current status
    Ok(get_distro_status(distro, config))
}

/// Remove the bridge from a WSL distro
pub fn teardown_distro(distro: &str) -> Result<(), MazeSshError> {
    // Stop + disable service
    let _ = wsl_service::run_in_wsl(
        distro,
        &["systemctl", "--user", "disable", "--now", "maze-ssh-relay.service"],
    );

    // Remove files
    let _ = wsl_service::run_in_wsl(
        distro,
        &["rm", "-f", &format!("~/{}", RELAY_SCRIPT_PATH), &format!("~/{}", SYSTEMD_UNIT_PATH)],
    );

    // Reload systemd
    let _ = wsl_service::run_in_wsl(distro, &["systemctl", "--user", "daemon-reload"]);

    // Remove SSH_AUTH_SOCK from bashrc
    remove_shell_env(distro)?;

    Ok(())
}

// ── Service lifecycle ──

pub fn start_relay(distro: &str) -> Result<(), MazeSshError> {
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
    Ok(())
}

pub fn stop_relay(distro: &str) -> Result<(), MazeSshError> {
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
    Ok(())
}

pub fn restart_relay(distro: &str) -> Result<(), MazeSshError> {
    let result = wsl_service::run_in_wsl(
        distro,
        &["systemctl", "--user", "restart", "maze-ssh-relay.service"],
    )?;
    if !result.success {
        return Err(MazeSshError::BridgeError(format!(
            "Failed to restart relay: {}",
            result.stderr.trim()
        )));
    }
    Ok(())
}

// ── Health checks ──

/// Get full bridge status for a single distro
pub fn get_distro_status(distro: &str, config: &BridgeConfig) -> DistroBridgeStatus {
    let socket_path = resolve_socket_path(config, distro);

    // Check if distro is WSL2 and running
    let (wsl_version, distro_running) = match wsl_service::list_distros() {
        Ok(distros) => match distros.iter().find(|d| d.name == distro) {
            Some(d) => (d.version, d.state == "Running"),
            None => (0, false),
        },
        Err(_) => (0, false),
    };

    let enabled = config.distros.iter().any(|d| d.distro_name == distro && d.enabled);

    // Short-circuit if distro isn't running
    if !distro_running {
        return DistroBridgeStatus {
            distro_name: distro.to_string(),
            wsl_version,
            distro_running: false,
            enabled,
            relay_installed: false,
            service_active: false,
            socket_exists: false,
            agent_reachable: false,
            socat_installed: false,
            systemd_available: false,
            error: Some("Distro is not running".to_string()),
        };
    }

    let socat_installed = wsl_service::has_socat(distro);
    let systemd_available = wsl_service::has_systemd(distro);

    let relay_installed = wsl_service::wsl_file_exists(distro, &format!("~/{}", RELAY_SCRIPT_PATH))
        && wsl_service::wsl_file_exists(distro, &format!("~/{}", SYSTEMD_UNIT_PATH));

    let service_active = wsl_service::run_in_wsl(
        distro,
        &["systemctl", "--user", "is-active", "maze-ssh-relay.service"],
    )
    .map(|o| o.stdout.trim().to_string() == "active")
    .unwrap_or(false);

    let socket_exists = wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
        .map(|o| o.success)
        .unwrap_or(false);

    // ssh-add -l: exit 0 = keys listed, exit 1 = agent reachable but no keys, exit 2 = unreachable
    let agent_reachable = if socket_exists {
        wsl_service::run_in_wsl(
            distro,
            &["env", &format!("SSH_AUTH_SOCK={}", socket_path), "ssh-add", "-l"],
        )
        .map(|o| {
            // Exit code 0 (keys) or 1 (no keys but agent responding) = reachable
            o.success || o.stderr.contains("no identities")
                || o.stdout.contains("no identities")
                // Also check raw exit status via output content patterns
                || !o.stderr.contains("Error connecting")
                    && !o.stderr.contains("Could not open")
        })
        .unwrap_or(false)
    } else {
        false
    };

    let error = if !socat_installed {
        Some("socat not installed".to_string())
    } else if !systemd_available {
        Some("systemd not available".to_string())
    } else if relay_installed && !service_active {
        Some("Service installed but not active".to_string())
    } else if service_active && !socket_exists {
        Some("Service active but socket not found".to_string())
    } else if socket_exists && !agent_reachable {
        Some("Socket exists but agent unreachable — Windows agent may be stopped".to_string())
    } else {
        None
    };

    DistroBridgeStatus {
        distro_name: distro.to_string(),
        wsl_version,
        distro_running,
        enabled,
        relay_installed,
        service_active,
        socket_exists,
        agent_reachable,
        socat_installed,
        systemd_available,
        error,
    }
}

/// Get full bridge overview across all WSL2 distros
pub fn get_bridge_overview(config: &BridgeConfig) -> BridgeOverview {
    let wsl_available = wsl_service::is_wsl_available();

    if !wsl_available {
        return BridgeOverview {
            wsl_available: false,
            npiperelay_installed: is_npiperelay_installed(),
            windows_agent_running: is_windows_agent_running(),
            distros: Vec::new(),
        };
    }

    let npiperelay_installed = is_npiperelay_installed();
    let windows_agent_running = is_windows_agent_running();

    let distros = match wsl_service::list_distros() {
        Ok(all) => all
            .iter()
            .filter(|d| d.version == 2)
            .map(|d| get_distro_status(&d.name, config))
            .collect(),
        Err(_) => Vec::new(),
    };

    BridgeOverview {
        wsl_available,
        npiperelay_installed,
        windows_agent_running,
        distros,
    }
}

// ── Shell env management ──

/// Configure SSH_AUTH_SOCK in ~/.bashrc and ~/.profile (idempotent, marker-based)
fn configure_shell_env(distro: &str, socket_path: &str) -> Result<(), MazeSshError> {
    let block = generate_bashrc_block(socket_path);

    for rc_file in &["~/.bashrc", "~/.profile"] {
        // Read current content
        let current = wsl_service::run_in_wsl(distro, &["cat", rc_file])
            .map(|o| o.stdout)
            .unwrap_or_default();

        // Remove existing block if present
        let cleaned = remove_marker_block(&current);

        // Append new block
        let new_content = format!("{}\n{}", cleaned.trim_end(), block);
        wsl_service::wsl_write_file(distro, rc_file, &new_content)?;
    }

    Ok(())
}

/// Remove the SSH_AUTH_SOCK block from shell profiles
fn remove_shell_env(distro: &str) -> Result<(), MazeSshError> {
    for rc_file in &["~/.bashrc", "~/.profile"] {
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

/// Remove content between (and including) bridge markers
fn remove_marker_block(content: &str) -> String {
    let mut result = String::new();
    let mut inside_block = false;

    for line in content.lines() {
        if line.trim() == BRIDGE_MARKER_BEGIN {
            inside_block = true;
            continue;
        }
        if line.trim() == BRIDGE_MARKER_END {
            inside_block = false;
            continue;
        }
        if !inside_block {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
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
    fn test_generate_relay_script() {
        let script = generate_relay_script("/mnt/c/Users/test/.maze-ssh/bin/npiperelay.exe", "/tmp/maze-ssh-agent.sock");
        assert!(script.contains("socat UNIX-LISTEN"));
        assert!(script.contains("npiperelay.exe"));
        assert!(script.contains("/tmp/maze-ssh-agent.sock"));
    }
}
