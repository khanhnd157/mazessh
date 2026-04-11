# Maze SSH

SSH Identity Orchestrator for Git Workflows

Maze SSH is a desktop application that helps developers manage multiple SSH identities across GitHub, GitLab, Gitea, and other Git providers. Switch between accounts in one click — no more manual `~/.ssh/config` editing or wrong-key pushes.

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
5. **Connection testing** — Verify SSH connectivity per profile directly from the app

## Features

- **Profile Manager** — Create, edit, and delete SSH identity profiles with provider tagging (GitHub, GitLab, Gitea, Bitbucket)
- **SSH Key Scanner** — Auto-detects existing keys in `~/.ssh` during profile creation
- **Quick Switch** — Switch active identity from the titlebar dropdown
- **SSH Agent Management** — Starts Windows OpenSSH Agent automatically, loads only the active key
- **SSH Config Generator** — Marker-based managed section (`BEGIN/END MAZE-SSH MANAGED`) that never corrupts your existing config
- **Connection Test** — Run `ssh -T git@hostname` with the profile's key to verify authentication
- **System Tray** — Minimize to tray, restore with click, tooltip shows active profile
- **Activity Log** — Timestamped log of all SSH operations
- **Dark & Light Theme** — Proton-inspired design with theme toggle
- **Custom Titlebar** — Windows 11-style window controls

## Screenshots

### Dark Theme

![Maze SSH - Dark Theme](images/Maze%20SSH%20-%20Dark%20theme.png)

### Light Theme

![Maze SSH - Light Theme](images/Maze%20SSH%20-%20Light%20theme.png)

## Tech Stack

| Layer             | Technology                                      |
| ----------------- | ----------------------------------------------- |
| Desktop Runtime   | [Tauri 2](https://tauri.app/)                   |
| Backend           | Rust                                            |
| Frontend          | React + TypeScript                              |
| Styling           | Tailwind CSS v4                                 |
| State             | Zustand                                         |
| Icons             | Lucide React                                    |
| Notifications     | Sonner                                          |
| Security          | Windows Credential Manager via `keyring` crate  |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/) (v9+)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Windows 10/11 with OpenSSH installed

### Install & Run

```bash
# Clone the repository
git clone https://github.com/khanhnd/maze-ssh.git
cd maze-ssh

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

### Build for Production

```bash
pnpm tauri build
```

The installer will be generated in `src-tauri/target/release/bundle/`.

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
│   ├── components/               # UI components
│   │   ├── layout/               # TitleBar, Sidebar, MainPanel, BottomBar
│   │   ├── profiles/             # ProfileList, ProfileCard, ProfileDetail, ProfileForm
│   │   ├── switch/               # QuickSwitch, ActiveBadge
│   │   ├── ssh-config/           # ConfigPreview
│   │   └── logs/                 # ActivityLog
│   ├── stores/                   # Zustand stores (profile, app, log, theme)
│   ├── lib/                      # Tauri command wrappers
│   └── types/                    # TypeScript type definitions
│
├── src-tauri/                    # Rust backend
│   └── src/
│       ├── commands/             # 15 Tauri commands
│       ├── models/               # SshProfile, RepoMapping, LogEntry
│       └── services/             # profile_service, ssh_engine, config_engine,
│                                 # key_scanner, security
```

## Roadmap

- [x] **M1 — Core MVP**: Profile CRUD, quick switch, SSH agent integration, config generator
- [ ] **M2 — Automation**: Per-repo auto-mapping, git identity sync (`user.name`/`user.email`)
- [ ] **M3 — Security**: Secure vault, auto-lock, biometric unlock
- [ ] **M4 — Advanced**: Virtual SSH agent, git hooks integration
- [ ] **M5 — Ecosystem**: CLI tool (`maze-ssh use work`), API integration, key generator

## License

MIT

## Author

Built by [@khanhnd](https://github.com/khanhnd)
