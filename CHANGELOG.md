# Changelog

All notable changes to Maze SSH are documented in this file.

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
