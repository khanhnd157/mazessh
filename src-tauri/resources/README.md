# Bridge Resources

## npiperelay.exe

The WSL bridge feature requires `npiperelay.exe` to relay the Windows OpenSSH agent's named pipe into WSL.

### How to obtain

Download the pre-built binary from: https://github.com/jstarks/npiperelay/releases

Or build from source (requires Go):
```bash
go install github.com/jstarks/npiperelay@latest
```

### Where to place

For development: place `npiperelay.exe` in this directory (`src-tauri/resources/`).

At runtime: MazeSSH expects it at `~/.maze-ssh/bin/npiperelay.exe`. Users can place it manually,
or the app will copy it from bundled resources on first use (when bundled in production builds).
