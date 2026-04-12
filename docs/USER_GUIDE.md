# Maze SSH — User Guide

A guide to using the Maze SSH desktop application for managing SSH identities in Git workflows.

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Main Interface](#main-interface)
4. [Managing Profiles](#managing-profiles)
5. [Switching Profiles](#switching-profiles)
6. [Repo Mapping](#repo-mapping)
7. [SSH Config](#ssh-config)
8. [Security](#security)
9. [Keyboard Shortcuts](#keyboard-shortcuts)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

Maze SSH helps developers manage multiple SSH identities (GitHub, GitLab, Gitea, Bitbucket) on the same machine. Instead of manually editing `~/.ssh/config`, you can switch between accounts with a single click.

### What happens when you switch a profile?

1. Loads the SSH key into the Windows SSH Agent (`ssh-add`)
2. Sets the `GIT_SSH_COMMAND` environment variable for all new terminals
3. Updates `git config user.name` and `user.email`
4. Writes an env file at `~/.maze-ssh/env` for the current terminal to source

Result: all `git push`, `git pull`, and `git clone` operations use the correct SSH key.

---

## Installation

### System Requirements

- Windows 10/11 (or macOS, Linux)
- OpenSSH installed (built into Windows 10+)
- Git installed

### Download and Install

Download the installer from [GitHub Releases](https://github.com/khanhnd157/mazessh/releases):

| OS                    | File                                                      |
| --------------------- | --------------------------------------------------------- |
| Windows               | `Maze.SSH_x64-setup.exe` or `Maze.SSH_x64_en-US.msi`     |
| macOS Intel           | `Maze.SSH_x64.dmg`                                       |
| macOS Apple Silicon   | `Maze.SSH_aarch64.dmg`                                   |
| Linux (Debian/Ubuntu) | `Maze.SSH_amd64.deb`                                     |
| Linux (Fedora/RHEL)   | `Maze.SSH-x86_64.rpm`                                    |
| Linux (Universal)     | `Maze.SSH_amd64.AppImage`                                |

### After Installation

The application automatically:

- Scans for existing SSH keys in `~/.ssh/`
- Starts the Windows SSH Agent service if not running
- Appears in the system tray

---

## Main Interface

```text
+----------------------------------------------------------+
| [Logo] Maze SSH | * Profile Name  Provider  [Switch] ... | <- Titlebar
+---------+----------------------------------------------+-+
| PROFILES|  Profiles  | Repo Mappings | SSH Config | ...  | <- Tabs
|         +------------------------------------------------+
| * Prof1 |                                                |
| o Prof2 |          Profile Detail / Tab Content          | <- Main
|         |                                                |
+---------+------------------------------------------------+
| Activity Log                                             | <- Bottom
| Git: username <email>                                    |
+----------------------------------------------------------+
```

### Titlebar

- **Status**: displays the active profile (green pulsing dot) or "No active profile"
- **Switch**: dropdown for quick profile switching
- **Deactivate**: turns off the current profile
- **Lock**: locks the application (when PIN is configured)
- **Theme**: toggles Dark/Light mode
- **Window controls**: Minimize, Maximize, Close (hides to tray)

### Sidebar

Lists all profiles. Click a profile to view its details. The **+ New** button creates a new profile.

### Tabs

- **Profiles**: view and manage profile details
- **Repo Mappings**: assign repositories to profiles
- **SSH Config**: view, write, and rollback SSH config
- **Settings**: security, PIN, timeouts

### Bottom Bar

- **Activity Log**: history of operations (switch, test, lock...)
- **Git identity**: displays the current `user.name <user.email>`

---

## Managing Profiles

### Creating a new profile

1. Click **+ New** in the sidebar
2. Fill in the details:
   - **Profile Name**: display name (e.g., "Work GitHub")
   - **Provider**: select GitHub, GitLab, Gitea, or Bitbucket
   - **Email**: email associated with the Git account
   - **Git Username**: Git username (shown in commits)
   - **SSH Private Key**: path to the private key
     - The app automatically scans `~/.ssh/` and lists detected keys
     - Click a key to select it quickly
   - **Host Alias**: alias name for SSH config (auto-generated from Profile Name)
   - **Hostname**: server address (auto-filled based on Provider)
3. Click **Create Profile**

### Editing a profile

1. Select the profile in the sidebar
2. Click **Edit** in the detail view
3. Modify the information and click **Save Changes**

### Deleting a profile

1. Select the profile in the sidebar
2. Click **Delete**
3. Confirm in the dialog. The profile and all associated repo mappings will be removed.

### Viewing profile details

The profile detail page displays:

- **Host Alias** and **Hostname**: SSH connection info
- **SSH User** and **Port**: defaults to `git` and `22`
- **Git Username** and **Key Type**: identity information
- **SSH Private Key**: key path (hover to copy)
- **Key Fingerprint**: SHA256 hash and key type (ED25519, RSA...)
- **Mapped Repositories**: list of repos assigned to this profile

### Testing the connection

Click **Test Connection** to verify that the SSH key can connect to the server:

- **Success**: green message with the authenticated username
- **Failure**: red message with error details

### Export / Import

In the **Settings** tab:

- **Export to Clipboard**: copies all profiles as JSON (does not include passphrases)
- **Import from Clipboard**: paste JSON to import new profiles (skips duplicates by name)

---

## Switching Profiles

### Method 1: Switch from Titlebar (fastest)

1. Click **Switch** on the titlebar
2. Select a profile from the dropdown
3. The app automatically: loads key, sets env, syncs git identity

### Method 2: Activate from Profile Detail

1. Select a profile in the sidebar
2. Click **Activate**

### Method 3: Keyboard shortcut

Use **Ctrl+L** to lock, **Ctrl+1-4** to switch tabs.

### What happens after switching?

- **New terminals**: automatically use the correct key (via `GIT_SSH_COMMAND` environment variable)
- **Currently open terminals**: run `source ~/.maze-ssh/env` or open a new terminal
- **ssh-add -l**: shows the currently active key
- **git push/pull**: uses the correct identity

### Deactivate

Click **Deactivate** on the titlebar or use the CLI `maze-ssh-cli off`:

- Removes the key from the SSH agent
- Clears the `GIT_SSH_COMMAND` environment variable
- Does not affect SSH keys on disk

---

## Repo Mapping

Automatically switch profiles based on the repository directory.

### Creating a mapping

1. Switch to the **Repo Mappings** tab
2. Click **Add Mapping**
3. Enter the repository path (the app validates whether it is a git repo)
4. Select a profile
5. Choose the scope:
   - **Local**: sets `git config` for this repo only (recommended)
   - **Global**: sets `git config --global`
6. Click **Create Mapping**

### Installing a Git Hook

On each mapping card, hover to reveal the **Git Branch** icon. Click to install a pre-push hook:

- The hook checks `git config user.email` before each `git push`
- If the email does not match the profile, the push is blocked with a warning
- Only removes hooks created by Maze SSH

### Removing a mapping

Hover over the mapping card, click the **Trash** icon, and confirm.

---

## SSH Config

The **SSH Config** tab manages the `~/.ssh/config` file.

### Preview

View the SSH config that will be generated from all profiles:

```text
# === BEGIN MAZE-SSH MANAGED ===
Host github-work
  HostName github.com
  User git
  IdentityFile C:\Users\you\.ssh\gh_ed25519_work
  IdentitiesOnly yes
# === END MAZE-SSH MANAGED ===
```

### Write Config

Click **Write Config** to write to `~/.ssh/config`:

- Automatically creates a backup before writing
- Only modifies the section between `BEGIN/END MAZE-SSH MANAGED` markers
- Your manually written content outside the markers is preserved

### Current

View the current contents of `~/.ssh/config`.

### Backups

View the list of backups with timestamps and sizes. Click **Rollback** to restore any backup (the current config is backed up before rolling back).

---

## Security

### Setting up a PIN

1. Switch to the **Settings** tab
2. Under **PIN Protection**, click **Set PIN**
3. Enter a PIN (minimum 4 characters) and confirm
4. The PIN is hashed with Argon2 and stored in Windows Credential Manager

### Locking the application

- **Manual**: click the **Lock** icon on the titlebar or press **Ctrl+L**
- **Automatic**: configure a timeout in Settings (5, 15, 30, or 60 minutes of inactivity)
- **On minimize**: enable "Lock when minimized to tray" in Settings

When locked:

- The lock screen covers the entire interface
- SSH agent keys are cleared
- All operations are blocked until the correct PIN is entered

### Failed PIN attempt limit

- Maximum of 5 consecutive failed attempts
- After 5 failures, wait 60 seconds before trying again

### Agent Key Timeout

Configure in Settings under **Agent Key Timeout**:

- After the configured time, SSH keys are automatically removed from the agent
- Independent of the application lock
- The profile is automatically deactivated when keys expire

### Changing / Removing PIN

In Settings under **PIN Protection**:

- **Change PIN**: enter old PIN + new PIN
- **Remove PIN**: enter PIN to confirm, disables the lock feature

### Audit Log

All security-related operations are logged to `~/.maze-ssh/audit.log`:

- Lock / Unlock (successful and failed)
- PIN changes
- Settings changes
- Agent key expirations

View in Settings, **Audit Log** section, click **View Log**.

---

## Keyboard Shortcuts

| Shortcut   | Action                    |
| ---------- | ------------------------- |
| **Ctrl+1** | Switch to Profiles tab    |
| **Ctrl+2** | Switch to Repo Mappings   |
| **Ctrl+3** | Switch to SSH Config tab  |
| **Ctrl+4** | Switch to Settings tab    |
| **Ctrl+L** | Lock the application      |
| **Escape** | Close dialog / dropdown   |

---

## Troubleshooting

### "ssh-add -l" does not show the key after switching

The current terminal needs to be refreshed. Open a new terminal or run:

```bash
source ~/.maze-ssh/env
```

### SSH Agent service fails to start

Open PowerShell as Administrator and run:

```powershell
Set-Service ssh-agent -StartupType Manual
Start-Service ssh-agent
```

### Pushing with the wrong account

1. Check the active profile: look at the titlebar or run `maze-ssh-cli current`
2. Switch to the correct profile
3. Verify git identity: `git config user.email`
4. Install a git hook to prevent future mistakes: Repo Mappings tab, hover a mapping, click the Git Branch icon

### SSH config corrupted after writing

1. Go to SSH Config tab and click **Backups**
2. Select the most recent backup and click **Rollback**
3. Content outside the `MAZE-SSH MANAGED` markers is always preserved

### Application is locked and PIN is forgotten

The PIN is stored in Windows Credential Manager. To reset:

1. Open **Credential Manager** in Windows (Control Panel, then Credential Manager)
2. Select the **Windows Credentials** tab
3. Find the entry `maze-ssh / pin-hash`
4. Delete that entry
5. Restart the application. The PIN will be reset.

### System tray icon is not visible

The application minimizes to the tray when the window is closed. Click the Maze SSH icon in the system tray to reopen it. If the icon is not visible, check the hidden icons area on the taskbar.

---

## Data Storage

All data is stored at `~/.maze-ssh/`:

| File                 | Contents                                                            |
| -------------------- | ------------------------------------------------------------------- |
| `profiles.json`      | Profile information (does not contain private keys, only paths)     |
| `active.txt`         | ID of the currently active profile                                  |
| `repo_mappings.json` | Repository to profile mappings                                      |
| `settings.json`      | Security settings (timeouts, lock-on-minimize)                      |
| `env`                | Shell-sourceable env file                                           |
| `audit.log`          | Security audit trail                                                |

### Data Security

- **Private keys**: NOT stored in the application, only file paths are saved
- **Passphrases**: stored in Windows Credential Manager (encrypted by the OS)
- **PIN**: Argon2 hash, stored in Windows Credential Manager
- **Profiles**: plaintext JSON containing only metadata, no secrets
