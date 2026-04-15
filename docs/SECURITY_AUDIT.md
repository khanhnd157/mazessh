# MazeSSH Security Audit Report

**Date:** 2026-04-15
**Scope:** Full codebase audit — all Rust crates, Tauri commands, frontend, IPC, WSL bridge
**Method:** Static code review, line-by-line analysis of all security-critical modules

---

## Security Architecture Map

```
                            TRUST BOUNDARY: OS Process
  ┌────────────────────────────────────────────────────────────────┐
  │  Tauri App Process (maze-ssh.exe)                              │
  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
  │  │ React UI      │  │ Vault        │  │ Agent Daemon         │ │
  │  │ (WebView)     │◄─┤ (AppState)   │◄─┤ (Named Pipe Listener)│ │
  │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │
  │         │ invoke()        │ Mutex                │             │
  │         ▼                 ▼                      ▼             │
  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
  │  │ Tauri Commands│  │ maze-vault   │  │ \\.\pipe\maze-ssh-   │ │
  │  │ (IPC bridge) │  │ (encrypted)  │  │ agent                │ │
  │  └──────────────┘  └──────────────┘  └──────────────────────┘ │
  └────────────────────────────┬───────────────────────┬──────────┘
                               │                       │
          TRUST BOUNDARY: Filesystem        TRUST BOUNDARY: Named Pipe
                               │                       │
                               ▼                       ▼
                    ~/.maze-ssh/vault/          Any local process
                    Windows Credential Mgr     (ssh.exe, git.exe, attacker)
```

### Trust Boundaries

| Boundary | Description | Risk Level |
|----------|-------------|------------|
| Named Pipe | Any local process can connect to `\\.\pipe\maze-ssh-agent` | **HIGH** |
| Tauri IPC | WebView ↔ Rust commands via `invoke()` | MEDIUM |
| Filesystem | `~/.maze-ssh/vault/` files readable by current user | MEDIUM |
| Keyring | Windows Credential Manager — OS-level protection | LOW |
| WSL Bridge | socat relay between Windows pipe and WSL unix socket | HIGH |

### Sensitive Assets

| Asset | Location | Protection |
|-------|----------|------------|
| Private SSH keys (encrypted) | `~/.maze-ssh/vault/keys/*.enc` | AES-256-GCM + VEK |
| Vault Encryption Key (VEK) | In-memory (`VaultSession`) | ZeroizeOnDrop |
| Vault Master Key (VMK) | Derived on-demand, never stored | Zeroize after use |
| PIN hash | Windows Credential Manager | Argon2id |
| Key passphrases | Windows Credential Manager | Plaintext in keyring |
| Policy rules | `~/.maze-ssh/vault/policy-rules.json` | **No file permissions** |
| Audit log | `~/.maze-ssh/audit.log` | No encryption |

### Privileged Operations

| Operation | Required Auth | Mechanism |
|-----------|--------------|-----------|
| Vault unlock | PIN/passphrase | Argon2id → VMK → VEK |
| Key signing | Vault unlocked + consent | Policy check → consent popup |
| Key generation | Vault unlocked | `ensure_unlocked()` guard |
| Key export (private) | Vault unlocked + export policy | Export policy check |
| Profile activation | App unlocked | `ensure_unlocked()` guard |
| File deletion | App unlocked + path validation | `~/.ssh/` restriction |

---

## Findings

### CRITICAL — C1: Named Pipe Has No Access Control (DACL)

**File:** `src-tauri/src/services/agent_service.rs:43-46`

**Description:** The named pipe `\\.\pipe\maze-ssh-agent` is created with default Windows permissions. Any process running on the same machine can connect, list keys, and request signatures.

**Evidence:**
```rust
// Line 43-46
let server = ServerOptions::new()
    .first_pipe_instance(false)
    .create(PIPE_NAME)?;
```
No `SecurityAttributes` or DACL is set. Windows default allows all users in the same logon session.

