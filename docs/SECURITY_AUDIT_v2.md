# MazeSSH Security Audit — Version 2

**Date:** 2026-04-15  
**Auditor:** Internal (post-Sprint-2 hardening pass)  
**Scope:** Full codebase — `crates/maze-crypto`, `crates/maze-vault`, `crates/maze-agent-protocol`, `src-tauri/src` (all modules)  
**Methodology:** Static analysis, data-flow tracing, threat modelling; no fuzzing or dynamic analysis.

---

## 1. Security Architecture Map

```
┌────────────────────────────────────────────────────────────────────────────┐
│  TRUST BOUNDARY: User's Windows Session                                    │
│                                                                            │
│  ┌──────────────┐  IPC (named pipe)  ┌──────────────────────────────────┐ │
│  │ SSH Client   │◄──────────────────►│  MazeSSH Agent Daemon            │ │
│  │ (git, ssh)   │  \\.\pipe\maze-    │  src-tauri/src/services/         │ │
│  └──────────────┘  ssh-agent         │  agent_service.rs                │ │
│                    DACL: SY+OW only  │  • decodes SSH agent protocol    │ │
│                                      │  • consent → sign with VEK       │ │
│  ┌──────────────┐                    └──────────────┬───────────────────┘ │
│  │  Tauri UI    │                                   │                      │
│  │  WebView     │◄──── Tauri IPC ──────────────────►│ AppState            │ │
│  │  (React)     │  (commands/*)                     │ • vault_session     │ │
│  └──────────────┘                                   │ • pending_consents  │ │
│                                                     │ • session_rules     │ │
│  ┌──────────────┐    WSL exec                       └──────────────┬──────┘ │
│  │ WSL distro   │◄──wsl -d <distro>──────────────────────────────► │      │ │
│  │ (socat relay)│                                    vault_dir      │      │ │
│  └──────────────┘                                  ~/.maze-ssh/vault│      │ │
│                                                     ├─ vault-meta.json     │ │
│                                                     └─ keys/{uuid}.enc     │ │
└────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────┐
│ EXTERNAL TRUST BOUNDARY                              │
│  Windows Credential Manager ← PIN hash, passphrases │
│  ~/.ssh/config              ← SSH identity config    │
│  ~/.maze-ssh/audit.log      ← append-only audit log │
└──────────────────────────────────────────────────────┘
```

### Key Hierarchy

```
User Passphrase (string, never stored)
      │  Argon2id(64 MiB / 3 iter / 1 par)
      ▼
 VMK (32 B) — derived per unlock, never persisted
      │  AES-256-GCM
      ▼
 encrypted_vek ─────────────── vault-meta.json
      │  (on unlock: decrypt → VEK)
      ▼
 VEK (32 B) ─── held in VaultSession (ZeroizeOnDrop)
      │  AES-256-GCM per key
      ▼
 {uuid}.enc ─────────────────── keys/ directory
```

---

## 2. Trust Boundaries

| Boundary | Direction | What crosses it |
|----------|-----------|-----------------|
| WebView → Tauri commands | →  | User input (passphrase, profile data, key names) |
| Tauri commands → WebView | → | Serialized responses, error strings |
| Named pipe client → agent daemon | → | SSH agent protocol frames (wire-format binary) |
| Agent daemon → WebView | → | `consent-request` events via Tauri emit |
| MazeSSH → WSL distro | → | Shell commands via `wsl -d <distro> --` |
| MazeSSH → Filesystem | ↔ | vault-meta.json, .enc key files, SSH config, audit log |
| MazeSSH → Windows Credential Manager | ↔ | PIN Argon2 hash, SSH passphrases |

---

## 3. Attack Surfaces

| Surface | Entry point | Threat actor |
|---------|-------------|--------------|
| Named pipe | `\\.\pipe\maze-ssh-agent` | Other local processes (same Windows session) |
| Tauri IPC | WebView invoke | Compromised WebView / XSS payload |
| Vault files | `~/.maze-ssh/vault/` | Any process running as the same user |
| WSL bridge | `wsl -d <distro> --` | Malicious WSL distro content |
| SSH config | `~/.ssh/config` | Marker injection / config confusion |
| Audit log | `~/.maze-ssh/audit.log` | Read by other local users (Unix) |
| Import PEM | `vault_import_key` | Malformed/passphrase-encrypted PEM |
| Key export | `vault_export_private_key` | XSS / compromised UI leaking exported key |

---

## 4. Sensitive Assets & Privileged Operations

