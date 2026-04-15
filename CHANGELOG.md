# Changelog

<!-- markdownlint-disable MD024 -->

All notable changes to Maze SSH are documented in this file.

## [1.1.2] - 2026-04-15

### SSH Key Vault (New)

- Encrypted SSH key vault backed by AES-256-GCM with Argon2id KDF (64 MiB / 3 iterations)
- Two-layer key hierarchy: passphrase → VMK → VEK → per-key encrypted files
- VEK held in memory as `VaultSession` with `ZeroizeOnDrop` — cleared automatically on lock
- Key generation: Ed25519 and RSA 4096 via `ssh-key` crate
- Key import: accepts OpenSSH PEM (encrypted or plaintext)
- Key metadata: fingerprint (SHA-256), algorithm, state (active/archived), export policy, per-key allowed hosts
- Export public key (always allowed) and private key (governed by per-key export policy)
- Archive and delete keys; delete rejects symlinks and path-traversal attempts
- Vault tab in UI: key grid with search and state filter, generate/import dialogs, key detail sheet
- Vault setup prompt (first-time init) and unlock prompt (separate passphrase or shared with PIN)
- Change vault passphrase without re-encrypting individual key files
- Vault state hidden from frontend when app is locked

### SSH Agent (New)

- Native Windows SSH agent on named pipe `\\.\pipe\maze-ssh-agent`
- Implements SSH agent protocol: `request-identities`, `sign-request`, `remove-all-identities`
- `add-identity` rejected — keys are vault-managed only
- Named pipe DACL restricts connections to SYSTEM and current user (no other local processes)
- Per-connection semaphore cap (100 concurrent clients)
- Pending read buffer cap (1 MB) to prevent memory exhaustion
- SSH agent protocol message size limit (256 KB) to prevent integer overflow / DoS
- Process identification on connect via `GetNamedPipeClientProcessId` + `QueryFullProcessImageNameW`
- Consent popup on first sign request per key: shows key name, fingerprint, requesting process name/path, PID
- 60-second consent timeout with automatic denial and audit log entry
- Constant-time key blob comparison (`subtle::ConstantTimeEq`) to prevent timing attacks

### Policy Engine (New)

- Three consent modes: **Once** (single request), **Session** (until app lock), **Always** (persisted rule)
- Session rules stored in memory, cleared on app lock or restart
- Persistent "always allow" rules saved to `~/.maze-ssh/policy-rules.json` (`0o600` on Unix)
- Policy rule management: list, delete individual rule, clear all rules
- Policy checked before consent popup — pre-approved keys sign without user interaction
- Host-based routing: keys can be restricted to specific SSH hostnames (`allowed_hosts` field)

### Security Hardening

- All private key PEM bytes zeroized after use in signing and export operations
- Vault passphrase zeroized in all Tauri command handlers after use
- Argon2 hash strings wrapped in `Zeroizing<String>` in lock service
- SSH passphrase `get_passphrase` returns `Zeroizing<String>` across all callers
- VEK bytes explicitly zeroized in `change_passphrase` after re-encryption
- Consent commands (`respond_to_consent`, `get_pending_consent`) require app to be unlocked
- `test_agent_connection` command requires app to be unlocked
- Error serialization strips filesystem paths before reaching frontend
- Comprehensive security audit report (`docs/SECURITY_AUDIT.md`, `docs/SECURITY_AUDIT_v2.md`)

### Profile Integration

- `vault_key_id` field on profiles: links a profile to a vault key
- Agent mode setting: **MazeSSH Vault** (sign via vault) vs **External Agent** (forward to system agent)
- Vault unlock mode: **Shared PIN** (vault unlocks with app PIN) or **Separate Passphrase**
- Profile activation uses vault key for signing when agent mode is set to Vault
- Agent-aware env file generation

### Migration

- Migration wizard: import existing `~/.ssh` key files into vault in bulk
- Preview shows eligible profiles and skipped profiles with reasons
- Migrated keys retain passphrase via Windows Credential Manager lookup
- Original key files not deleted automatically — user confirms deletion per key

### UI

- Vault tab with key grid, search, algorithm/state filter
- Generate Key dialog: name, algorithm toggle (Ed25519/RSA 4096), comment, export policy
- Import Key dialog: file drop zone or paste PEM, optional source passphrase
- Key Detail sheet (slide-in panel): overview, public key copy, security settings, export
- Migration Wizard: 4-step dialog (intro → preview → progress → results)
- Agent Mode Settings section in Security tab
- Consent popup window (separate Tauri window, always-on-top, 60s countdown timer)

## [1.1.1] - 2026-04-15

### WSL Bridge (New)

