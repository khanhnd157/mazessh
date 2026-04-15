use std::io::Write;
use std::process::Stdio;

use serde::{Deserialize, Serialize};

use crate::error::MazeSshError;
use crate::models::bridge::WslDistro;
use crate::services::ssh_engine::hidden_cmd;

/// A shell detected inside a WSL distro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellProfile {
    pub shell: String,   // "bash" | "zsh" | "fish"
    pub rc_file: String, // "~/.bashrc" | "~/.zshrc" | "~/.config/fish/config.fish"
    pub is_installed: bool,
}

/// Detect which shells are installed in a WSL distro.
/// bash is always included (it's always present in WSL even if not in PATH).
pub fn detect_shells(distro: &str) -> Vec<ShellProfile> {
    let shells: &[(&str, &str)] = &[
        ("bash", "~/.bashrc"),
        ("zsh", "~/.zshrc"),
        ("fish", "~/.config/fish/config.fish"),
    ];

    shells
        .iter()
        .map(|(shell, rc_file)| {
            let is_installed = if *shell == "bash" {
                // bash is always present in WSL
                true
            } else {
                run_in_wsl(distro, &["which", shell])
                    .map(|o| o.success)
                    .unwrap_or(false)
            };
            ShellProfile {
                shell: shell.to_string(),
                rc_file: rc_file.to_string(),
                is_installed,
            }
        })
        .collect()
}

/// Output from a command executed inside WSL
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Timeout for WSL discovery commands (is_wsl_available, list_distros).
/// Shorter than WSL_CMD_TIMEOUT_SECS because these are host-level checks
/// that should complete instantly when WSL is healthy.
const WSL_DISCOVERY_TIMEOUT_SECS: u64 = 10;

/// Check if WSL is installed and functional.
/// Uses a polling loop with a hard timeout so a hung wsl.exe cannot
/// block the Tauri command thread indefinitely.
pub fn is_wsl_available() -> bool {
    let mut child = match hidden_cmd("wsl")
        .args(["--status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(WSL_DISCOVERY_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

/// List all WSL distributions by parsing `wsl -l -v`.
///
/// The output of `wsl -l -v` on Windows is UTF-16LE with BOM, formatted as:
/// ```text
///   NAME      STATE    VERSION
/// * Ubuntu    Running  2
///   Debian    Stopped  2
/// ```
pub fn list_distros() -> Result<Vec<WslDistro>, MazeSshError> {
    let mut child = hidden_cmd("wsl")
        .args(["-l", "-v"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| MazeSshError::WslNotAvailable(format!("Failed to spawn wsl: {}", e)))?;

    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(WSL_DISCOVERY_TIMEOUT_SECS);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(MazeSshError::WslNotAvailable(
                        "wsl -l -v returned non-zero exit code".to_string(),
                    ));
                }
                break;
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(MazeSshError::WslNotAvailable(
                        "wsl -l -v timed out".to_string(),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(MazeSshError::WslNotAvailable(format!(
                    "Failed to poll wsl: {}",
                    e
                )))
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| MazeSshError::WslNotAvailable(format!("Failed to collect output: {}", e)))?;

    let text = decode_wsl_output(&output.stdout);
    parse_distro_list(&text)
}

/// Decode the raw bytes from `wsl -l -v` which uses UTF-16LE with BOM on Windows
fn decode_wsl_output(raw: &[u8]) -> String {
    // Try UTF-16LE decode (Windows wsl.exe outputs this)
    if raw.len() >= 2 {
        // Check for UTF-16LE BOM (0xFF 0xFE)
        let start = if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
            2
        } else {
            0
        };

        // Try to interpret as UTF-16LE
        if (raw.len() - start) % 2 == 0 {
            let u16s: Vec<u16> = raw[start..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let decoded = String::from_utf16_lossy(&u16s);
            // If the decoded string looks reasonable (has ASCII content), use it
            if decoded.chars().any(|c| c.is_ascii_alphabetic()) {
                return decoded;
            }
        }
    }

    // Fallback to UTF-8
    String::from_utf8_lossy(raw).to_string()
}

/// Parse the text output of `wsl -l -v` into WslDistro structs
fn parse_distro_list(text: &str) -> Result<Vec<WslDistro>, MazeSshError> {
    let mut distros = Vec::new();

    for line in text.lines().skip(1) {
        // Skip empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // The `*` prefix marks the default distro
        let is_default = line.contains('*');
        let clean = line.replace('*', " ");

        // Parse from the right: VERSION (digit), STATE (word), NAME (rest)
        let parts: Vec<&str> = clean.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        // Last element is version, second-to-last is state
        let version_str = parts[parts.len() - 1];
        let state = parts[parts.len() - 2];
        let name = parts[..parts.len() - 2].join(" ");

        let version = version_str.parse::<u8>().unwrap_or(0);

        if name.is_empty() || version == 0 {
            continue;
        }

        distros.push(WslDistro {
            name,
            state: state.to_string(),
            version,
            is_default,
        });
    }

    Ok(distros)
}

/// Maximum time to wait for a WSL command before killing it and returning an error.
const WSL_CMD_TIMEOUT_SECS: u64 = 30;

/// Validate WSL distro name: must be non-empty, reasonable length,
/// and contain only characters valid in WSL distro names.
fn validate_distro_name(distro: &str) -> Result<(), MazeSshError> {
    if distro.trim().is_empty() {
        return Err(MazeSshError::WslCommandFailed("Distro name cannot be empty".to_string()));
    }
    if distro.len() > 128 {
        return Err(MazeSshError::WslCommandFailed("Distro name too long (max 128 chars)".to_string()));
    }
    // WSL distro names: alphanumeric, hyphens, underscores, spaces, dots (no shell metacharacters)
    if !distro.chars().all(|c| c.is_alphanumeric() || " -_.".contains(c)) {
        return Err(MazeSshError::WslCommandFailed(format!(
            "Distro name '{}' contains invalid characters",
            distro
        )));
    }
    Ok(())
}

/// Run a command inside a specific WSL distro with a hard 30-second timeout.
/// If the distro hangs or becomes unresponsive the child process is killed and
/// an error is returned instead of blocking the caller indefinitely.
pub fn run_in_wsl(distro: &str, args: &[&str]) -> Result<CmdOutput, MazeSshError> {
    validate_distro_name(distro)?;
    let mut cmd = hidden_cmd("wsl");
    cmd.args(["-d", distro, "--"]);
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        MazeSshError::WslCommandFailed(format!("Failed to spawn command in {}: {}", distro, e))
    })?;

    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(WSL_CMD_TIMEOUT_SECS);

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break, // process finished — collect output below
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait(); // reap zombie
                    return Err(MazeSshError::WslCommandFailed(format!(
                        "Command timed out after {}s in distro '{}'",
                        WSL_CMD_TIMEOUT_SECS, distro
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(MazeSshError::WslCommandFailed(format!(
                    "Failed to poll command in {}: {}",
                    distro, e
                )));
            }
        }
    }

    let output = child.wait_with_output().map_err(|e| {
        MazeSshError::WslCommandFailed(format!("Failed to collect output from {}: {}", distro, e))
    })?;

    Ok(CmdOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    })
}

/// Check if a file exists inside a WSL distro
pub fn wsl_file_exists(distro: &str, path: &str) -> bool {
    run_in_wsl(distro, &["test", "-e", path])
        .map(|o| o.success)
        .unwrap_or(false)
}

/// Write content to a file inside WSL via stdin pipe
pub fn wsl_write_file(distro: &str, path: &str, content: &str) -> Result<(), MazeSshError> {
    let mut child = hidden_cmd("wsl")
        .args(["-d", distro, "--", "tee", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| MazeSshError::WslCommandFailed(format!("Failed to spawn tee in {}: {}", distro, e)))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| MazeSshError::WslCommandFailed(format!("Failed to write to stdin: {}", e)))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| MazeSshError::WslCommandFailed(format!("Failed to wait for tee: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MazeSshError::WslCommandFailed(format!(
            "tee failed in {}: {}",
            distro, stderr
        )));
    }

    Ok(())
}

