# Action Plugins & Customization

The Action Plugin system is designed for maximum user flexibility, allowing for both simple scripts and complex applications to be triggered.

## Plugin Execution Model

- When a monitored command is detected, the Core Service executes the associated plugin as a new child process.
- The Core Service passes context to the plugin via **environment variables**:
    - `VIBEROT_PID`: The Process ID of the monitored command.
    - `VIBEROT_COMMAND`: The full command line of the monitored command.
- The plugin runs concurrently with the monitored command.

## Finish Notification

- When the monitored command finishes, the Core Service **closes the `stdin` stream** of the plugin process.
- This is a simple, robust, and language-agnostic IPC mechanism. The plugin can detect this event by reading from its standard input and noticing the end-of-file (EOF).

---

## 1. Embedded Lua Scripts

For simple, lightweight actions, users can write Lua scripts directly in the `config.toml` file or link to a `.lua` file.

- **Why Lua?**: It's extremely fast, has a tiny memory footprint, and can be securely sandboxed and embedded directly into the Rust Core Service. This avoids the need for the user to have any other language or runtime installed.

- **Example `config.toml`:**
  ```toml
  [[rule]]
  command = "git push"
  action.lua = """
    print("Starting git push...")

    -- Register a function to run when the push is done
    viberot.on_finish(function()
      print("Git push complete!")
    end)
  """
  ```

- **Lua API (`viberot` global table)**:
  - `viberot.on_finish(callback)`: The primary function. Registers a callback that will be invoked when the monitored command exits.
  - `viberot.log(message)`: Writes a message to the main service log.
  - `viberot.pid()`: Returns the PID of the monitored process.
  - `viberot.command()`: Returns the command line of the monitored process.

---

## 2. External Executables

For more complex tasks, users can specify any executable file or script.

- **Example `config.toml`:**
  ```toml
  [[rule]]
  command = "npm install"
  # The current WPF spinner app could be used as an external plugin
  action.exec = "C:\\Users\\Me\\Projects\\NpmSpinner\\spinner.exe"

  [[rule]]
  command = "docker build *"
  action.exec = "/home/me/scripts/notify_build.py"
  ```

- **Example Python Plugin (`notify_build.py`)**:
  ```python
  import sys
  import os

  print(f"Docker build started with PID: {os.getenv('VIBEROT_PID')}")

  # This loop will run until stdin is closed by the Core Service
  for line in sys.stdin:
      # You could process messages from the core service here if needed
      pass

  # When the loop finishes (EOF is received), the command is done.
  print("Docker build finished. Sending notification...")
  # ... code to send a desktop notification or email ...
  ```
This dual approach provides a low-barrier entry for simple customizations (Lua) while imposing no limits on what advanced users can achieve (external executables).
