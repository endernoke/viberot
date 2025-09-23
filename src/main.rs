mod config;
mod config_watcher;
mod rule_engine;
mod action_orchestrator;
mod platform;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::signal;
use tracing::{info, error};

use config::Config;
use config_watcher::ConfigWatcher;
use rule_engine::RuleEngine;
use action_orchestrator::ActionOrchestrator;
use platform::{PlatformProbe, PlatformProbeTrait, ProcessLifecycleEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Command-Sidekick Core Service");

    // Load configuration
    let config_path = get_config_path()?;
    let config = Arc::new(RwLock::new(Config::load(&config_path)?));

    // Set up configuration file watching for hot-reload
    let (_config_watcher, mut config_change_rx) = ConfigWatcher::new(&config_path)?;
    
    // Spawn task to handle config changes
    let config_for_watcher = Arc::clone(&config);
    tokio::spawn(async move {
        while let Some(new_config) = config_change_rx.recv().await {
            info!("Configuration changed, updating...");
            {
                let mut config_guard = config_for_watcher.write().await;
                *config_guard = new_config;
            }
            info!("Configuration reloaded successfully - rules will be updated for new processes");
        }
    });

    info!("Configuration loaded with hot-reload enabled");

    // Create lifecycle event channel (process start/stop events)
    let (lifecycle_tx, mut lifecycle_rx) = broadcast::channel(1024);

    // Create rule engine
    let rule_engine = RuleEngine::new();

    // Create action orchestrator  
    let action_orchestrator = ActionOrchestrator::new();

    // Start platform-specific probe
    let probe = PlatformProbe::new(lifecycle_tx);
    if let Err(e) = probe.start().await {
        error!("Failed to start platform probe: {}", e);
        return Err(e);
    }

    info!("Platform probe started successfully");

    // Main event loop - process lifecycle events
    loop {
        tokio::select! {
            // Handle shutdown signal (Ctrl+C)
            _ = signal::ctrl_c() => {
                info!("Received shutdown signal, cleaning up...");
                break;
            }
            // Handle process lifecycle events
            event_result = lifecycle_rx.recv() => {
                match event_result {
                    Ok(ProcessLifecycleEvent::Started(event)) => {
                        // debug!("Process started: {} (PID: {})", event.command, event.pid);
                        
                        // Match against rules
                        let config_guard = config.read().await;
                        if let Some(action) = rule_engine.match_command(&event.command, &config_guard).await {
                            info!("Rule matched, starting action: {:?}", action);
                            
                            // Start the action
                            if let Err(e) = action_orchestrator.start_action(action, &event).await {
                                error!("Failed to start action: {}", e);
                            }
                        }
                    }
                    Ok(ProcessLifecycleEvent::Ended { pid }) => {
                        // debug!("Process ended: PID {}", pid);

                        // Notify action orchestrator that the process ended
                        if let Err(e) = action_orchestrator.finish_action(pid).await {
                            error!("Failed to finish action for PID {}: {}", pid, e);
                        }
                    }
                    Err(e) => {
                        error!("Lifecycle event channel error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    // Cleanup
    info!("Shutting down gracefully...");
    if let Err(e) = action_orchestrator.shutdown().await {
        error!("Error shutting down action orchestrator: {}", e);
    }
    
    if let Err(e) = probe.stop().await {
        error!("Error stopping probe: {}", e);
    }

    info!("Shutdown complete");
    Ok(())
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory - this usually means the HOME environment variable is not set")?;
    
    let mut path = config_dir;
    path.push("command-sidekick");
    
    if let Err(e) = std::fs::create_dir_all(&path) {
        return Err(format!("Failed to create config directory at {:?}: {}", path, e).into());
    }
    
    path.push("config.toml");
    info!("Using config file: {:?}", path);
    Ok(path)
}