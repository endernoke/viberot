# Development Milestones

This document outlines a proposed roadmap for developing Command-Sidekick.

## Milestone 1: Core Service and Windows MVP

The goal of this milestone is to have a working Minimum Viable Product (MVP) on Windows, as it's the platform for the original proof-of-concept.

- **[Core Service]**
    - [x] Set up Rust project structure.
    - [x] Implement TOML configuration loading and parsing.
    - [x] Implement the rule-matching logic for command strings.
    - [x] Implement process spawning for external Action Plugins.
    - [x] Implement process monitoring (wait for PID to exit) and `stdin` closing for finish notification.

- **[Windows Probe]**
    - [x] Implement the WMI listener for process creation events.
    - [x] Implement the named pipe for sending events to the Core Service.

- **[Action Plugin]**
    - [x] Adapt the existing `NpmInstallListener` C# application to act as a simple external plugin that closes when its `stdin` is closed.

- **[Installer/Distribution]**
    - [x] Create a simple installer script or package (e.g., MSIX) that installs the Core Service and registers it to run on startup.

**Outcome**: A user can install the service on Windows, configure it to watch for `npm install`, and see the spinner UI appear and disappear correctly.

---

## Milestone 2: Linux Support and Embedded Scripting

This milestone focuses on achieving cross-platform support and introducing the more flexible Lua scripting.

- **[Core Service]**
    - [ ] Refactor IPC to use Unix sockets on Linux.
    - [ ] Embed the Lua interpreter (`mlua`).
    - [ ] Expose the initial `sidekick` API to the Lua environment.
    - [ ] Implement the logic to run Lua actions from the config file.

- **[Linux Probe]**
    - [ ] Integrate an eBPF library (`libbpf-rs` or similar).
    - [ ] Write the eBPF program to trace `execve`.
    - [ ] Connect the eBPF event reader to the Core Service's event bus.

- **[Testing]**
    - [ ] Set up a CI/CD pipeline (e.g., GitHub Actions) to build and test on both Windows and Linux.

**Outcome**: The service now runs on Linux. Users can write simple Lua scripts to react to commands.

---

## Milestone 3: macOS Support and Feature Polish

This milestone completes the initial platform support and adds features to improve usability.

- **[Core Service]**
    - [ ] Add any necessary platform-specific logic for macOS.

- **[macOS Probe]**
    - [ ] Write the DTrace script (`.d` file) for tracing process execution.
    - [ ] Implement the logic in the Core Service to launch and manage the `dtrace` process.
    - [ ] Ensure the Core Service can correctly parse the `stdout` from the `dtrace` process.

- **[Features]**
    - [ ] **Configuration Reloading**: Implement hot-reloading of the config file.
    - [ ] **Improved Logging**: Add robust logging to a file for easier debugging.
    - [ ] **More Lua APIs**: Expand the Lua API with more capabilities (e.g., running shell commands, basic file I/O).

**Outcome**: The project is now fully cross-platform (Windows, Linux, macOS) and has a more mature feature set.

---

## Future Milestones (Post-v1.0)

- **Plugin Marketplace**: A way for users to share and discover Action Plugins.
- **More Trigger Types**: Trigger actions on file changes, network events, etc.
- **GUI Configurator**: A simple UI for editing the configuration file.
- **Advanced Rule Matching**: Support for regular expressions and more complex conditions in rules.