| Asset | Location | Protection |
|-------|----------|------------|
| VEK (vault encryption key) | In-memory `VaultSession` | `ZeroizeOnDrop` |
| VMK (vault master key) | Transient `DerivedKey` | `ZeroizeOnDrop` |
| Private key PEM (plaintext) | Transient during sign/export | Partially zeroized (sign ✓, export ✗ — see H1) |
| PIN Argon2 hash | Windows Credential Manager | `Zeroizing<String>` wrapper ✓ |
| SSH passphrases | Windows Credential Manager | Not zeroized in security.rs (see M1) |
| Policy rules | `~/.maze-ssh/policy.json` | `0o600` on Unix ✓ |
| Audit log | `~/.maze-ssh/audit.log` | No permission restriction (see L1) |
| Bridge config | `~/.maze-ssh/bridge.json` | `0o600` on Unix via `profile_service::atomic_write` ✓ |

---

## 5. Sprint 1 & 2 Fixes Applied (Previously Audited)

| ID | Severity | Fix |
|----|----------|-----|
| C1 | Critical | Named pipe DACL: `D:(A;;GA;;;SY)(A;;GA;;;OW)` via `create_with_security_attributes_raw` |
| H3a | High | `vault.rs::change_passphrase` — `vek.zeroize()` after re-encryption |
| H3b | High | `lock_service::set_pin/verify_pin` — `Zeroizing<String>` for hash strings |
| H4 | High | Semaphore cap (100 concurrent connections) on pipe daemon |
| H5 | High | Policy file permissions `0o600` on Unix |
| M1 | Medium | Message size cap `262_144` in codec; pending buffer cap `1_048_576` |
| M2 | Medium | `vault_get_state` returns no info when app is locked |
| M3 | Medium | `test_agent_connection` requires `ensure_unlocked` |
| M4 | Medium | `delete_original_key_file` rejects symlinks and path traversal |
| M5 | Medium | Error serialization strips full filesystem paths |
| M6 | Medium | `respond_to_consent`/`get_pending_consent` require `ensure_unlocked` |
| M7 | Medium | CSP hardened: `frame-ancestors 'none'`, `object-src 'none'`, `base-uri 'self'` |
| L1 | Low | WSL timeout failure → audit log entry instead of silent drop |
| L2 | Low | Removed `shell:allow-execute/spawn/stdin-write` from frontend capability |
| L3 | Low | Bridge config uses `profile_service::atomic_write` (0o600 on Unix) |
| — | Low | `subtle::ConstantTimeEq` for key blob comparison |

---

## 6. New Findings (This Audit)

---

### H1 — Private Key PEM Not Zeroized in `export_private_key`

**Severity:** High  
**File:** `crates/maze-vault/src/vault.rs:370-372`  
**Function:** `SshKeyVault::export_private_key`

**Description:**  
`decrypt_key_file` returns a `Vec<u8>` containing the plaintext private key PEM. The code then calls `String::from_utf8(pem_bytes)`, which **moves** the bytes into a `String` without zeroizing the original buffer. The returned `String` is then serialized through the Tauri command layer and eventually dropped — without explicit memory clearing.

Contrast with `sign()` at line 396, which correctly calls `pem_bytes.zeroize()` after decryption.

```rust
// vault.rs:370-372 — current (UNSAFE)
let pem_bytes = decrypt_key_file(session.vek(), id, vault_dir)?;
String::from_utf8(pem_bytes)   // moves pem_bytes — no zeroize
    .map_err(|e| VaultError::KeyParseError(format!("...")))

// vault.rs:384-396 — sign() correctly zeroizes
let mut pem_bytes = decrypt_key_file(session.vek(), id, vault_dir)?;
// ... uses pem_bytes ...
pem_bytes.zeroize();   // ✓
```

**Attack Scenario:**  
After a private key export, the plaintext PEM bytes remain in the Rust heap. A process with read access to the process memory (e.g., a memory scanner, crash dump analyser, or another process running as the same user with `ReadProcessMemory`) could recover the private key material.

**Impact:** High — full private key compromise possible post-export.  
**Root Cause:** `String::from_utf8` consumes the `Vec<u8>` without zeroizing it first.  
**Fix:**

```rust
pub fn export_private_key(session: &VaultSession, id: &str, vault_dir: &Path) -> Result<String, VaultError> {
    // ...
    let mut pem_bytes = decrypt_key_file(session.vek(), id, vault_dir)?;
    let pem_str = std::str::from_utf8(&pem_bytes)
        .map_err(|e| VaultError::KeyParseError(format!("private key is not valid UTF-8: {e}")))?
        .to_string();
    pem_bytes.zeroize();   // clear before String is returned
    Ok(pem_str)
}
```

