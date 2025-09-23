mod config;
mod event_bus;
mod rule_engine;
mod action_orchestrator;

#[cfg(windows)]
mod windows;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, error};

use config::Config;
use event_bus::EventBus;
use rule_engine::RuleEngine;
use action_orchestrator::ActionOrchestrator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Command-Sidekick Core Service");

    // Load configuration
    let config_path = get_config_path()?;
    let config = Arc::new(RwLock::new(Config::load(&config_path)?));

    // Create event bus
    let (event_tx, _) = broadcast::channel(1024);

    // Create rule engine
    let rule_engine = RuleEngine::new(config.clone());

    // Create action orchestrator
    let action_orchestrator = ActionOrchestrator::new();

    // Start platform-specific probe
    #[cfg(windows)]
    {
        let windows_probe = windows::WindowsProbe::new(event_tx.clone());
        tokio::spawn(async move {
            if let Err(e) = windows_probe.start().await {
                error!("Windows probe failed: {}", e);
            }
        });
    }

    // Main event loop
    let mut event_rx = event_tx.subscribe();
    loop {
        match event_rx.recv().await {
            Ok(event) => {
                info!("Received process event: {} (PID: {})", event.command, event.pid);
                
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
            Err(e) => {
                error!("Event bus error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir()
        .ok_or("Could not find config directory")?;
    path.push("command-sidekick");
    std::fs::create_dir_all(&path)?;
    path.push("config.toml");
    Ok(path)
}