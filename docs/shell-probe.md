# VibeRot Shell Probe Implementation

## Overview

This implementation adds POSIX support to VibeRot through a shell-based probe that monitors command execution in bash and zsh shells. This approach was chosen as the initial Linux (and macOS) implementation because:

- **No special privileges required**: Works without root access or kernel modifications
- **Simple setup**: Easy to install and configure 
- **Cross-shell compatibility**: Supports both bash and zsh
- **Accessible development**: No complex kernel/userspace integration needed

## How It Works

### Architecture
1. **Shell Hook Installation**: Modifies `.bashrc`/`.zshrc` with monitoring functions
2. **Unix Socket Communication**: Shell hooks communicate with VibeRot service via JSON messages
3. **Session-Based Tracking**: Uses shell session IDs to match command start/end events
4. **Automatic Action Management**: Triggers and terminates actions based on shell commands

### Shell Integration
The probe installs these components in your shell:

- **Pre-command hook**: Captures command before execution
- **Post-command hook**: Captures exit codes after execution
- **Communication functions**: Send JSON messages to VibeRot service
- **Session tracking**: Unique identifiers for each command execution

### Message Protocol
Shell hooks send JSON messages like:
```json
{
    "session_id": "viberot_12345_1234567890123456789",
    "event_type": "CommandStart",
    "command": "cargo build --release",
    "working_directory": "/home/user/project",
    "environment": {}
}
```

## Installation and Setup

### 1. Build VibeRot
```bash
cargo build --release
```

### 2. Run VibeRot Service
```bash
./target/release/viberot-service
```

### 3. Shell Hook Installation
On first run, VibeRot will prompt to install shell hooks:

```
🎉 Welcome to VibeRot Shell Integration Setup!
═══════════════════════════════════════════════

To monitor shell commands, VibeRot needs to install hooks in your shell configuration.
This will add a few functions to your .bashrc or .zshrc file.

Would you like to proceed with automatic installation? [y/N]:
```

**Automatic Installation**: Choose 'y' to automatically append hooks to your shell config.

**Manual Installation**: Choose 'n' to get instructions for manual setup.

### 4. Reload Shell Configuration
After installation:
```bash
# For bash users
source ~/.bashrc

# For zsh users  
source ~/.zshrc

# Or simply restart your terminal
```

## Features and Capabilities

### ✅ Implemented Features
- **Command monitoring**: Detects when shell commands start and finish
- **Working directory capture**: Knows where commands are executed
- **Session-based tracking**: Matches command starts with their completions
- **Shell compatibility**: Works with bash and zsh
- **User consent**: Only installs hooks with explicit user approval
- **Clean uninstall**: Easy removal of shell hooks

### 🚧 Current Limitations
- **Shell-only monitoring**: Only captures commands run in instrumented shells
- **No system-wide coverage**: Won't detect background processes or GUI applications
- **Shell restart required**: Changes take effect after reloading shell configuration

## Troubleshooting

### Shell Hooks Not Working
1. Verify hooks are installed in your shell config:
   ```bash
   grep -A 5 "VibeRot Shell Integration" ~/.bashrc ~/.zshrc
   ```

2. Check that the socket path is accessible:
   ```bash
   ls -la /run/user/$(id -u)/viberot-shell.sock
   # or
   ls -la /tmp/viberot-shell.sock
   ```

3. Test communication tools:
   ```bash
   # Check if nc or socat are available
   which nc socat
   ```

### VibeRot Service Issues
1. Check service logs in `~/.local/share/viberot-service/logs/`
2. Ensure config file exists at `~/.config/viberot-service/config.toml`
3. Verify no permission issues with socket creation