/// Check if socat is installed in the distro
pub fn has_socat(distro: &str) -> bool {
    run_in_wsl(distro, &["which", "socat"])
        .map(|o| o.success)
        .unwrap_or(false)
}

/// Check if systemd --user is functional in the distro
pub fn has_systemd(distro: &str) -> bool {
    run_in_wsl(distro, &["systemctl", "--user", "status"])
        .map(|o| {
            // systemctl status exits 0 or with specific codes when systemd is running
            // If systemd is not available at all, it exits with a different error
            // Check stderr for "System has not been booted with systemd"
            !o.stderr.contains("not been booted with systemd")
                && !o.stderr.contains("Failed to connect")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_distro_list_typical() {
        let text = "  NAME      STATE    VERSION\n\
                     * Ubuntu    Running  2\n\
                       Debian    Stopped  2\n";
        let distros = parse_distro_list(text).unwrap();
        assert_eq!(distros.len(), 2);

        assert_eq!(distros[0].name, "Ubuntu");
        assert_eq!(distros[0].state, "Running");
        assert_eq!(distros[0].version, 2);
        assert!(distros[0].is_default);

        assert_eq!(distros[1].name, "Debian");
        assert_eq!(distros[1].state, "Stopped");
        assert_eq!(distros[1].version, 2);
        assert!(!distros[1].is_default);
    }

    #[test]
    fn test_parse_distro_list_with_spaces_in_name() {
        let text = "  NAME                STATE    VERSION\n\
                     * Ubuntu 24.04 LTS   Running  2\n";
        let distros = parse_distro_list(text).unwrap();
        assert_eq!(distros.len(), 1);
        assert_eq!(distros[0].name, "Ubuntu 24.04 LTS");
        assert_eq!(distros[0].version, 2);
    }

    #[test]
    fn test_parse_distro_list_empty() {
        let text = "  NAME      STATE    VERSION\n";
        let distros = parse_distro_list(text).unwrap();
        assert!(distros.is_empty());
    }

    #[test]
    fn test_decode_utf16le_with_bom() {
        // "Hi\n" as UTF-16LE with BOM
        let raw: Vec<u8> = vec![
            0xFF, 0xFE, // BOM
            b'H', 0x00, b'i', 0x00, b'\n', 0x00,
        ];
        let decoded = decode_wsl_output(&raw);
        assert_eq!(decoded, "Hi\n");
    }
}
