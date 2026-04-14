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
    /// User-defined provider with a custom Windows named pipe path
    Custom {
        #[serde(default)]
        pipe_path: String,
    },
}

impl Default for BridgeProvider {
    fn default() -> Self {
        BridgeProvider::WindowsOpenSsh
    }
}

impl BridgeProvider {
    /// Human-readable display name for UI
    pub fn display_name(&self) -> &str {
        match self {
            BridgeProvider::WindowsOpenSsh => "Windows OpenSSH",
            BridgeProvider::OnePassword => "1Password",
            BridgeProvider::Pageant => "Pageant",
            BridgeProvider::Custom { .. } => "Custom",
        }
    }

    /// The Windows named pipe path this provider uses (None for non-pipe providers like Pageant)
    pub fn named_pipe(&self) -> Option<String> {
        match self {
            BridgeProvider::WindowsOpenSsh => Some("//./pipe/openssh-ssh-agent".to_string()),
            BridgeProvider::OnePassword => Some("//./pipe/op-ssh-sign-pipe".to_string()),
            BridgeProvider::Pageant => None,
            BridgeProvider::Custom { pipe_path } => {
                if pipe_path.is_empty() {
                    None
                } else {
                    Some(pipe_path.clone())
                }
            }
        }
    }

    /// Which relay binary this provider needs
    pub fn relay_binary(&self) -> RelayBinary {
        match self {
            BridgeProvider::WindowsOpenSsh
            | BridgeProvider::OnePassword
            | BridgeProvider::Custom { .. } => RelayBinary::Npiperelay,
            BridgeProvider::Pageant => RelayBinary::WslSshPageant,
        }
    }

    /// Whether this provider requires socat in WSL
    pub fn needs_socat(&self) -> bool {
        match self {
            BridgeProvider::WindowsOpenSsh
            | BridgeProvider::OnePassword
            | BridgeProvider::Custom { .. } => true,
            BridgeProvider::Pageant => false,
        }
    }

    /// Description for the systemd service unit
    pub fn service_description(&self) -> String {
        format!("Maze SSH Agent Relay ({} -> WSL)", self.display_name())
    }

    /// Score for auto-recommendation (higher = better)
    pub fn recommendation_score(&self) -> u8 {
        match self {
            BridgeProvider::OnePassword => 3,
            BridgeProvider::WindowsOpenSsh => 2,
            BridgeProvider::Pageant => 1,
            BridgeProvider::Custom { .. } => 0,
        }
    }
}

/// Which external binary is used to relay the agent protocol into WSL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayBinary {
    /// npiperelay.exe — bridges Windows named pipes to stdio (for OpenSSH, 1Password, Custom)
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

    /// GitHub repo owner/name for downloading from releases
    pub fn github_repo(&self) -> &'static str {
        match self {
            RelayBinary::Npiperelay => "jstarks/npiperelay",
            RelayBinary::WslSshPageant => "benpye/wsl-ssh-pageant",
        }
    }

    /// Exact filename to look for in GitHub release assets
    pub fn asset_name(&self) -> &'static str {
        self.filename()
    }

    /// Key used in BinaryVersion JSON
    pub fn version_key(&self) -> &'static str {
        match self {
            RelayBinary::Npiperelay => "npiperelay",
            RelayBinary::WslSshPageant => "wsl_ssh_pageant",
        }
    }

    /// Parse from version_key string
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "npiperelay" => Some(RelayBinary::Npiperelay),
            "wsl-ssh-pageant" | "wsl_ssh_pageant" => Some(RelayBinary::WslSshPageant),
            _ => None,
        }
    }
}

/// Installed version record, persisted at ~/.maze-ssh/bin/bin-version.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BinaryVersion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npiperelay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl_ssh_pageant: Option<String>,
}

/// Progress event payload emitted during a binary download
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    /// "npiperelay" | "wsl-ssh-pageant"
    pub binary: String,
    /// 0–100
    pub percent: u8,
    /// "downloading" | "done" | "error"
    pub status: String,
}

/// Health/availability status for a specific provider on the Windows side
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: BridgeProvider,
    pub display_name: String,
    pub available: bool,
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
    fn test_serde_custom_roundtrip() {
        let provider = BridgeProvider::Custom {
            pipe_path: "//./pipe/my-agent".to_string(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("custom"));
        assert!(json.contains("my-agent"));
        let parsed: BridgeProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, provider);
    }

    #[test]
    fn test_serde_default_deserialization() {
        #[derive(Deserialize)]
        struct TestConfig {
            #[allow(dead_code)]
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
        assert!(BridgeProvider::Custom { pipe_path: "test".into() }.needs_socat());
    }

    #[test]
    fn test_relay_binaries() {
        assert_eq!(BridgeProvider::Pageant.relay_binary(), RelayBinary::WslSshPageant);
        assert_eq!(BridgeProvider::WindowsOpenSsh.relay_binary(), RelayBinary::Npiperelay);
        assert_eq!(BridgeProvider::OnePassword.relay_binary(), RelayBinary::Npiperelay);
        assert_eq!(BridgeProvider::Custom { pipe_path: "x".into() }.relay_binary(), RelayBinary::Npiperelay);
    }

    #[test]
    fn test_custom_named_pipe() {
        let p = BridgeProvider::Custom { pipe_path: "//./pipe/test".to_string() };
        assert_eq!(p.named_pipe(), Some("//./pipe/test".to_string()));

        let empty = BridgeProvider::Custom { pipe_path: String::new() };
        assert_eq!(empty.named_pipe(), None);
    }

    #[test]
    fn test_recommendation_scores() {
        assert!(BridgeProvider::OnePassword.recommendation_score() > BridgeProvider::WindowsOpenSsh.recommendation_score());
        assert!(BridgeProvider::WindowsOpenSsh.recommendation_score() > BridgeProvider::Pageant.recommendation_score());
        assert!(BridgeProvider::Pageant.recommendation_score() > BridgeProvider::Custom { pipe_path: "x".into() }.recommendation_score());
    }
}
