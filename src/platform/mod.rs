// Platform-specific probes for process monitoring
// Each platform uses native, high-performance APIs as specified in the design

#[cfg(windows)]
pub mod windows_etw;

#[cfg(windows)]
pub use windows_etw::WindowsEtwProbe as PlatformProbe;

// Linux shell probe
#[cfg(target_os = "linux")]
pub mod linux_shell;

#[cfg(target_os = "linux")]
pub use linux_shell::LinuxShellProbe as PlatformProbe;

// Stub implementation for other platforms
#[cfg(not(any(windows, target_os = "linux")))]
pub mod stub;

#[cfg(not(any(windows, target_os = "linux")))]
pub use stub::StubProbe as PlatformProbe;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

/// Identifies which probe detected the process event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbeSource {
    WindowsEtw,
    LinuxShell,
    MacOsDtrace,
    // Future: LinuxEbpf, etc.
}

/// Represents a process creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub command: String,
    pub timestamp: u64,
    pub working_directory: Option<String>,
    pub environment: Option<HashMap<String, String>>,
    pub shell_session_id: Option<String>, // Keep for context, but PID is primary identifier
    pub probe_source: ProbeSource,
}

impl ProcessEvent {
    pub fn new(pid: u32, command: String, probe_source: ProbeSource) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            pid,
            command,
            timestamp,
            working_directory: None,
            environment: None,
            shell_session_id: None,
            probe_source,
        }
    }

    pub fn with_working_directory(mut self, wd: String) -> Self {
        self.working_directory = Some(wd);
        self
    }

    pub fn with_environment(mut self, env: HashMap<String, String>) -> Self {
        self.environment = Some(env);
        self
    }

    pub fn with_shell_session_id(mut self, session_id: String) -> Self {
        self.shell_session_id = Some(session_id);
        self
    }
}

/// Trait that all platform probes must implement
pub trait PlatformProbeTrait {
    /// Start the probe and begin monitoring process events
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Stop the probe
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the capability level of this probe
    fn get_capability(&self) -> PlatformCapability;
}

/// Auto-detect and choose the best available probe method
pub fn detect_best_probe(
    lifecycle_sender: tokio::sync::broadcast::Sender<ProcessLifecycleEvent>
) -> (PlatformProbe, PlatformCapability) {
    #[cfg(windows)]
    {
        let probe = PlatformProbe::new(lifecycle_sender);
        (probe, PlatformCapability::SystemWide)
    }
    
    #[cfg(target_os = "linux")]
    {
        let probe = PlatformProbe::new(lifecycle_sender);
        (probe, PlatformCapability::ShellOnly)
    }
    
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let probe = PlatformProbe::new(lifecycle_sender);
        (probe, PlatformCapability::Polling) // Stub implementation
    }
}

/// Platform capability levels
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformCapability {
    SystemWide,    // eBPF, ETW, DTrace - monitors all processes
    ShellOnly,     // bash/zsh hooks - only monitors shell commands
    Polling,       // fallback approach - periodic polling (future)
}

/// Extended process event that includes lifecycle information
#[derive(Debug, Clone)]
pub enum ProcessLifecycleEvent {
    /// Process started
    Started(ProcessEvent),
    /// Process ended
    Ended { pid: u32 },
}