# Path Resolution in VibeRot Actions

VibeRot provides flexible path resolution for action executables with predictable behavior and useful environment variable expansion.

## Path Resolution Rules

### 1. Executable Names (PATH Lookup)
For simple executable names without path separators, VibeRot preserves the standard PATH lookup behavior:

```toml
[rules.action]
type = "exec"
path = "python"      # Found via PATH
path = "notepad"     # Found via PATH  
path = "git"         # Found via PATH
```

**Behavior**: These are resolved using the system PATH environment variable, just like running the command in a terminal.

### 2. Absolute Paths
Absolute paths are used exactly as specified:

```toml
[rules.action]
type = "exec"
path = "C:\\Program Files\\MyApp\\action.exe"   # Windows
path = "/usr/local/bin/my-action"               # Linux/macOS
```

**Behavior**: No modification - used directly.

### 3. Relative Paths
Relative paths are resolved relative to the **viberot project root directory** (the directory containing the main `Cargo.toml`):

```toml
[rules.action]
type = "exec"
path = "test_action.py"                                    # -> {viberot_root}/test_action.py
path = "actions/overlay/target/release/overlay.exe"        # -> {viberot_root}/actions/overlay/target/release/overlay.exe
path = "../other-project/script.py"                       # -> {viberot_root}/../other-project/script.py
```

**Behavior**: Always predictable regardless of where the viberot service is started from.

## Environment Variable Expansion

VibeRot supports environment variable expansion in paths using `${VAR_NAME}` syntax.

### Built-in Variables

- **`${VIBEROT_HOME}`**: Path to the viberot project root directory
- **`${VIBEROT_ACTIONS}`**: Path to the `actions` subdirectory

### System Variables
Any system environment variable can be expanded:

```toml
[rules.action]
type = "exec"
path = "${USERPROFILE}/my-scripts/action.py"    # Windows user profile
path = "${HOME}/bin/my-action"                  # Unix home directory
path = "${PROGRAMFILES}/MyApp/action.exe"       # Windows Program Files
```

### Example Combinations

```toml
# Use built-in variables
path = "${VIBEROT_HOME}/test_action.py"
path = "${VIBEROT_ACTIONS}/overlay/target/release/overlay.exe"

# Mix variables and relative paths
path = "${VIBEROT_HOME}/../sibling-project/action.py"
path = "${VIBEROT_ACTIONS}/../custom-actions/notifier.exe"

# Use system variables
path = "${USERPROFILE}/Desktop/my-action.bat"
path = "${TEMP}/temp-action.py"
```

## Working Directory

Action processes are started with their working directory set to the **viberot project root**, ensuring predictable file access for your action scripts.

## Environment Variables Provided to Actions

When your action runs, it receives these environment variables:

- `VIBEROT_PID`: Process ID of the monitored command
- `VIBEROT_COMMAND`: Full command line that was intercepted
- `VIBEROT_TIMESTAMP`: Timestamp when the command was detected
- `VIBEROT_HOME`: Path to the viberot project root (same as `${VIBEROT_HOME}`)

## Error Handling

- If an environment variable is not found, a warning is logged and the variable is left unexpanded
- If the viberot project root cannot be determined, an error is returned
- If the resolved path cannot be executed, a detailed error message shows both original and resolved paths

## Migration from Simple Paths

If you have existing configurations with simple paths, they will continue to work:

- Executable names: No change needed
- Absolute paths: No change needed  
- Relative paths: Now resolved relative to project root instead of service working directory (more predictable)