- Forward the Windows OpenSSH Agent socket into WSL distributions via `npiperelay`-based relay
- Per-distro bridge status cards: bootstrap, teardown, start/stop/restart relay in one click
- Multi-provider support: OpenSSH Agent, 1Password, Pageant — switchable per distro
- Custom provider: point the bridge at any Unix socket path
- FIDO2 hardware key detection and auto-recommendation for compatible providers
- Agent forwarding toggle per distro
- Relay modes: Systemd service or background daemon (`nohup`)
- Binary auto-download: fetch `npiperelay` for Windows + relay script for Linux automatically
- Diagnostics panel with one-click auto-fix for common relay failures
- Relay log viewer (last N lines, live refresh)
- Pipe scanner: list all active Windows named pipes to help debug provider selection
- Bridge config export / import as JSON
- Bootstrap All: provision every configured distro in one operation with per-distro status
- Health history ring buffer (200 events per distro): relay lifecycle events with timestamps
- Per-distro watchdog with configurable `max_restarts` (1–20, adjustable via slider)
- Windows SSH config auto-population: adds `Host maze-wsl-<distro>` block to `~/.ssh/config` for direct `ssh maze-wsl-ubuntu` access
- SSH host connectivity test from the Bridge tab
- Tray notification when a relay restarts automatically
- CLI subcommands: `maze-ssh-cli bridge list`, `status`, `bootstrap`, `teardown`, `relay start/stop/restart`

### Security

- Named pipe `\\.\pipe\maze-ssh-agent` now has a DACL restricting connections to SYSTEM and the current Windows user only (previously allowed any local process)
- Socket path validation: strict character allowlist, `..`/`.` component rejection, prefix enforcement (`/tmp/` or `/run/user/` only)
- Diagnostic quick-fix commands validated against exact allowlist before execution; mapped to argv arrays (no shell string interpolation for most commands)
- Marker removal in shell RC files validated against allowlisted paths only (`~/.bashrc`, `~/.zshrc`, `~/.config/fish/config.fish`, `~/.profile`)
- WSL distro names validated (character allowlist) before any `wsl -d` invocation
- WSL command timeout: 30-second hard deadline with SIGKILL on breach
- PowerShell `GIT_SSH_COMMAND` env var passed via process environment, not interpolated into the script string
- Input validation hardened across profile, hostname, pin, and host-alias fields
- CSP hardened: `frame-ancestors 'none'`, `object-src 'none'`, `base-uri 'self'`
- `RwLock` for bridge config state; `Mutex` guards on all shared state with `map_err` poison recovery
- Atomic writes for all config files; Unix permissions `0o600` enforced post-write
- Path traversal prevention on SSH config backup/restore (canonicalize + containment check)
- Removed `shell:allow-execute`, `shell:allow-spawn`, `shell:allow-stdin-write` from frontend capability (WebView does not use the shell plugin)
- Error messages sanitised before reaching the frontend (no full filesystem paths)

### Key Management

- SSH key health check: analyses key file type, bit length, passphrase protection, and age; results shown in Security Settings
- Copy public key to clipboard directly from profile detail view

### Audit Log

- Log rotation at 1 MB (renames current file to `.log.1`, creates fresh file)
- Bridge lifecycle events (bootstrap, teardown, start/stop, relay restart, watchdog paused) written to audit log

### UI/UX

- Bridge tab with live distro status grid
- Collapsible history panel per distro (lazy-loaded on open, auto-refreshes on relay events)
- Advanced settings section per distro: `max_restarts` slider, Windows SSH config toggle
- `ErrorBoundary` component wrapping the full app for graceful crash recovery
- ARIA roles, labels, and `tabIndex` on all dialogs, tabs, and interactive buttons
- Consistent z-index scale across overlays, modals, and dropdowns
- Custom scrollbar styling for log panels

### Refactors & Code Quality

- Linter and auto-format pass across all backend and frontend files
- `hidden_cmd()` helper applied to WSL, SSH, and Git process spawning
- Relay watchdog restart count capped and surfaced in UI

## [1.1.0] - 2026-04-12

### CLI Tool (New)

- Standalone `maze-ssh-cli` binary sharing the same profile data as the desktop app
- Commands: `list`, `use <name>`, `use --auto`, `current`, `off`, `status`, `test`, `config preview/write/backups`, `export`, `import`
- Builds independently without Tauri dependency using Rust feature flags
- Colored terminal output with status indicators
- CI/CD builds CLI for Windows, macOS (Intel + ARM), and Linux

### Performance

- Eliminated terminal window flash on all spawned processes (`CREATE_NO_WINDOW` flag)
- SSH key fingerprint caching (ssh-keygen runs once per key, cached permanently)
- Instant app lock (emit-first pattern: UI locks before background agent cleanup)
- Optimistic UI updates for lock/switch actions
- `hidden_cmd()` helper applied to all powershell, ssh-add, ssh-keygen, and git calls

### Security

- PIN rate limiting: max 5 failed attempts, then 60-second lockout
- Failed attempt counter with remaining attempts shown in audit log

### New Features

