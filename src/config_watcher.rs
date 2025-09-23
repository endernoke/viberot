use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use tokio::sync::mpsc;
use std::path::Path;
use tracing::{info, error, warn};

use crate::config::Config;

/// Handles configuration file watching and hot-reloading
pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn new(
        config_path: impl AsRef<Path>
    ) -> Result<(Self, mpsc::Receiver<Config>), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel(10);
        let config_path = config_path.as_ref().to_path_buf();
        let watch_path = config_path.clone();

        let mut watcher = notify::recommended_watcher(move |res: NotifyResult<notify::Event>| {
            match res {
                Ok(event) => {
                    // Check if the config file was modified
                    if event.paths.iter().any(|p| p == &config_path) {
                        info!("Configuration file changed, reloading...");
                        
                        // Try to reload the configuration
                        match Config::load(&config_path) {
                            Ok(new_config) => {
                                // Send the new config through the channel
                                if let Err(e) = tx.blocking_send(new_config) {
                                    warn!("Failed to send config change notification: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to reload configuration: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("File watcher error: {:?}", e);
                }
            }
        })?;

        // Watch the parent directory of the config file
        if let Some(parent) = watch_path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
            info!("Started watching configuration directory: {:?}", parent);
        }

        Ok((Self {
            _watcher: watcher,
        }, rx))
    }
}