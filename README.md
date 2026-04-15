# Maze SSH

SSH Identity Orchestrator for Git Workflows

[![Release](https://img.shields.io/github/v/release/khanhnd157/mazessh?style=flat-square&label=release&color=4f46e5)](https://github.com/khanhnd157/mazessh/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078d4?style=flat-square&logo=windows)](https://github.com/khanhnd157/mazessh/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/khanhnd157/mazessh/release.yml?style=flat-square&label=build)](https://github.com/khanhnd157/mazessh/actions)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-ffc131?style=flat-square&logo=tauri)](https://tauri.app)

Manage multiple SSH identities across GitHub, GitLab, Gitea, and any Git provider — with encrypted key storage, a native Windows SSH agent, and WSL integration. Switch accounts in one click. No more wrong-key pushes.

[Download](https://github.com/khanhnd157/mazessh/releases/latest) · [User Guide](docs/USER_GUIDE.md) · [CLI Reference](docs/CLI.md) · [Changelog](CHANGELOG.md)

---

## The Problem

Every developer working across multiple Git accounts knows the friction:

- SSH agent loads all keys at once — wrong identity gets picked
- Manual `~/.ssh/config` editing every time you onboard a new account
- `git push` succeeds, then you realize it pushed under the wrong user
- WSL doesn't share Windows SSH keys without manual socket forwarding
- Private keys sit unencrypted on disk

Maze SSH solves all of this in a single desktop app.

---

## Features

### SSH Key Vault

Store private keys encrypted at rest — they never touch disk unprotected.

- **AES-256-GCM** encryption with **Argon2id** KDF (64 MiB / 3 iterations)
- Two-layer key hierarchy: passphrase → VMK → VEK → per-key encrypted files
- Password change re-encrypts only the master key, not individual key files
- Generate **Ed25519** or **RSA 4096** keys without leaving the app
- Import existing **OpenSSH PEM** keys (encrypted or plaintext)
- Per-key **export policy**: allow or deny private key export
- Per-key **allowed hosts**: restrict which SSH hostnames a key can sign for
- **Archive** keys without deleting; restore at any time
- Vault state hidden from frontend while app is locked

### Native SSH Agent

A Windows SSH agent running on a named pipe — compatible with any SSH client.

- Listens on `\\.\pipe\maze-ssh-agent`
- Implements SSH agent protocol: `request-identities`, `sign-request`, `remove-all-identities`
- **DACL-restricted** named pipe — only SYSTEM and the current user can connect; no other local processes
- **Consent popup** on each sign request: shows key name, fingerprint, and requesting process (name + path + PID)
- 60-second consent timeout with automatic denial and audit log entry
- **Policy Engine**: approve once, for this session, or always (persisted to `~/.maze-ssh/policy-rules.json`)
- Semaphore cap (100 concurrent clients) and 1 MB read buffer cap to prevent DoS
- Constant-time key blob comparison to prevent timing attacks

### Profile Management

- Create SSH identity profiles with provider tagging (GitHub, GitLab, Gitea, Bitbucket, custom)
- Link a profile to a vault key or an external key file
- **One-click activation** — correct key loads into agent, `GIT_SSH_COMMAND` is set globally
- **Connection test** per profile (`ssh -T`)
- **SSH Config Generator** — marker-based managed section that never corrupts your existing `~/.ssh/config`
- **Config rollback** — backup history with one-click restore
- **Profile export/import** — JSON backup and migration

### Repo Mapping & Git Integration

- Map git repositories to profiles — auto-switch when you `cd` into a repo
- Sync `git config user.name`/`user.email` on activation (local or global scope)
- **Git hooks** — pre-push identity validation to prevent wrong-account pushes
- Git identity badge in the status bar showing current `user.name <email>`

### WSL Bridge

Forward the Windows SSH agent socket into any WSL distribution.

- Per-distro bridge status with bootstrap, start, stop, restart in one click
- Multi-provider support: OpenSSH Agent, 1Password, Pageant, or any custom Unix socket
- Relay modes: systemd service or background daemon
- **Watchdog** with configurable `max_restarts` (1–20) — auto-restart on failure
- Binary auto-download: fetches `npiperelay` for Windows + relay script for Linux
- Diagnostics panel with one-click auto-fix for common relay failures
- **Bootstrap All** — provision every configured distro in one operation
- Adds `Host maze-wsl-<distro>` block to `~/.ssh/config` for direct `ssh maze-wsl-ubuntu` access
- Bridge config export/import as JSON

### CLI Tool

A standalone binary (`maze-ssh-cli`) that shares profile data with the desktop app.

```text
maze-ssh-cli list                      List all profiles
maze-ssh-cli use <name>                Activate a profile
maze-ssh-cli use --auto                Auto-switch based on current directory
maze-ssh-cli current                   Show active profile
maze-ssh-cli off                       Deactivate and clear agent
maze-ssh-cli status                    Show agent, git identity, and profile status
maze-ssh-cli test                      Test SSH connectivity
maze-ssh-cli config preview|write      Preview or apply SSH config changes
maze-ssh-cli export|import             Backup and restore profiles
maze-ssh-cli bridge list|bootstrap     Manage WSL bridges
```

### Security

- **PIN lock** — Argon2-hashed PIN stored in Windows Credential Manager
- **Auto-lock** — on inactivity timeout or minimize to tray
- **Agent key timeout** — auto-clear SSH keys after configurable period
- **Audit log** — persistent JSONL log (`~/.maze-ssh/audit.log`) of all security-sensitive operations, rotated at 1 MB
- All passphrase and key material memory is **zeroized** on drop
- Error messages sanitized before reaching the frontend — no filesystem paths exposed to the UI

---

## Screenshots

### Dark Theme

![Maze SSH - Dark Theme](images/Maze%20SSH%20-%20Dark%20theme.png)

### Light Theme

![Maze SSH - Light Theme](images/Maze%20SSH%20-%20Light%20theme.png)

---

## Getting Started

### Prerequisites

| Requirement                                        | Version              |
| -------------------------------------------------- | -------------------- |
| [Rust](https://www.rust-lang.org/tools/install)    | latest stable        |
| [Node.js](https://nodejs.org/)                     | 18+                  |
| [pnpm](https://pnpm.io/)                           | 9+                   |
| Windows                                            | 10 / 11 with OpenSSH |

OpenSSH ships with Windows 10 1809+ under **Settings → Optional Features**. If it is not installed, Maze SSH will prompt you.

### Development

```bash
# Clone
git clone https://github.com/khanhnd157/mazessh.git
cd mazessh

# Install frontend dependencies
pnpm install

# Run desktop app with hot reload (Tauri + Vite)
pnpm tauri dev

# Build CLI only (no Tauri dependency)
cargo build --bin maze-ssh-cli --no-default-features --release \
  --manifest-path src-tauri/Cargo.toml
```

### Production Build

```bash
# Desktop app — MSI + NSIS installers in src-tauri/target/release/bundle/
pnpm tauri build

# CLI binary
cargo build --bin maze-ssh-cli --no-default-features --release \
  --manifest-path src-tauri/Cargo.toml
```

Pre-built binaries for Windows (MSI, NSIS), macOS (DMG), and Linux (DEB, AppImage, RPM) are available on the [Releases page](https://github.com/khanhnd157/mazessh/releases/latest).

---

## How It Works

### Profile Activation Flow

```text
User clicks "Activate"
  → [Rust — fast path] Update state + save to disk + write ~/.maze-ssh/env
  → [Rust — background] Set GIT_SSH_COMMAND in Windows registry
                         Start ssh-agent service if stopped
                         ssh-add the active key
                         Sync git config user.name / user.email
  → [Frontend] Toast notification + audit log entry
```

### Vault Signing Flow (MazeSSH Agent mode)

```text
SSH client connects to \\.\pipe\maze-ssh-agent
  → Pipe DACL check (SYSTEM or current user only)
  → SSH agent protocol handshake
  → sign-request received → policy lookup
      Policy: Once    → consent popup shown (60 s timeout)
      Policy: Session → cached approval, sign immediately
      Policy: Always  → persisted rule, sign immediately
  → Key decrypted from vault in-process using VEK
  → Signature returned → PEM bytes zeroized from memory
```

---

## Project Structure

```text
mazessh/
├── crates/
│   ├── maze-crypto/          # AES-256-GCM + Argon2id primitives
│   ├── maze-vault/           # SSH key vault (CRUD, encrypt/decrypt, session)
│   └── maze-agent-protocol/  # SSH agent wire protocol (de)serialization
│
├── src/                      # React frontend
│   ├── components/           # UI components (profiles, vault, repos, bridge, security, settings)
│   ├── stores/               # Zustand stores (7 stores)
│   ├── hooks/                # useInactivityTracker, useKeyboardShortcuts, useConfirm
│   ├── lib/                  # Tauri command wrappers (60+ commands)
│   └── types/                # TypeScript type definitions
│
├── src-tauri/
│   └── src/
│       ├── bin/cli.rs        # maze-ssh-cli binary
│       ├── commands/         # Tauri IPC commands (bridge, vault, security, switch, …)
│       ├── models/           # SshProfile, RepoMapping, SecuritySettings, AuditEntry, …
│       └── services/         # Business logic shared by desktop + CLI
│
├── docs/                     # User guides (EN + VI), CLI reference, security audit
└── .github/workflows/        # CI/CD — builds all platform installers on tag push
```

---

## Tech Stack

| Layer            | Technology                                              |
| ---------------- | ------------------------------------------------------- |
| Desktop runtime  | [Tauri 2](https://tauri.app/)                           |
| Backend language | Rust (stable)                                           |
| Frontend         | React 19 + TypeScript                                   |
| Styling          | Tailwind CSS v4                                         |
| State management | Zustand 5                                               |
| CLI              | Clap 4 + Colored                                        |
| Cryptography     | AES-256-GCM (`aes-gcm`), Argon2id (`argon2`), `zeroize` |
| SSH primitives   | `ssh-key` 0.6                                           |
| Windows security | `windows-sys` (DACL, named pipe, process query)         |
| Keychain         | `keyring` (Windows Credential Manager)                  |

---

## Documentation

- [User Guide — English](docs/USER_GUIDE.md)
- [User Guide — Vietnamese](docs/USER_GUIDE_VI.md)
- [CLI Reference](docs/CLI.md)
- [Changelog](CHANGELOG.md)
- [Security Audit](docs/SECURITY_AUDIT_v2.md)

---

## License

[MIT](LICENSE) — Copyright (c) 2026 Duy Khanh

Built by [@khanhnd157](https://github.com/khanhnd157)
