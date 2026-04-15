use serde::Serialize;

use crate::models::bridge_provider::{BridgeProvider, ProviderStatus};
use crate::services::ssh_engine::hidden_cmd;

/// A Windows named pipe that may host an SSH agent
#[derive(Debug, Clone, Serialize)]
pub struct NamedPipeEntry {
    /// Full Windows path, e.g. `\\.\pipe\openssh-ssh-agent`
    pub path: String,
    /// Bare pipe name, e.g. `openssh-ssh-agent`
    pub display: String,
}

/// Enumerate named pipes on Windows and return those that look SSH/agent-related.
///
/// Uses PowerShell `Get-ChildItem \\.\pipe\` and filters entries whose name
/// contains "ssh", "agent", or "key" (case-insensitive).
pub fn scan_named_pipes() -> Vec<NamedPipeEntry> {
    let ps = r#"Get-ChildItem -Path '\\.\pipe\' | Select-Object -ExpandProperty Name | ConvertTo-Json -Compress"#;

    let output = hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", ps])
        .output();

    let raw = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return Vec::new(),
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // ConvertTo-Json returns a JSON string (single item) or JSON array
    let names: Vec<String> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).unwrap_or_default()
    } else if trimmed.starts_with('"') {
        // Single pipe → bare string
        serde_json::from_str::<String>(trimmed)
            .map(|s| vec![s])
            .unwrap_or_default()
    } else {
        return Vec::new();
    };

    let keywords = ["ssh", "agent", "key"];
    let mut entries: Vec<NamedPipeEntry> = names
        .into_iter()
        .filter(|name| {
            let lower = name.to_lowercase();
            keywords.iter().any(|kw| lower.contains(kw))
        })
        .map(|name| NamedPipeEntry {
            path: format!(r"\\.\pipe\{}", name),
            display: name,
        })
        .collect();

    entries.sort_by(|a, b| a.display.cmp(&b.display));
    entries
}

/// Check all built-in providers and return their Windows-side availability
pub fn check_all_providers() -> Vec<ProviderStatus> {
    vec![
        check_provider(&BridgeProvider::WindowsOpenSsh),
        check_provider(&BridgeProvider::OnePassword),
        check_provider(&BridgeProvider::Pageant),
    ]
}

/// Check if a specific provider's agent is available on Windows
pub fn check_provider(provider: &BridgeProvider) -> ProviderStatus {
    match provider {
        BridgeProvider::WindowsOpenSsh => check_windows_openssh(),
        BridgeProvider::OnePassword => check_onepassword(),
        BridgeProvider::Pageant => check_pageant(),
        BridgeProvider::Custom { ref pipe_path } => check_custom_pipe(pipe_path),
    }
}

/// Score and return the best available provider.
/// Priority: availability (required) > security tier > default preference.
pub fn recommend_provider(statuses: &[ProviderStatus]) -> Option<BridgeProvider> {
    let mut candidates: Vec<_> = statuses
        .iter()
        .filter(|s| s.available)
        .map(|s| (s, s.provider.recommendation_score()))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.first().map(|(s, _)| s.provider.clone())
}

/// Windows OpenSSH: check if ssh-agent service is running
fn check_windows_openssh() -> ProviderStatus {
    let available = hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", "(Get-Service ssh-agent).Status"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("Running")
        })
        .unwrap_or(false);

    ProviderStatus {
        provider: BridgeProvider::WindowsOpenSsh,
        display_name: "Windows OpenSSH".to_string(),
        available,
        error: if available {
            None
        } else {
            Some("ssh-agent service not running".to_string())
        },
    }
}