- Keyboard shortcuts: Ctrl+1-4 for tabs, Ctrl+L to lock
- Auto-navigate to Profiles tab when selecting a profile from any tab
- Loading spinner on Switch button during profile activation
- Custom ConfirmDialog replacing all native browser confirm() popups
- Switch dropdown renders via React Portal (no longer clipped by overflow containers)
- On switch: auto-select profile + navigate to detail + activate in background

### Documentation

- CLI user guide (`docs/CLI.md`)
- Desktop app user guide in English (`docs/USER_GUIDE.md`) and Vietnamese (`docs/USER_GUIDE_VI.md`)

### Infrastructure

- Feature flags: `desktop` (Tauri, default) vs no-features (CLI only)
- Separate `[[bin]]` target for CLI in Cargo.toml
- CI/CD builds CLI binaries for all platforms alongside desktop installers
- `uiStore` (Zustand) for shared tab navigation state

## [1.0.1] - 2026-04-12

### Performance

- Tab switching now uses CSS opacity transitions instead of unmount/remount — zero re-fetching, instant state preservation
- Stabilized all useEffect dependencies using `store.getState()` pattern — eliminated unnecessary re-renders and API calls
- Event listeners registered once with empty dependency arrays
- Inactivity tracker throttled to 2 DOM events with timer-based throttle

### UI/UX Improvements

- Smooth 150ms fade transition between tabs with animated underline indicator
- Custom ConfirmDialog component replacing all native `confirm()` popups — matches Proton design with danger/warning/info variants, backdrop blur, Escape to close
- Refined spacing and density across all components: sidebar, profile cards, activity log, bottom bar
- Fade-in animation for error messages and modal content
- LockScreen polish: logo image, visible input borders, fade-in errors
- Tighter letter-spacing and improved typography
- Quieter git identity status bar
- Smaller, more compact activity log entries

## [1.0.0] - 2026-04-12

### M4 — Advanced Features

- SSH config rollback: list backup history, restore any backup with one click
- View current `~/.ssh/config` content alongside generated preview
- Git hooks: generate pre-push hook that validates identity matches profile
- Install/remove hooks directly from Repo Mapping cards
- Profile export to clipboard as JSON for backup or migration
- Profile import from clipboard (merges, skips duplicates by name)
- SSH key fingerprint display on profile detail (via `ssh-keygen -lf`)

### M3 — Security

- PIN lock with argon2 hashing stored in Windows Credential Manager
- Lock screen with PIN input and shake animation on wrong attempt
- First-time PIN setup flow with skip option
- Auto-lock after configurable inactivity (5/15/30/60 minutes)
- Lock when minimized to tray (optional)
- Agent key timeout: auto-clear SSH keys after configurable period
- Lock guard on all sensitive backend commands
- Background timer (15s interval) for inactivity and agent expiry checks
- Persistent audit log at `~/.maze-ssh/audit.log` (JSONL format)
- Audit log viewer with pagination in Settings tab
- Security settings panel: PIN management, timeouts, toggles
- Lock button in custom titlebar

### M2 — Automation

- Per-repo auto-mapping: assign git repositories to SSH profiles
- Git identity sync: auto-set `git config user.name`/`user.email` on activation
- Local vs global scope per mapping
- Repo detection engine: find git root, normalize Windows paths
- Add Repo Mapping dialog with live git root validation
- Mapped repositories section in profile detail view
- Git identity badge in bottom bar showing current `user.name <email>`
- Cascade delete: removing a profile also removes its repo mappings

### M1 — Core MVP

- SSH identity profile management (CRUD) with provider tagging
- SSH key auto-detection in `~/.ssh`
- One-click profile switching from titlebar dropdown
- Windows OpenSSH Agent integration: auto-start service, `ssh-add` key
- `GIT_SSH_COMMAND` set as persistent user environment variable
- Shell-sourceable env file at `~/.maze-ssh/env`
- SSH config generator with marker-based managed section
- Connection test per profile (`ssh -T git@hostname`)
- System tray with minimize-to-tray, tooltip shows active profile
- Activity log with timestamped operations

### UI/UX

- Proton.me-inspired design with dark and light themes
- Custom Windows 11-style titlebar with window controls
- Theme toggle (sun/moon) in titlebar
- Active profile status displayed in titlebar
- Lucide React icons throughout
- Sonner toast notifications for all actions
- Profile edit via modal form
- Responsive layout with scrollable panels

### Infrastructure

- Tauri 2 + React + TypeScript + Tailwind CSS v4
- Zustand state management (6 stores)
- 40+ Tauri commands bridging frontend to Rust backend
- GitHub Actions CI/CD for Windows, macOS (Intel + ARM), Linux
- Production builds: MSI, NSIS, DMG, DEB, AppImage, RPM

## [0.1.0] - 2026-04-11

### Initial Release

- Project scaffolding with Tauri 2 + React + TypeScript
- Basic profile management and SSH agent integration