**Note:** The returned `String` (`pem_str`) still lives in memory until Tauri serializes and drops it. A full fix would change the return type to `Zeroizing<String>`, but that is a larger API change.

**Suggested Test:** After calling `export_private_key`, verify that `pem_bytes` memory no longer contains the key header (`-----BEGIN OPENSSH PRIVATE KEY-----`).

---

### M1 — SSH Config `IdentityFile` Path Not Quoted for Spaces

**Severity:** Medium  
**File:** `src-tauri/src/services/config_engine.rs:28`  
**Function:** `generate_config_block`

**Description:**  
The SSH config writer formats `IdentityFile` as:

```rust
config.push_str(&format!("  IdentityFile {}\n", profile.private_key_path.to_string_lossy()));
```

On Windows, key paths commonly contain spaces (e.g., `C:\Users\John Doe\.ssh\id_ed25519`). OpenSSH parses `IdentityFile` tokens greedily to the first whitespace, so a path with a space would be misread.

**Attack Scenario:**  
A user whose Windows username contains a space creates a profile. The SSH config is written as:
```
  IdentityFile C:\Users\John Doe\.ssh\id_ed25519
```
OpenSSH interprets `IdentityFile` as `C:\Users\John` and falls back to its default key search (agent, `~/.ssh/id_rsa`, etc.), potentially authenticating with an unintended key.

**Impact:** Authentication failure or silent wrong-key selection.  
**Root Cause:** Missing quotation of paths containing whitespace.  
**Fix:**

```rust
let path_str = profile.private_key_path.to_string_lossy();
let quoted_path = if path_str.contains(' ') {
    format!("\"{}\"", path_str)
} else {
    path_str.to_string()
};
config.push_str(&format!("  IdentityFile {}\n", quoted_path));
```

**Suggested Test:** Create a profile with a key path containing a space; verify the generated SSH config block quotes the path correctly.

---

### M2 — `diagnostic_cmd_to_argv` Fallback Executes via `bash -c`

**Severity:** Medium (maintenance risk / defense-in-depth failure mode)  
**File:** `src-tauri/src/commands/bridge.rs:526-528`  
**Function:** `diagnostic_cmd_to_argv`

**Description:**  
The function maps validated commands to argv arrays. A `_ =>` fallback arm handles the `nohup` command (which requires shell features). However, this arm passes `cmd` directly to `bash -c`:

```rust
_ => {
    vec!["bash".to_string(), "-c".to_string(), cmd.to_string()]
}
```

Currently this is only reached by the exact-match validated `nohup` string. But if a developer adds a new entry to the `EXACT` allowlist without adding a corresponding match arm in `diagnostic_cmd_to_argv`, it silently becomes a `bash -c` execution, bypassing argv safety.

**Attack Scenario:**  
Hypothetical: A future developer adds `"apt update && apt upgrade"` to `EXACT`, testing works, but in `diagnostic_cmd_to_argv` it falls through to `bash -c "apt update && apt upgrade"` — which is fine for this command but establishes the pattern that shell metacharacters pass through.

**Impact:** Code-quality / maintenance risk that could become a real vulnerability in future.  
**Root Cause:** Implicit fallback rather than explicit nohup arm + explicit denial of unknown commands.  
**Fix:**

```rust
fn diagnostic_cmd_to_argv(cmd: &str) -> Vec<String> {
    match cmd {
        // ... explicit arms ...
        nohup if nohup.starts_with("nohup ") => {
            // Must use shell for redirection and backgrounding
            vec!["bash".to_string(), "-c".to_string(), nohup.to_string()]
        }
        // Explicit denial — validate_diagnostic_cmd should have caught this
        other => {
            eprintln!("[maze-agent] BUG: unhandled validated command: {other}");
            vec![] // return empty, caller should check
        }
    }
}
```

**Suggested Test:** Verify that adding a command to `EXACT` that has no match arm causes a compile warning or test failure (add a test that asserts `diagnostic_cmd_to_argv` returns non-empty for all `EXACT` entries).

---

### M3 — SSH Passphrase String Not Zeroized in `security.rs`

**Severity:** Medium  
**File:** `src-tauri/src/services/security.rs:5-11`  
**Function:** `store_passphrase`

