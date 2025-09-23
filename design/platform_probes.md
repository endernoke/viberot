# Platform Probes

Platform Probes are small, efficient, native components responsible for detecting process creation and forwarding the events to the Core Service. This design isolates the OS-specific code, keeping the Core Service clean and portable.

## Communication Protocol

All probes will send a JSON object over a local IPC channel (Unix socket or named pipe) to the Core Service.

**Event Format:**
```json
{
  "pid": 12345,
  "command": "/usr/bin/npm install",
  "timestamp": 1678886400
}
```

---

## 1. Linux Probe

- **Technology**: **eBPF (extended Berkeley Packet Filter)**
- **Implementation**:
    - An eBPF program will be written in C and attached to the `execve` syscall tracepoint. This is highly efficient and provides immediate access to new process information.
    - The eBPF program will push event data into a ring buffer.
    - A user-space component, integrated into the Rust Core Service using a crate like `libbpf-rs`, will read from this ring buffer. This avoids the need for a separate probe process on Linux.

---

## 2. Windows Probe

- **Technology**: **ETW (Event Tracing for Windows)**
- **Implementation**:
    - A small Rust module within the Core Service will act as an ETW consumer.
    - It will subscribe to the `Microsoft-Windows-Kernel-Process` provider, which emits an event for process creation (`EventID=1`).
    - This approach is significantly more performant than the WMI-based method used in the initial proof-of-concept. It is real-time and does not involve polling.
    - The `windows-sys` or a similar crate can be used to implement this.

---

## 3. macOS Probe

- **Technology**: **DTrace**
- **Implementation**:
    - DTrace is a powerful tracing framework available on macOS. It can be used to monitor system calls like `execve`.
    - A DTrace script (`.d` file) will be created to capture process execution events.
    - The Core Service will launch the `dtrace` command-line tool with the script as an argument.
    - The DTrace script will be configured to print event data to its standard output in a structured format (e.g., JSON).
    - The Core Service will capture the `stdout` of the `dtrace` process to receive the events.
    - This method requires the user to grant terminal access for the `dtrace` process, but it avoids the need for special entitlements for distribution.