**Attack Scenario:**
1. Malicious process connects to the pipe
2. Sends `SSH_AGENTC_REQUEST_IDENTITIES` → gets list of all active vault keys
3. Sends `SSH_AGENTC_SIGN_REQUEST` with a key blob
4. If user has "Always Allow" policy rule, signing happens silently
5. Attacker gains SSH authentication capability

**Impact:** Complete SSH identity compromise without user interaction (if "Always" policy exists).

**Root Cause:** `tokio::net::windows::named_pipe::ServerOptions` doesn't set restrictive DACL by default.

**Fix Recommendation:**
```rust
// Create pipe with DACL restricting to current user + SYSTEM
use windows_sys::Win32::Security::*;
// Set SECURITY_ATTRIBUTES with restricted DACL before pipe creation
```

**Patch Strategy:** Create a `create_secure_pipe()` helper that builds proper SECURITY_ATTRIBUTES.

**Suggested Tests:**
- Unit test: verify pipe rejects connections from different user sessions
- Integration test: connect from a different user account → expect rejection

---

### CRITICAL — C2: Consent Response Injectable Without Auth

**File:** `src-tauri/src/commands/vault.rs:429-455`

**Description:** `respond_to_consent()` and `get_pending_consent()` have **no `ensure_unlocked()` guard**. If the app is PIN-locked, an attacker with WebView access (or a compromised frontend) can approve pending consent requests.

**Evidence:**
```rust
// Line 429 — NO ensure_unlocked() call
#[tauri::command]
pub fn respond_to_consent(
    consent_id: String,
    approved: bool,
    ...
) -> Result<(), MazeSshError> {
    let mut consents = state.pending_consents.lock()...
```

**Attack Scenario:**
1. User locks the app (PIN screen shown)
2. An SSH signing request arrives → pending consent created
3. Attacker calls `respond_to_consent("consent-id", true, ...)` via Tauri IPC
4. Signing proceeds despite the app being locked

**Impact:** Authorization bypass — signing occurs without user knowledge.

**Root Cause:** Missing auth guard on consent commands.

**Fix Recommendation:** Add `ensure_unlocked(&state)?;` as first line in both functions.

**Patch Strategy:** 1 line fix per function.

**Suggested Tests:**
- Test: lock app → create pending consent → call respond_to_consent → expect AppLocked error

---

### CRITICAL — C3: Integer Overflow in Agent Protocol Decoder

**File:** `crates/maze-agent-protocol/src/codec.rs:57-58, 162-163`

**Description:** `try_read_frame()` converts a u32 length to usize and adds 4 without overflow check. On 32-bit systems, `0xFFFFFFFF + 4` wraps to `3`, allowing the bounds check to pass with a tiny buffer.

**Evidence:**
```rust
// Line 162-163
let msg_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
let total = 4 + msg_len;  // No checked_add
```

**Attack Scenario:** Attacker sends `[0xFF, 0xFF, 0xFF, 0xFF, ...]` → integer overflow → buffer over-read.

**Impact:** Memory safety violation (panic or data leak).

**Fix Recommendation:**
```rust
let total = 4_usize.checked_add(msg_len)
    .filter(|&t| t <= MAX_MESSAGE_SIZE)
    .ok_or(AgentError::InvalidFormat("message too large".into()))?;
```

**Suggested Tests:** Fuzz test with `msg_len = u32::MAX`, verify error returned.

---

### HIGH — H1: No Message Size Limit (DoS via Memory Exhaustion)

**File:** `crates/maze-agent-protocol/src/codec.rs:19-30` and `src-tauri/src/services/agent_service.rs:98`

**Description:** No upper bound on SSH agent message size. A malicious client can claim a 2GB message, causing OOM. The agent buffer at line 98 is 64KB, but the `pending` Vec grows unboundedly.