**Description:**  
The `store_passphrase(profile_id: &str, passphrase: &str)` function stores the passphrase directly to the Windows Credential Manager. The `passphrase: &str` is a borrowed reference — the function cannot zeroize the caller's memory. In `commands/profile.rs` and `commands/migration.rs`, the passphrase is received as a `String` parameter which IS zeroized by the callers (they use `zeroize()` after the call). However, the Argon2 hash round-trip (set_pin → verify_pin) was fixed, but `store_passphrase` itself creates no intermediate allocation to zeroize.

The actual gap is that `security.rs::get_passphrase` returns a `String` from keyring that is NOT wrapped in `Zeroizing`. Every caller that receives a passphrase from `get_passphrase` gets a plain `String` that isn't automatically zeroized.

```rust
// services/security.rs:14-22
pub fn get_passphrase(profile_id: &str) -> Result<Option<String>, MazeSshError> {
    // ...
    Ok(Some(pass))   // plain String — no Zeroizing wrapper
}
```

**Impact:** SSH passphrases retrieved from Credential Manager linger in heap after use.  
**Root Cause:** Return type is `String` rather than `Zeroizing<String>`.  
**Fix:** Change return type to `Result<Option<Zeroizing<String>>, MazeSshError>` and update all callers.

---

### L1 — Audit Log Missing File Permission Restriction

**Severity:** Low  
**File:** `src-tauri/src/services/audit_service.rs:34`  
**Function:** `append_log`

**Description:**  
The audit log (`~/.maze-ssh/audit.log`) is created via:

```rust
if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
```

On Unix this creates the file with permissions `0o666 & ~umask`, typically `0o644` (world-readable). A local attacker can read the audit log, which contains: timestamps, action names, key names, profile names, distro names.

The audit log does NOT contain private key material, passphrases, or the vault passphrase. The information leak is operational metadata.

**Impact:** Low — operational metadata leakage to other local users on shared Unix systems.  
**Root Cause:** No explicit `set_permissions(0o600)` call after log file creation.  
**Fix:**

```rust
if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    // ... write log entry
}
```

---

### L2 — `ExportPolicy` Defaults to Allow Private Export

**Severity:** Low (design)  
**File:** `crates/maze-vault/src/types.rs`  
**Type:** `ExportPolicy`

**Description:**  
New vault keys default to `allow_private_export: true`:

```rust
impl Default for ExportPolicy {
    fn default() -> Self {
        Self { allow_private_export: true }
    }
}
```

This means every key generated or imported without specifying a policy will be exportable. A user who does not explicitly set `allow_private_export: false` is unaware that their key can be exported from the vault.

**Impact:** Design choice with slight security implications. Keys are vault-protected, so export still requires the vault to be unlocked. But the default-permissive posture is inconsistent with least-privilege.  
**Root Cause:** Default is convenience-oriented.  
**Recommendation:** Consider making the default `false` (no export) and prompting users to explicitly enable it, or at minimum document this in the UI.

---

### L3 — H2 (Still Open): Process Identification via Name Only

**Severity:** High (acknowledged, deferred)  
**File:** `src-tauri/src/services/agent_service.rs:206-260`  
**Function:** `get_pipe_client_info`

**Description:**  
The agent uses `GetNamedPipeClientProcessId` to get the connecting process PID, then reads its image path from `QueryFullProcessImageNameW`. The process path (e.g., `C:\Program Files\Git\cmd\git.exe`) is shown in the consent dialog UI.

A malicious actor who has already compromised the user's session could:
1. Copy any binary to a path like `%APPDATA%\Microsoft\Windows\git.exe`
2. Connect to the named pipe
3. The consent dialog shows `git.exe` — the user approves

**Root Cause:** No Authenticode/digital signature verification of the connecting process binary.  
**Fix:** Verify the code signing certificate of the process image using `WinVerifyTrust` API. If the binary is signed by Microsoft or a trusted Git distributor, the UI can show a higher-trust indicator. Unsigned binaries should be flagged.  
**Complexity:** High — requires `Win32_Security_WinTrust` feature and certificate chain validation logic.  
**Status:** Deferred to Sprint 3.

---

### L4 — TOCTOU in Atomic Write (Windows)

**Severity:** Low (Windows-specific, narrow window)  
**File:** `crates/maze-vault/src/vault.rs:422-432`, `src-tauri/src/services/profile_service.rs:45-55`  
**Function:** `atomic_write`

**Description:**  
```rust
fn atomic_write(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)?;    // tmp file created with default permissions
    fs::rename(&tmp_path, path)?;      // race window: another process reads tmp
    #[cfg(unix)]
    { /* 0o600 applied */ }
}
```

