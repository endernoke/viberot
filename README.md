# Command-Sidekick

A configurable utility to run actions based on command execution. Add some fun to your workflow by triggering custom actions (like UI spinners, notifications, or scripts) when specific commands are running.

## Features

- **Cross-platform**: Designed for Windows, Linux, and macOS (Windows MVP implemented)
- **Configurable**: Simple TOML configuration for defining command patterns and actions
- **Lightweight**: Minimal resource usage with efficient process monitoring
- **Extensible**: Support for external executables and embedded Lua scripts

## Quick Start (Windows)

### Prerequisites

- Windows 10/11
- .NET 8.0 Runtime (for the probe and spinner components)
- Administrator privileges (for installation)

### Installation

1. **Build the project:**
   ```cmd
   build.bat
   ```

2. **Install as Windows services:**
   ```cmd
   install.bat
   ```
   *Note: Run as Administrator*

3. **Test it out:**
   ```cmd
   npm install
   ```
   You should see a spinner UI appear while the command runs!

### Configuration

Edit the configuration file at `%USERPROFILE%\.config\command-sidekick\config.toml`:

```toml
[[rule]]
command = "npm install*"
[rule.action]
type = "exec"
path = "C:\\Program Files\\Command-Sidekick\\SpinnerPlugin.exe"

[[rule]]
command = "docker build*"
[rule.action]
type = "exec"
path = "C:\\Program Files\\Command-Sidekick\\SpinnerPlugin.exe"
```

### Uninstallation

```cmd
uninstall.bat
```

## Architecture

### Core Service (Rust)
- Loads configuration and manages rules
- Orchestrates action plugins
- Monitors process lifecycle

### Platform Probe (.NET on Windows)
- Monitors process creation events using WMI
- Sends events to Core Service via named pipes

### Action Plugins
- **Spinner Plugin**: WPF application showing a loading animation
- **Custom Plugins**: Any executable that responds to stdin closure

## Development

### Building

```cmd
# Build all components
build.bat

# Or build individually:
cargo build --release                          # Rust core
cd windows-probe && dotnet build -c Release   # .NET probe  
cd spinner-plugin && dotnet build -c Release  # WPF spinner
```

### Running in Development

1. Start the Core Service:
   ```cmd
   target\release\sidekick-core.exe
   ```

2. Start the Windows Probe:
   ```cmd
   windows-probe\bin\Release\net8.0-windows\WindowsProbe.exe
   ```

3. Run a monitored command:
   ```cmd
   npm install
   ```

## Configuration Reference

### Rule Format

```toml
[[rule]]
command = "pattern*"     # Glob pattern to match commands
[rule.action]
type = "exec"           # Action type: "exec" or "lua"
path = "path/to/exe"    # For exec actions
# script = "lua code"   # For lua actions (future)
```

### Environment Variables (for plugins)

When an action is triggered, plugins receive:
- `SIDEKICK_PID`: Process ID of the monitored command
- `SIDEKICK_COMMAND`: Full command line
- `SIDEKICK_TIMESTAMP`: Unix timestamp when detected

### Plugin Protocol

Action plugins are notified when the monitored command finishes:
- **Start**: Plugin receives environment variables and starts
- **Finish**: Core Service closes the plugin's stdin stream
- **Cleanup**: Plugin should exit gracefully

## Roadmap

✅ **Milestone 1: Windows MVP** (Current)
- Core Service in Rust
- Windows process monitoring with .NET/WMI
- WPF spinner plugin
- Windows service installation

🔄 **Milestone 2: Linux Support**
- eBPF process monitoring
- Unix socket IPC
- Lua scripting support

🔄 **Milestone 3: macOS Support**
- DTrace process monitoring
- Complete cross-platform support

## Troubleshooting

### Services not starting
- Ensure you ran `install.bat` as Administrator
- Check Windows Event Viewer for service errors

### Spinner not appearing
- Verify configuration file exists and is valid
- Check that services are running: `sc query CommandSidekickCore`
- Look for logs in `%TEMP%\command-sidekick-spinner.log`

### Commands not detected
- Ensure Windows Probe service is running
- Check that your command pattern matches (use `*` for wildcards)

## Contributing

See [design/](design/) for architecture documentation and development roadmap.

## License

MIT License - see LICENSE file for details.