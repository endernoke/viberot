use tokio::sync::broadcast;
use tracing::{error, info};

use crate::platform::{PlatformProbeTrait, ProcessLifecycleEvent};

/// Stub implementation for unsupported platforms
/// This allows the code to compile and provides clear error messages
pub struct StubProbe {
    _lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>,
}

impl StubProbe {
    pub fn new(lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>) -> Self {
        Self {
            _lifecycle_sender: lifecycle_sender,
        }
    }
}

impl PlatformProbeTrait for StubProbe {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        error!("Process monitoring is not yet implemented for this platform");
        error!("VibeRot currently supports:");
        error!("  - Windows (using ETW - Event Tracing for Windows)");
        error!("Future platform support is planned for:");
        error!("  - Linux (using ePBF)");
        error!("  - macOS (technology to be decided)");
        error!("  - bash/zsh (using shell hooks)");

        #[cfg(target_os = "linux")]
        info!("Running on Linux - support coming soon!");

        #[cfg(target_os = "macos")]
        info!("Running on macOS - support coming soon!");

        info!("Learn more or contribute at: https://github.com/endernoke/viberot");
        
        Err("Platform not supported yet. Please use Windows or wait for cross-platform support.".into())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Nothing to stop for stub implementation
        Ok(())
    }
}