/// 1Password: check if process is running AND the SSH agent named pipe exists
fn check_onepassword() -> ProviderStatus {
    let process_running = hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "$null -ne (Get-Process '1Password' -ErrorAction SilentlyContinue)",
        ])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("True")
        })
        .unwrap_or(false);

    let pipe_exists = hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Test-Path \\\\.\\pipe\\op-ssh-sign-pipe",
        ])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("True")
        })
        .unwrap_or(false);

    let available = process_running && pipe_exists;
    let error = if !process_running {
        Some("1Password is not running".to_string())
    } else if !pipe_exists {
        Some(
            "1Password SSH agent pipe not found — enable 'Use the SSH agent' in 1Password settings"
                .to_string(),
        )
    } else {
        None
    };

    ProviderStatus {
        provider: BridgeProvider::OnePassword,
        display_name: "1Password".to_string(),
        available,
        error,
    }
}

/// Pageant: check if a Pageant-compatible window exists using FindWindow via PowerShell P/Invoke
fn check_pageant() -> ProviderStatus {
    let ps_script = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class PageantCheck {
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
}
"@
$hwnd = [PageantCheck]::FindWindow("Pageant", "Pageant")
$hwnd -ne [IntPtr]::Zero
"#;

    let available = hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", ps_script])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("True")
        })
        .unwrap_or(false);

    ProviderStatus {
        provider: BridgeProvider::Pageant,
        display_name: "Pageant".to_string(),
        available,
        error: if available {
            None
        } else {
            Some("No Pageant-compatible agent detected (PuTTY, KeeAgent, or GPG4Win)".to_string())
        },
    }
}

/// Custom: verify the user-defined named pipe exists
fn check_custom_pipe(pipe_path: &str) -> ProviderStatus {
    if pipe_path.is_empty() {
        return ProviderStatus {
            provider: BridgeProvider::Custom { pipe_path: pipe_path.to_string() },
            display_name: "Custom".to_string(),
            available: false,
            error: Some("No pipe path configured".to_string()),
        };
    }

    // Convert //./pipe/xxx to \\.\pipe\xxx for PowerShell Test-Path
    let ps_path = pipe_path.replace("//./pipe/", r"\\.\pipe\");
    let cmd_str = format!("Test-Path '{}'", ps_path);

    let available = hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", &cmd_str])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("True")
        })
        .unwrap_or(false);

    ProviderStatus {
        provider: BridgeProvider::Custom { pipe_path: pipe_path.to_string() },
        display_name: "Custom".to_string(),
        available,
        error: if available {
            None
        } else {
            Some(format!("Pipe not found: {}", pipe_path))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend_provider_picks_highest_score() {
        let statuses = vec![
            ProviderStatus {
                provider: BridgeProvider::WindowsOpenSsh,
                display_name: "OpenSSH".into(),
                available: true,
                error: None,
            },
            ProviderStatus {
                provider: BridgeProvider::OnePassword,
                display_name: "1Password".into(),
                available: true,
                error: None,
            },
            ProviderStatus {
                provider: BridgeProvider::Pageant,
                display_name: "Pageant".into(),
                available: true,
                error: None,
            },
        ];
        let rec = recommend_provider(&statuses);
        assert_eq!(rec, Some(BridgeProvider::OnePassword));
    }

    #[test]
    fn test_recommend_provider_skips_unavailable() {
        let statuses = vec![
            ProviderStatus {
                provider: BridgeProvider::OnePassword,
                display_name: "1Password".into(),
                available: false,
                error: Some("not running".into()),
            },
            ProviderStatus {
                provider: BridgeProvider::WindowsOpenSsh,
                display_name: "OpenSSH".into(),
                available: true,
                error: None,
            },
        ];
        let rec = recommend_provider(&statuses);
        assert_eq!(rec, Some(BridgeProvider::WindowsOpenSsh));
    }

    #[test]
    fn test_recommend_provider_none_available() {
        let statuses = vec![
            ProviderStatus {
                provider: BridgeProvider::WindowsOpenSsh,
                display_name: "OpenSSH".into(),
                available: false,
                error: Some("down".into()),
            },
        ];
        assert_eq!(recommend_provider(&statuses), None);
    }
}
