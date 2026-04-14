use serde::{Deserialize, Serialize};

// ── Persisted config (stored at ~/.maze-ssh/bridge.json) ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BridgeConfig {
    /// Per-distro bridge settings
    pub distros: Vec<DistroBridgeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroBridgeConfig {
    pub distro_name: String,
    pub enabled: bool,
    /// Override socket path inside WSL (default: /tmp/maze-ssh-agent.sock)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,
}

// ── Runtime types (returned to frontend) ──

/// A WSL distribution detected via `wsl -l -v`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslDistro {
    pub name: String,
    /// "Running" or "Stopped"
    pub state: String,
    /// 1 or 2
    pub version: u8,
    pub is_default: bool,
}

/// Detailed bridge status for one WSL distro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroBridgeStatus {
    pub distro_name: String,
    pub wsl_version: u8,
    pub distro_running: bool,
    pub enabled: bool,
    /// Relay script + systemd unit exist in distro
    pub relay_installed: bool,
    /// systemctl --user is-active reports "active"
    pub service_active: bool,
    /// Unix socket file present
    pub socket_exists: bool,
    /// ssh-add -l succeeds through the bridged socket
    pub agent_reachable: bool,
    /// socat binary available in distro
    pub socat_installed: bool,
    /// systemd --user functional in distro
    pub systemd_available: bool,
    /// Last error encountered during checks, if any
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Full bridge overview for the frontend dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeOverview {
    pub wsl_available: bool,
    pub npiperelay_installed: bool,
    pub windows_agent_running: bool,
    pub distros: Vec<DistroBridgeStatus>,
}

// ── Constants ──

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/maze-ssh-agent.sock";
pub const RELAY_SCRIPT_PATH: &str = ".local/bin/maze-ssh-relay.sh";
pub const SYSTEMD_UNIT_PATH: &str = ".config/systemd/user/maze-ssh-relay.service";
pub const BRIDGE_MARKER_BEGIN: &str = "# >>> maze-ssh-bridge >>>";
pub const BRIDGE_MARKER_END: &str = "# <<< maze-ssh-bridge <<<";