**Evidence:**
```rust
// agent_service.rs:98
let mut buf = vec![0u8; 65536];
let mut pending = Vec::new();
// ... pending grows without limit
```

**Attack Scenario:** Attacker sends header `[0x7F, 0xFF, 0xFF, 0xFF]` (2GB) then streams garbage → `pending` vector grows until OOM.

**Impact:** Agent daemon crash, DoS.

**Fix Recommendation:** Add `const MAX_MESSAGE_SIZE: usize = 262_144;` (256KB) and `const MAX_PENDING_SIZE: usize = 1_048_576;` (1MB). Reject messages exceeding limits.

---

### HIGH — H2: Process Identification Can Be Spoofed

**File:** `src-tauri/src/services/agent_service.rs:157-195`

**Description:** `get_process_info()` uses `GetModuleFileNameExW` to get the process name/path, but this can be trivially spoofed by renaming an executable.

**Evidence:**
```rust
// Line 185-188
let path = OsString::from_wide(&buf[..len as usize]).to_string_lossy().to_string();
let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
```

**Attack Scenario:** Attacker renames malware to `git.exe` → consent popup shows "git.exe wants to sign" → user approves.

**Impact:** Social engineering attack leading to unauthorized signing.

**Fix Recommendation:** Verify binary signature (Authenticode) or maintain a trusted-path allowlist.

---

### HIGH — H3: Passphrase/PIN Not Zeroized in Lower Layers

**Files:**
- `src-tauri/src/services/lock_service.rs:11,27` — `set_pin(pin)` / `verify_pin(pin)` don't zeroize the `&str` parameter
- `crates/maze-vault/src/vault.rs:40,67,81` — passphrase `&str` parameters not zeroized
- `src-tauri/src/services/security.rs:5` — `store_passphrase(passphrase)` not zeroized

**Description:** While `commands/security.rs` properly zeroizes `String` parameters after use, the lower-level functions receive `&str` references to the original `String`. The `String` is zeroized, but intermediate copies in the call stack may not be. Additionally, `lock_service.rs` functions take `&str` and cannot zeroize the caller's data.

**Impact:** Passphrase residue in memory on error paths, vulnerable to memory dumps.

**Fix Recommendation:** Audit all code paths. Consider passing `Zeroizing<String>` from the `zeroize` crate instead of `&str`.

---

### HIGH — H4: Unbounded Concurrent Pipe Connections

**File:** `src-tauri/src/services/agent_service.rs:35-78`

**Description:** Each pipe connection spawns an unbounded `tokio::spawn` with a 64KB buffer. No connection limit.

**Attack Scenario:** Attacker opens 10,000 pipe connections → 640MB memory consumption → OOM.

**Fix Recommendation:** Add `tokio::sync::Semaphore` with max 100 concurrent connections.

---

### HIGH — H5: Policy-rules.json Has No File Permissions

**File:** `src-tauri/src/services/policy_service.rs:65-72`

**Description:** `save_rules()` uses atomic write but sets **no file permissions**. The vault's `atomic_write()` sets 0o600 on Unix, but `policy_service` doesn't call it.

**Evidence:**
```rust
// Line 69-71 — no permission setting
let tmp_path = path.with_extension("tmp");
fs::write(&tmp_path, &content)?;
fs::rename(&tmp_path, &path)?;
```

**Impact:** Policy rules world-readable. Attacker can read which keys have "always allow" rules.

**Fix Recommendation:** Use the vault's `atomic_write()` helper or set 0o600 permissions after rename.

---

### MEDIUM — M1: Timing Side-Channel in Key Blob Matching

**File:** `src-tauri/src/services/agent_service.rs:459`

**Description:** `find_key_by_blob()` uses `==` for key blob comparison, which is not constant-time.

**Evidence:**
```rust
if &blob[..] == target_blob {
```

**Impact:** Attacker can infer key blob structure via timing analysis.