On Windows, between `fs::write` and `fs::rename`, the `.tmp` file exists with default DACL (inheriting from parent dir). Another process running as the same user can read the temp file contents — which may include the re-encrypted `vault-meta.json` during a passphrase change.

**Impact:** Narrow TOCTOU window. The attacker must know the exact path and have concurrently observed the file. Impact is limited to vault metadata (KDF params + encrypted VEK), not plaintext keys.  
**Root Cause:** Windows does not easily support secure exclusive temp file creation in Rust's std.  
**Status:** Acknowledged, low priority. Full fix requires the `tempfile` crate with `NamedTempFile` that sets security attributes at creation, or Windows ACL APIs.

---

## 7. Executive Summary

MazeSSH has undergone two security hardening sprints addressing the most critical findings. The attack surface is well-understood and appropriate for a desktop SSH identity manager.

### Positive Findings

- **Two-layer key hierarchy** (VMK→VEK→per-key) is correctly implemented with `ZeroizeOnDrop` on the VEK.
- **Named pipe DACL** now restricts access to `SYSTEM` and the current Windows user only (fixed in Sprint 2).
- **Argon2id** with 64 MiB memory cost provides strong brute-force resistance for vault passphrase.
- **Constant-time comparison** (`subtle::ConstantTimeEq`) used for key blob matching in the agent.
- **Rate limiting** on PIN (5 attempts, 60s lockout) and connection semaphore (100 concurrent pipes).
- **All Tauri commands** that modify sensitive state are guarded by `ensure_unlocked`.
- **Input validation** covers hostnames, profile names, socket paths, distro names, and shell RC file allowlists.
- **Bridge diagnostic commands** use an exact-match allowlist + argv-based execution (no shell string interpretation for most commands).
- **Error messages** are sanitized before reaching the frontend.

### Top Risks Remaining

1. **(H1)** Private key PEM not zeroized in `export_private_key` — private key bytes linger in heap post-export.
2. **(L3/H2)** Process identification by name only — spoofable by renamed binaries.
3. **(M3)** SSH passphrase `String` from `get_passphrase` not wrapped in `Zeroizing`.
4. **(M1)** SSH config `IdentityFile` paths with spaces break SSH connectivity and may cause key fallback.
5. **(M2)** `diagnostic_cmd_to_argv` fallback to `bash -c` is a maintenance hazard.

---

## 8. Quick Wins (Fix Within One Sprint)

| Priority | Finding | File | Effort |
|----------|---------|------|--------|
| 1 | H1 — zeroize pem_bytes in export_private_key | vault.rs:370 | 5 min |
| 2 | M1 — quote IdentityFile paths with spaces | config_engine.rs:28 | 10 min |
| 3 | M3 — Zeroizing<String> for get_passphrase | security.rs:14 | 30 min |
| 4 | L1 — set audit log 0o600 on Unix | audit_service.rs:34 | 5 min |
| 5 | M2 — explicit nohup arm + deny-by-default fallback | bridge.rs:526 | 15 min |
| 6 | L2 — consider flipping ExportPolicy default to false | types.rs | 5 min + UI change |

---

## 9. Fix Roadmap

### Sprint 3 (Quick Wins — 1-2 days)

- [ ] Fix H1: `export_private_key` zeroize pem_bytes before String conversion
- [ ] Fix M1: Quote `IdentityFile` paths in `generate_config_block`
- [ ] Fix M3: `Zeroizing<String>` for `security.rs::get_passphrase` return type + update all callers
- [ ] Fix L1: Apply `0o600` to audit log on first creation (Unix)
- [ ] Fix M2: Add explicit nohup match arm in `diagnostic_cmd_to_argv` and add exhaustiveness test
- [ ] Discuss L2 with product: change `ExportPolicy::default()` to `allow_private_export: false`

### Sprint 4 (Medium Complexity — 3-5 days)

- [ ] H2/L3: Implement Authenticode signature verification for consent dialog (`WinVerifyTrust`)
  - Show "Verified Publisher: Microsoft Corporation" vs "Unsigned binary" in consent UI
  - Block always-allow policies for unsigned processes

### Sprint 5 (Hardening & Audit)

- [ ] L4: Evaluate `tempfile` crate for atomic writes on Windows with proper ACL
- [ ] Add tamper-detection to audit log (HMAC over entries)
- [ ] Implement audit log `0o600` enforcement at directory creation time
- [ ] Consider structured log rotation with cryptographic chain-of-custody

