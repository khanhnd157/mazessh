# Maze SSH

SSH Identity Orchestrator for Git Workflows

Maze SSH is a desktop application + CLI tool that helps developers manage multiple SSH identities across GitHub, GitLab, Gitea, and other Git providers. Switch between accounts in one click — no more manual `~/.ssh/config` editing or wrong-key pushes.

## The Problem

Developers working with multiple Git accounts face:

- SSH agent loading multiple keys, causing wrong key selection
- Manual `~/.ssh/config` editing for each account
- Accidentally pushing with the wrong identity
- No quick way to switch context between projects

## How Maze SSH Solves It

1. **One-click switch** — Select a profile, and your entire Git workflow uses the correct SSH key immediately
2. **SSH Agent integration** — Automatically manages the Windows OpenSSH Agent (`ssh-add`), so any terminal session picks up the active key
3. **Environment injection** — Sets `GIT_SSH_COMMAND` as a persistent user environment variable for all new terminals
4. **SSH Config generation** — Auto-generates `~/.ssh/config` entries with host aliases, preserving your existing config
5. **CLI tool** — `maze-ssh-cli use work` from any terminal to switch profiles instantly

## Features

### Desktop App

- **Profile Manager** — Create, edit, and delete SSH identity profiles with provider tagging
- **SSH Key Scanner** — Auto-detects existing keys in `~/.ssh` during profile creation
- **Quick Switch** — Switch active identity from the titlebar dropdown
- **SSH Agent Management** — Starts Windows OpenSSH Agent automatically, loads only the active key
- **Repo Mapping** — Auto-switch profiles based on repository directory with git identity sync
- **SSH Config Generator** — Marker-based managed section that never corrupts your existing config
- **Git Hooks** — Pre-push identity validation to prevent wrong-account pushes
- **Config Rollback** — Backup history with one-click restore for SSH config
- **Profile Export/Import** — Backup and migrate profiles as JSON
- **Key Fingerprints** — View SSH key fingerprint (SHA256) for each profile
- **PIN Lock** — Protect profiles with PIN (Argon2 hashed, Windows Credential Manager)
- **Auto-Lock** — Lock on inactivity timeout or minimize to tray
- **Agent Key Timeout** — Auto-clear SSH keys from agent after configurable period
- **Audit Log** — Persistent log of all security-sensitive operations
- **Dark & Light Theme** — Proton-inspired design with theme toggle
- **Connection Test** — Verify SSH connectivity per profile
- **System Tray** — Minimize to tray, tooltip shows active profile

### CLI Tool (`maze-ssh-cli`)

- `maze-ssh-cli list` — List all profiles
- `maze-ssh-cli use <name>` — Activate a profile
- `maze-ssh-cli use --auto` — Auto-switch based on current directory
- `maze-ssh-cli current` — Show active profile
- `maze-ssh-cli off` — Deactivate and clear agent keys
- `maze-ssh-cli status` — Show agent, git identity, and profile status
- `maze-ssh-cli test` — Test SSH connection
- `maze-ssh-cli config preview/write/backups` — Manage SSH config
- `maze-ssh-cli export/import` — Backup and restore profiles

## Screenshots

### Dark Theme

![Maze SSH - Dark Theme](images/Maze%20SSH%20-%20Dark%20theme.png)

### Light Theme

![Maze SSH - Light Theme](images/Maze%20SSH%20-%20Light%20theme.png)

## Documentation

- [User Guide (English)](docs/USER_GUIDE.md)
- [User Guide (Vietnamese)](docs/USER_GUIDE_VI.md)
- [CLI Reference](docs/CLI.md)

## Tech Stack

| Layer           | Technology                                     |
| --------------- | ---------------------------------------------- |
| Desktop Runtime | [Tauri 2](https://tauri.app/)                  |
| Backend         | Rust                                           |
| Frontend        | React + TypeScript                             |
| Styling         | Tailwind CSS v4                                |
| State           | Zustand                                        |
| CLI             | Clap + Colored                                 |
| Security        | Windows Credential Manager via `keyring` crate |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/) (v9+)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Windows 10/11 with OpenSSH installed

### Install & Run

```bash
# Clone the repository
git clone https://github.com/khanhnd157/mazessh.git
cd mazessh

# Install dependencies
pnpm install

# Run desktop app in development mode
pnpm tauri dev

# Build CLI tool
cd src-tauri
cargo build --bin maze-ssh-cli --no-default-features --release
```

### Build for Production

```bash
# Desktop app (generates MSI + NSIS installers)
pnpm tauri build

# CLI tool
cd src-tauri && cargo build --bin maze-ssh-cli --no-default-features --release
```

## How It Works

When you activate a profile, Maze SSH performs three actions:

1. **Writes `~/.maze-ssh/env`** — Shell-sourceable file with `GIT_SSH_COMMAND`
2. **Sets user environment variable** — `GIT_SSH_COMMAND` persisted via Windows registry, picked up by all new terminals
3. **Updates SSH Agent** — Starts `ssh-agent` service if needed, clears existing keys, adds only the active profile's key via `ssh-add`

This means after switching:

- `ssh-add -l` in any terminal shows the correct key
- `git push` uses the correct identity
- No manual configuration needed

## Project Structure

```text
maze-ssh/
├── src/                          # React frontend
│   ├── components/               # UI components (layout, profiles, repos, security, settings)
│   ├── stores/                   # Zustand stores (profile, app, log, theme, security, repoMapping, ui)
│   ├── hooks/                    # useInactivityTracker, useKeyboardShortcuts, useConfirm
│   ├── lib/                      # Tauri command wrappers (40+ commands)
│   └── types/                    # TypeScript type definitions
│
├── src-tauri/                    # Rust backend
│   └── src/
│       ├── bin/cli.rs            # CLI binary (maze-ssh-cli)
│       ├── commands/             # 40+ Tauri commands
│       ├── models/               # SshProfile, RepoMapping, SecuritySettings, AuditEntry
│       └── services/             # Shared business logic (used by both desktop and CLI)
│
├── docs/                         # User guides (EN + VI) and CLI reference
└── .github/workflows/            # CI/CD for all platforms
```

## License

MIT

## Author

Built by [@khanhnd157](https://github.com/khanhnd157)