**Fix Recommendation:** Use `subtle::ConstantTimeEq` from the `subtle` crate.

---

### MEDIUM — M2: vault_get_state() Leaks Info When App Locked

**File:** `src-tauri/src/commands/vault.rs:82-101`

**Description:** `vault_get_state()` has no `ensure_unlocked()` guard. Returns vault initialization status and key count even when app is PIN-locked.

**Impact:** Information disclosure — attacker learns if vault exists and how many keys are stored.

**Fix Recommendation:** Return only `initialized` field when locked, hide `key_count` and `unlocked`.

---

### MEDIUM — M3: test_agent_connection() Callable Without Auth

**File:** `src-tauri/src/commands/vault.rs:25-77`

**Description:** `test_agent_connection()` has no auth guard. Can probe agent status while app is locked.

**Fix Recommendation:** Add `ensure_unlocked(&state)?;`.

---

### MEDIUM — M4: Symlink Following in delete_original_key_file()

**File:** `src-tauri/src/commands/vault.rs:401`

**Description:** `dunce::canonicalize()` follows symlinks. Attacker could create `~/.ssh/id_rsa` → `/etc/important_file` symlink.

**Fix Recommendation:** Check `std::fs::symlink_metadata()` and reject symlinks.

---

### MEDIUM — M5: Error Messages Leak Full File Paths

**File:** `src-tauri/src/error.rs`

**Description:** `KeyNotFound(PathBuf)` and `NotAGitRepo(PathBuf)` serialize full paths to the frontend.

**Impact:** Reveals username, directory structure.

**Fix Recommendation:** Sanitize paths in `Serialize` impl — show filename only, not full path.

---

### MEDIUM — M6: TOCTOU in Vault Atomic Write (Windows)

**File:** `crates/maze-vault/src/vault.rs:422-432`

**Description:** On Windows, temp file has default (permissive) permissions during the brief window between `fs::write` and `fs::rename`.

**Fix Recommendation:** Create temp file with restricted ACL on Windows, or use `tempfile` crate with restricted permissions.

---

### MEDIUM — M7: CSP Missing frame-ancestors and object-src

**File:** `src-tauri/tauri.conf.json:25`

**Current CSP:** `default-src 'self'; script-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data: asset: https://asset.localhost`

**Missing:** `frame-ancestors 'none'; object-src 'none'; base-uri 'self'`

---

### LOW — L1: Consent Timeout Not Audit-Logged

**File:** `src-tauri/src/services/agent_service.rs:340`

**Description:** When consent times out (60s), the denial is not logged to audit trail.

---

### LOW — L2: Shell Capabilities Overly Broad

**File:** `src-tauri/capabilities/default.json:19-20`

**Description:** `shell:allow-execute` and `shell:allow-spawn` grant unrestricted shell access.

---

### LOW — L3: Bridge Config Stored Without File Permissions

**File:** `src-tauri/src/services/bridge_service.rs` — `bridge.json` saved without restricted permissions.

---

## Executive Summary

MazeSSH implements a strong cryptographic architecture (AES-256-GCM + Argon2id, 2-layer key hierarchy, ZeroizeOnDrop). However, the **agent daemon's named pipe lacks access control** (C1), allowing any local process to request signing. Combined with policy rules that auto-approve signing (M3 "Always Allow"), this creates a path to **silent SSH identity compromise without user interaction**.

The consent flow has an **authorization bypass** (C2) where consent can be approved even when the app is PIN-locked. The protocol decoder has an **integer overflow** (C3) that could cause memory safety violations.

**Positive findings:** Private key encryption is sound, VaultSession properly zeroizes, error messages are generally safe, rate limiting on PIN is correct, export policies are enforced.

---

## Top 5 Risks

