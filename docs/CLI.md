# Maze SSH CLI — User Guide

Command-line interface for Maze SSH, the SSH Identity Orchestrator.

The CLI shares the same profile data as the desktop app (`~/.maze-ssh/`), so profiles created in the GUI are immediately available in the CLI and vice versa.

## Installation

### Build from source

```bash
cd src-tauri
cargo build --bin maze-ssh-cli --no-default-features --release
```

The binary will be at `src-tauri/target/release/maze-ssh-cli.exe`.

### Add to PATH

Copy the binary to a directory in your PATH, or add the build directory:

```bash
# Option 1: Copy to a bin directory
cp src-tauri/target/release/maze-ssh-cli.exe ~/bin/maze-ssh.exe

# Option 2: Add to PATH (PowerShell)
$env:PATH += ";C:\path\to\maze-ssh-cli"
```

## Commands

### List profiles

```bash
maze-ssh-cli list
```

Shows all configured SSH profiles with an active indicator:

```
SSH Profiles
────────────────────────────────────────────────────────────
  ● Personal  github  user@example.com
    key: C:\Users\you\.ssh\gh_ed25519_personal [ACTIVE]
  ○ Work  github  work@company.com
    key: C:\Users\you\.ssh\gh_ed25519_work
```

### Activate a profile

```bash
maze-ssh-cli use Personal
```

This performs all the same operations as clicking "Switch" in the desktop app:

1. Sets the profile as active
2. Loads the SSH key into the system agent (`ssh-add`)
3. Sets `GIT_SSH_COMMAND` as a persistent environment variable
4. Updates global git identity (`user.name` and `user.email`)

```
→ Activating Personal...
  ✓ Key loaded into ssh-agent
  ✓ Git identity: username <user@example.com>

✓ Personal is now active.
```

Profile names are case-insensitive: `maze-ssh-cli use work` and `maze-ssh-cli use Work` both work.

### Auto-switch by repository

```bash
cd ~/projects/my-work-repo
maze-ssh-cli use --auto
```

Detects the git repository in the current directory, looks up the repo mapping, and activates the associated profile:

```
→ Detected repo: my-work-repo → Work
→ Activating Work...
  ✓ Key loaded into ssh-agent
  ✓ Git identity: workuser <work@company.com>

✓ Work is now active.
```

Requires a repo mapping created in the desktop app (Repo Mappings tab).

### Show active profile

```bash
maze-ssh-cli current
```

```
Active: Personal
  Provider: github
  Email:    user@example.com
  Key:      C:\Users\you\.ssh\gh_ed25519_personal
  Alias:    github-personal
```

### Deactivate profile

```bash
maze-ssh-cli off
```

Clears the active profile, removes keys from the SSH agent, and clears the `GIT_SSH_COMMAND` environment variable:

```
✓ Profile deactivated. Agent keys cleared.
```

### Check status

```bash
maze-ssh-cli status
```

Shows a summary of the current SSH environment:

```
Maze SSH Status
──────────────────────────────────────────────────
  Profile: Personal (github)
  Agent: 256 SHA256:abc... C:\Users\you\.ssh\gh_ed25519_personal (ED25519)
  Git: username <user@example.com>
  Profiles: 2
  Mappings: 3
```

### Test SSH connection

```bash
# Test active profile
maze-ssh-cli test

# Test a specific profile
maze-ssh-cli test Work
```

Runs `ssh -T git@hostname` with the profile's key to verify authentication:

```
→ Testing connection for Personal...
  ✓ Hi username! You've successfully authenticated, but GitHub does not provide shell access.
```

### SSH config management

#### Preview generated config

```bash
maze-ssh-cli config preview
```

Shows the SSH config that Maze SSH would generate for all profiles.

#### Write config to file

```bash
maze-ssh-cli config write
```

Writes the generated config to `~/.ssh/config`, automatically creating a backup first:

```
  ✓ Backup: C:\Users\you\.ssh\config.backup.20260412_143022
  ✓ SSH config written to ~/.ssh/config
```

Only the section between `# === BEGIN MAZE-SSH MANAGED ===` and `# === END MAZE-SSH MANAGED ===` markers is modified. Your manual entries outside the markers are preserved.

#### List backups

```bash
maze-ssh-cli config backups
```

```
SSH Config Backups
────────────────────────────────────────────────────────────
  config.backup.20260412_143022 (2026-04-12 14:30:22, 1.2 KB)
  config.backup.20260411_201500 (2026-04-11 20:15:00, 0.8 KB)
```

### Export profiles

```bash
maze-ssh-cli export > profiles.json
```

Exports all profiles as JSON to stdout. Useful for backup or migration. Does not include passphrases or secrets.

### Import profiles

```bash
maze-ssh-cli import profiles.json
```

Imports profiles from a JSON file. Merges with existing profiles, skipping any with duplicate names:

```
  ✓ Imported 'New Profile'
  – Skipping 'Personal' (already exists)

✓ 1 profile(s) imported.
```

## Shell Integration

### Add to .bashrc / .zshrc

To automatically activate the last-used profile when opening a new terminal:

```bash
# Source Maze SSH environment
if [ -f ~/.maze-ssh/env ]; then
  source ~/.maze-ssh/env
fi
```

### Auto-switch in shell prompt

Add to your shell config to auto-switch when changing directories:

```bash
cd() {
  builtin cd "$@" && maze-ssh-cli use --auto 2>/dev/null
}
```

## Data Location

All data is stored at `~/.maze-ssh/`:

| File | Purpose |
| ---- | ------- |
| `profiles.json` | Profile definitions |
| `active.txt` | Currently active profile ID |
| `repo_mappings.json` | Repository-to-profile mappings |
| `settings.json` | Security settings |
| `env` | Shell-sourceable environment file |
| `audit.log` | Activity audit trail |

## Troubleshooting

### "No profiles configured"

Create profiles in the Maze SSH desktop app first, or import from a JSON file.

### "Not inside a git repository" (use --auto)

The `--auto` flag requires running from within a git repository that has a repo mapping configured.

### SSH agent not starting

The CLI starts the Windows OpenSSH Authentication Agent service automatically. If it fails, start it manually:

```powershell
Start-Service ssh-agent
```

### Profile switch doesn't affect current terminal

New environment variables only apply to terminals opened after the switch. For the current terminal, source the env file:

```bash
source ~/.maze-ssh/env
```
