use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::bridge::*;
use crate::models::bridge_provider::*;
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

// ── Relay binary management ──

/// Path for a relay binary on the Windows filesystem (~/.maze-ssh/bin/{filename})
pub fn relay_binary_path(binary: RelayBinary) -> PathBuf {
    profile_service::data_dir().join("bin").join(binary.filename())
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
    nohup ~/.local/bin/maze-ssh-relay.sh &>/dev/null &
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
                &["bash", "-c", &format!("nohup ~/.local/bin/maze-ssh-relay.sh &>/dev/null & sleep 0.5")],
            );
        }
    }

    // Configure SSH_AUTH_SOCK in bashrc (idempotent)
    configure_shell_env(distro, &socket_path, &relay_mode)?;

    // Brief pause for service to create socket
    std::thread::sleep(std::time::Duration::from_millis(500));

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
                &["bash", "-c", "nohup ~/.local/bin/maze-ssh-relay.sh &>/dev/null &"],
            )?;
            if !result.success {
                return Err(MazeSshError::BridgeError(format!(
                    "Failed to start relay daemon: {}",
                    result.stderr.trim()
                )));
            }
        }
    }
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

    let allow_agent_forwarding = config
        .distros
        .iter()
        .find(|d| d.distro_name == distro)
        .map(|d| d.allow_agent_forwarding)
        .unwrap_or(false);

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
            error: Some("Distro is not running".to_string()),
        };
    }

    let socat_installed = wsl_service::has_socat(distro);
    let systemd_available = wsl_service::has_systemd(distro);

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

fn configure_shell_env(distro: &str, socket_path: &str, relay_mode: &RelayMode) -> Result<(), MazeSshError> {
    let block = generate_bashrc_block(socket_path, relay_mode);

    for rc_file in &["~/.bashrc", "~/.profile"] {
        let current = wsl_service::run_in_wsl(distro, &["cat", rc_file])
            .map(|o| o.stdout)
            .unwrap_or_default();
        let cleaned = remove_marker_block(&current);
        let new_content = format!("{}\n{}", cleaned.trim_end(), block);
        wsl_service::wsl_write_file(distro, rc_file, &new_content)?;
    }

    Ok(())
}

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

    let mut steps: Vec<DiagnosticsStep> = Vec::new();
    let mut first_fail: Option<usize> = None;

    let mut add_step = |name: &str, passed: bool, detail: Option<String>| {
        if !passed && first_fail.is_none() {
            first_fail = Some(steps.len());
        }
        steps.push(DiagnosticsStep {
            name: name.to_string(),
            passed,
            detail,
        });
    };

    // Step 1: Provider reachable
    let ps = provider_health::check_provider(&provider);
    add_step("Provider reachable", ps.available, ps.error.clone());

    // Step 2: Relay binary installed
    let binary_ok = is_relay_binary_installed(relay_binary);
    let binary_detail = if !binary_ok {
        Some(format!("Expected at: {}", relay_binary_path(relay_binary).display()))
    } else {
        None
    };
    add_step("Relay binary installed", binary_ok, binary_detail);

    // Step 3: Distro running
    let distro_running = wsl_service::run_in_wsl(distro, &["echo", "ok"])
        .map(|o| o.stdout.trim() == "ok")
        .unwrap_or(false);
    add_step("Distro running", distro_running, None);

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
    add_step(mode_label, service_active, None);

    // Step 5: Socket exists
    let socket_ok = wsl_service::run_in_wsl(distro, &["test", "-S", &socket_path])
        .map(|o| o.success)
        .unwrap_or(false);
    add_step("Unix socket exists", socket_ok, Some(socket_path.clone()));

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

/// Generic marker-block removal helper
fn remove_block_between(content: &str, begin: &str, end: &str) -> String {
    let mut result = String::new();
    let mut inside_block = false;

    for line in content.lines() {
        if line.trim() == begin {
            inside_block = true;
            continue;
        }
        if line.trim() == end {
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