---

## 10. Security Hardening Checklist for PR Review

Use this checklist when reviewing PRs that touch security-sensitive code:

### Key Material
- [ ] Private key bytes (`Vec<u8>`, `String`) are zeroized after use
- [ ] Passphrase parameters use `Zeroizing<String>` or `mut String` + `zeroize()` at end
- [ ] `VaultSession` is never cloned or passed by value
- [ ] `VEK` bytes are only accessed via `session.vek()` and never stored separately

### Tauri Commands
- [ ] Every command that reads/modifies state starts with `ensure_unlocked(&state)?`
- [ ] Commands that require vault access hold the lock minimally (lock → read vek → release)
- [ ] Passphrase parameters are `mut String` and zeroized in the command body
- [ ] Return errors do not contain full filesystem paths

### Input Validation
- [ ] Profile hostname/alias validated by `validate_hostname` (alphanumeric + `-` + `.`)
- [ ] WSL distro names validated by `validate_distro_name`
- [ ] Socket paths validated by `validate_socket_path` (`/tmp/` or `/run/user/`, char allowlist)
- [ ] File paths from user input are canonicalized and checked against an expected parent directory
- [ ] Backup/rollback paths must be inside `~/.ssh/` and match `config.backup.*`

### Agent / Named Pipe
- [ ] New pipe instances use `create_secure_pipe_server` (SDDL DACL)
- [ ] `MAX_PENDING` buffer limit is enforced before parsing frames
- [ ] Consent always requires `ensure_unlocked` before resolving

### Crypto
- [ ] `rand::rngs::OsRng` used for all nonce and salt generation
- [ ] AES-256-GCM nonces are 12 bytes, never reused per key
- [ ] Argon2id parameters: memory ≥ 64 MiB, time ≥ 3, parallelism = 1
- [ ] `subtle::ConstantTimeEq` used for any key material comparison

### Filesystem
- [ ] Config/key files written via `atomic_write` (temp → rename)
- [ ] Unix: `0o600` applied after write
- [ ] Symlinks rejected before deletion operations (canonicalize + symlink_metadata check)
- [ ] Path containment verified before backup/restore operations

### WSL Bridge
- [ ] Diagnostic commands validated via exact allowlist in `validate_diagnostic_cmd`
- [ ] `diagnostic_cmd_to_argv` uses explicit argv (no shell string) for all commands except nohup
- [ ] RC file paths for shell injection are validated against `ALLOWED` list
- [ ] `wsl_write_file` path argument comes from hardcoded shell profile paths, not user input

### Capabilities / IPC
- [ ] Frontend capability file does not include `shell:allow-execute` or `shell:allow-spawn`
- [ ] CSP includes `frame-ancestors 'none'`, `object-src 'none'`, `base-uri 'self'`
- [ ] New Windows have their own capability file scoped to minimum required permissions

---

## Appendix: Finding Classification Reference

| ID | Module | Severity | Status |
|----|--------|----------|--------|
| C1 | agent_service.rs | Critical | **Fixed (Sprint 2)** |
| H1 | vault.rs:370 | High | **Open (Sprint 3)** |
| H2 | agent_service.rs | High | **Open (Sprint 4)** |
| H3 | vault.rs, lock_service.rs | High | **Fixed (Sprint 2)** |
| H4 | agent_service.rs | High | **Fixed (Sprint 1)** |
| H5 | policy_service.rs | High | **Fixed (Sprint 1)** |
| M1 | config_engine.rs:28 | Medium | **Open (Sprint 3)** |
| M2 | bridge.rs:526 | Medium | **Open (Sprint 3)** |
| M3 | security.rs:14 | Medium | **Open (Sprint 3)** |
| M4 | codec.rs | Medium | **Fixed (Sprint 1)** |
| M5 | vault.rs (commands) | Medium | **Fixed (Sprint 1)** |
| M6 | vault.rs (commands) | Medium | **Fixed (Sprint 1)** |
| M7 | tauri.conf.json | Medium | **Fixed (Sprint 1)** |
| L1 | audit_service.rs:34 | Low | **Open (Sprint 3)** |
| L2 | types.rs | Low | **Open (design discussion)** |
| L3 | agent_service.rs | Low | **Fixed (Sprint 2)** |
| L4 | vault.rs:422 | Low | **Open (Sprint 5)** |
| L5 | bridge.rs:526 | Low | **Fixed (Sprint 2)** |