1. **Silent signing via named pipe** (C1 + "Always" policy) — any local malware can sign as user
2. **Consent bypass when locked** (C2) — locked app can still approve signing
3. **Protocol DoS** (C3 + H1 + H4) — crash agent via malformed messages or connection flood
4. **Process spoofing** (H2) — user approves signing for fake "git.exe"
5. **Passphrase residue** (H3) — passphrase strings not fully zeroized in lower layers

---

## Quick Wins (< 1 hour each)

| Fix | Effort | Impact |
|-----|--------|--------|
| Add `ensure_unlocked()` to `respond_to_consent` + `get_pending_consent` | 5 min | Fixes C2 |
| Add `checked_add()` in `try_read_frame()` | 10 min | Fixes C3 |
| Add `MAX_MESSAGE_SIZE` constant (256KB) + check | 15 min | Fixes H1 |
| Add `MAX_PENDING_SIZE` check in `handle_connection` | 10 min | Fixes H1 |
| Add `Semaphore(100)` for concurrent connections | 15 min | Fixes H4 |
| Set file permissions on `policy-rules.json` | 10 min | Fixes H5 |
| Add `ensure_unlocked()` to `test_agent_connection` | 5 min | Fixes M3 |
| Add symlink check in `delete_original_key_file` | 10 min | Fixes M4 |
| Log consent timeouts | 5 min | Fixes L1 |
| Harden CSP with frame-ancestors, object-src | 5 min | Fixes M7 |

---

## Fix Roadmap

### Sprint 1 (Critical + Quick Wins) — 1-2 days
- [ ] C2: Add ensure_unlocked to consent commands
- [ ] C3: Integer overflow fix in codec
- [ ] H1: Message size limits
- [ ] H4: Connection semaphore
- [ ] H5: Policy file permissions
- [ ] M3: Auth guard on test command
- [ ] M4: Symlink check
- [ ] M7: CSP hardening
- [ ] L1: Consent timeout logging

### Sprint 2 (Named Pipe Security) — 3-5 days
- [ ] C1: Implement DACL on named pipe (Windows Security API)
- [ ] H2: Add trusted-path allowlist for process verification
- [ ] M1: Constant-time key blob comparison

### Sprint 3 (Memory Hardening) — 2-3 days
- [ ] H3: Audit all passphrase/PIN code paths, use Zeroizing<String>
- [ ] M6: Secure temp file creation on Windows
- [ ] M5: Sanitize error message paths

### Sprint 4 (Defense in Depth) — 2-3 days
- [ ] L2: Restrict shell capabilities to specific executables
- [ ] L3: Bridge config file permissions
- [ ] Add integration tests for all security-critical paths
- [ ] Fuzz testing for agent protocol decoder

---

## Security Hardening Checklist for PR Review

### Before approving any PR, verify:

**Crypto & Keys:**
- [ ] No private key material logged, serialized, or returned without export policy check
- [ ] All `String` containing passphrases/PINs are `.zeroize()`-d after use (including error paths)
- [ ] `VaultSession` is never cloned, serialized, or sent across threads without Mutex
- [ ] New crypto uses `OsRng`, not `thread_rng()` or other weak RNG

**Commands:**
- [ ] Every new Tauri command that accesses vault data calls `ensure_unlocked(&state)?;`
- [ ] User-supplied paths validated against path traversal (no `..`, no symlinks)
- [ ] User-supplied strings not interpolated into shell commands (use `.env()` or `.arg()`)
- [ ] Error messages don't include full file paths or internal state

**Agent/IPC:**
- [ ] No new pipe/socket listeners without explicit access control
- [ ] Message parsing has size limits and overflow checks
- [ ] Consent decisions validated (consent ID exists, not expired)

**Frontend:**
- [ ] No private key material displayed (only fingerprints)
- [ ] Consent popup can't be auto-approved by script
- [ ] Clipboard operations clear after timeout

**Dependencies:**
- [ ] `cargo audit` passes with no known vulnerabilities
- [ ] No new `unsafe` blocks without justification
- [ ] New crates reviewed for security posture
