// Platform-specific probes for process monitoring
// Each platform uses native, high-performance APIs as specified in the design

#[cfg(windows)]
pub mod windows_etw;

#[cfg(windows)]
pub use windows_etw::WindowsEtwProbe as PlatformProbe;

// Stub implementation for non-Windows platforms
#[cfg(not(windows))]
pub mod stub;

#[cfg(not(windows))]
pub use stub::StubProbe as PlatformProbe;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a process creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub command: String,
    pub timestamp: u64,
}

impl ProcessEvent {
    pub fn new(pid: u32, command: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            pid,
            command,
            timestamp,
        }
    }
}

/// Trait that all platform probes must implement
pub trait PlatformProbeTrait {
    /// Start the probe and begin monitoring process events
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Stop the probe
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Extended process event that includes lifecycle information
#[derive(Debug, Clone)]
pub enum ProcessLifecycleEvent {
    /// Process started
    Started(ProcessEvent),
    /// Process ended
    Ended { pid: u32 },
}