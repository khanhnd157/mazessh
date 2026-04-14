use serde::{Deserialize, Serialize};

/// Identifies which SSH agent provider to bridge into WSL.
/// Stored in bridge.json per-distro. Defaults to WindowsOpenSsh for backward compat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum BridgeProvider {
    /// Windows OpenSSH Agent — named pipe at \\.\pipe\openssh-ssh-agent
    WindowsOpenSsh,
    /// 1Password SSH Agent — named pipe at \\.\pipe\op-ssh-sign-pipe
    OnePassword,
    /// Pageant-compatible agent (PuTTY, KeeAgent, GPG4Win) — WM_COPYDATA protocol
    Pageant,
}

impl Default for BridgeProvider {
    fn default() -> Self {
        BridgeProvider::WindowsOpenSsh
    }
}

impl BridgeProvider {
    /// Human-readable display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            BridgeProvider::WindowsOpenSsh => "Windows OpenSSH",
            BridgeProvider::OnePassword => "1Password",
            BridgeProvider::Pageant => "Pageant",
        }
    }

    /// The Windows named pipe path this provider uses (None for non-pipe providers)
    pub fn named_pipe(&self) -> Option<&'static str> {
        match self {
            BridgeProvider::WindowsOpenSsh => Some("//./pipe/openssh-ssh-agent"),
            BridgeProvider::OnePassword => Some("//./pipe/op-ssh-sign-pipe"),
            BridgeProvider::Pageant => None,
        }
    }

    /// Which relay binary this provider needs
    pub fn relay_binary(&self) -> RelayBinary {
        match self {
            BridgeProvider::WindowsOpenSsh | BridgeProvider::OnePassword => RelayBinary::Npiperelay,
            BridgeProvider::Pageant => RelayBinary::WslSshPageant,
        }
    }

    /// Whether this provider requires socat in WSL
    pub fn needs_socat(&self) -> bool {
        match self {
            BridgeProvider::WindowsOpenSsh | BridgeProvider::OnePassword => true,
            BridgeProvider::Pageant => false, // wsl-ssh-pageant creates the socket itself
        }
    }

    /// Description for the systemd service unit
    pub fn service_description(&self) -> String {
        format!("Maze SSH Agent Relay ({} -> WSL)", self.display_name())
    }
}

/// Which external binary is used to relay the agent protocol into WSL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayBinary {
    /// npiperelay.exe — bridges Windows named pipes to stdio (for OpenSSH, 1Password)
    Npiperelay,
    /// wsl-ssh-pageant.exe — converts Pageant protocol to Unix socket
    WslSshPageant,
}

impl RelayBinary {
    pub fn filename(&self) -> &'static str {
        match self {
            RelayBinary::Npiperelay => "npiperelay.exe",
            RelayBinary::WslSshPageant => "wsl-ssh-pageant.exe",
        }
    }

    /// All known relay binaries
    pub fn all() -> &'static [RelayBinary] {
        &[RelayBinary::Npiperelay, RelayBinary::WslSshPageant]
    }
}

/// Health/availability status for a specific provider on the Windows side
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: BridgeProvider,
    pub display_name: String,
    pub available: bool,
    /// Why unavailable, if applicable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Installation status for a relay binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayBinaryStatus {
    pub binary: RelayBinary,
    pub installed: bool,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_default_is_windows_openssh() {
        assert_eq!(BridgeProvider::default(), BridgeProvider::WindowsOpenSsh);
    }

    #[test]
    fn test_serde_roundtrip() {
        let provider = BridgeProvider::OnePassword;
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("one-password"));
        let parsed: BridgeProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BridgeProvider::OnePassword);
    }

    #[test]
    fn test_serde_default_deserialization() {
        // Simulates an old config entry without provider field
        #[derive(Deserialize)]
        struct TestConfig {
            name: String,
            #[serde(default)]
            provider: BridgeProvider,
        }
        let json = r#"{"name": "Ubuntu"}"#;
        let config: TestConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.provider, BridgeProvider::WindowsOpenSsh);
    }

    #[test]
    fn test_pipe_providers_need_socat() {
        assert!(BridgeProvider::WindowsOpenSsh.needs_socat());
        assert!(BridgeProvider::OnePassword.needs_socat());
        assert!(!BridgeProvider::Pageant.needs_socat());
    }

    #[test]
    fn test_pageant_uses_wsl_ssh_pageant() {
        assert_eq!(BridgeProvider::Pageant.relay_binary(), RelayBinary::WslSshPageant);
        assert_eq!(BridgeProvider::WindowsOpenSsh.relay_binary(), RelayBinary::Npiperelay);
        assert_eq!(BridgeProvider::OnePassword.relay_binary(), RelayBinary::Npiperelay);
    }
}
