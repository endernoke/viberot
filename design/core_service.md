# Core Service Architecture

The Core Service is the central component of VibeRot, written in Rust.

## Responsibilities

1.  **Configuration Management**:
    - On startup, load and parse a `config.toml` file from the user's home directory.
    - The configuration will define rules mapping command patterns (e.g., `npm install`, `docker build *`) to specific Action Plugins.
    - Watch for changes to the configuration file and reload it automatically.

2.  **Event Bus**:
    - Listen for process creation events from the Platform Probes via a standardized IPC mechanism (e.g., a Unix socket on Linux/macOS and a named pipe on Windows).
    - The event payload will be a simple, serialized format (like JSON or MessagePack) containing the process ID (PID), command line, and parent PID.

3.  **Rule Matching Engine**:
    - For each incoming process event, match the command line against the user-defined rules.
    - The matching should support simple wildcards (`*`) and potentially regular expressions in the future.

4.  **Action Orchestration**:
    - When a rule is matched, spawn the corresponding Action Plugin as a child process.
    - Pass relevant information to the plugin via environment variables or command-line arguments (e.g., `VIBEROT_PID`, `VIBEROT_COMMAND`).

5.  **Process Lifecycle Management**:
    - Keep track of the PID of the monitored command.
    - Continuously monitor the process. When the process exits, notify the Action Plugin.
    - The notification mechanism will be to close the `stdin` of the child plugin process. This is a simple and universally supported method.

6.  **Embedded Lua Environment (for scripting)**:
    - Embed a Lua interpreter (using a crate like `mlua`).
    - Expose a minimal, safe API to the Lua scripts. For example:
        - `viberot.log(message)`: Log a message to the Core Service's log file.
        - `viberot.on_finish(callback)`: Register a function to be called when the monitored command completes.

## Rust Crates to Consider

- `tokio`: For asynchronous I/O and process management.
- `serde`: For serialization/deserialization of configuration and event data.
- `toml`: For parsing the configuration file.
- `libbpf-rs` / `redbpf`: For interacting with eBPF on Linux.
- `windows-sys`: For interacting with ETW on Windows.
- `mlua`: For embedding the Lua interpreter.
