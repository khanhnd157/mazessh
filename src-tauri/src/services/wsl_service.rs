use std::io::Write;
use std::process::Stdio;

use crate::error::MazeSshError;
use crate::models::bridge::WslDistro;
use crate::services::ssh_engine::hidden_cmd;

/// Output from a command executed inside WSL
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Check if WSL is installed and functional
pub fn is_wsl_available() -> bool {
    hidden_cmd("wsl")
        .args(["--status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
    let output = hidden_cmd("wsl")
        .args(["-l", "-v"])
        .output()
        .map_err(|e| MazeSshError::WslNotAvailable(format!("Failed to run wsl: {}", e)))?;

    if !output.status.success() {
        return Err(MazeSshError::WslNotAvailable(
            "wsl -l -v returned non-zero exit code".to_string(),
        ));
    }

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

/// Run a command inside a specific WSL distro
pub fn run_in_wsl(distro: &str, args: &[&str]) -> Result<CmdOutput, MazeSshError> {
    let mut cmd = hidden_cmd("wsl");
    cmd.args(["-d", distro, "--"]);
    cmd.args(args);

    let output = cmd
        .output()
        .map_err(|e| MazeSshError::WslCommandFailed(format!("Failed to run command in {}: {}", distro, e)))?;

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
