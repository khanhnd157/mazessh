use serde::{Deserialize, Serialize};

use super::bridge_provider::{BridgeProvider, ProviderStatus, RelayBinaryStatus};

// ── Relay mode ──

/// How the relay service is managed in WSL
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RelayMode {
    /// Managed via systemd --user (recommended, requires systemd in WSL)
    #[default]
    Systemd,
    /// Background daemon launched from .bashrc (no systemd required)
    Daemon,
}

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
    /// Which SSH agent provider to bridge. Defaults to WindowsOpenSsh for backward compat.
    #[serde(default)]
    pub provider: BridgeProvider,
    /// Allow SSH agent forwarding to remote hosts (default: false for security)
    #[serde(default)]
    pub allow_agent_forwarding: bool,
    /// How the relay service is managed (default: Systemd)
    #[serde(default)]
    pub relay_mode: RelayMode,
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
    /// Which provider this distro is configured to bridge
    pub provider: BridgeProvider,
    /// Relay script + systemd unit exist in distro
    pub relay_installed: bool,
    /// systemctl --user is-active reports "active"
    pub service_active: bool,
    /// Unix socket file present
    pub socket_exists: bool,
    /// ssh-add -l succeeds through the bridged socket
    pub agent_reachable: bool,
    /// Whether agent forwarding is enabled for this distro
    pub allow_agent_forwarding: bool,
    /// socat binary available in distro (only relevant for pipe-based providers)
    pub socat_installed: bool,
    /// systemd --user functional in distro
    pub systemd_available: bool,
    /// How the relay service is managed
    #[serde(default)]
    pub relay_mode: RelayMode,
    /// Last error encountered during checks, if any
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ── Diagnostics ──

/// Result of a step-by-step bridge connectivity test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsResult {
    pub distro: String,
    pub steps: Vec<DiagnosticsStep>,
    /// Fingerprint lines from `ssh-add -l` through the bridged socket
    pub keys_visible: Vec<String>,
    /// Human-readable remediation hints for the first failing step
    pub suggestions: Vec<String>,
}

/// One step in a bridge diagnostics run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsStep {
    pub name: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Full bridge overview for the frontend dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeOverview {
    pub wsl_available: bool,
    /// Backward compat: true if npiperelay.exe is installed
    pub npiperelay_installed: bool,
    /// Backward compat: true if Windows OpenSSH agent is running
    pub windows_agent_running: bool,
    /// Per-provider availability on the Windows side
    pub provider_statuses: Vec<ProviderStatus>,
    /// Which relay binaries are installed
    pub relay_binaries: Vec<RelayBinaryStatus>,
    pub distros: Vec<DistroBridgeStatus>,
}

// ── Constants ──

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/maze-ssh-agent.sock";
pub const RELAY_SCRIPT_PATH: &str = ".local/bin/maze-ssh-relay.sh";
pub const SYSTEMD_UNIT_PATH: &str = ".config/systemd/user/maze-ssh-relay.service";
pub const BRIDGE_MARKER_BEGIN: &str = "# >>> maze-ssh-bridge >>>";
pub const BRIDGE_MARKER_END: &str = "# <<< maze-ssh-bridge <<<";
pub const FORWARD_MARKER_BEGIN: &str = "# >>> maze-ssh-forward >>>";
pub const FORWARD_MARKER_END: &str = "# <<< maze-ssh-forward <<<";
