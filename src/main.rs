// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use config_watcher::ConfigWatcher;
use rule_engine::RuleEngine;
use action_orchestrator::ActionOrchestrator;
use platform::{PlatformProbe, PlatformProbeTrait, ProcessLifecycleEvent};

fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get log directory
    let log_dir = get_log_dir()?;
    
    // Create a file appender that rotates daily
    let file_appender = tracing_appender::rolling::daily(&log_dir, "viberot-service.log");
    
    // Create layers for logging
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // Disable ANSI colors for file output
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true);
    
    // For Windows subsystem apps, we still want to try console output in case it's redirected
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(false);
    
    // Initialize subscriber with both layers
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(file_layer)
        .with(console_layer)
        .init();
        
    // Log the log file location
    eprintln!("VibeRot logs will be written to: {}", log_dir.join("viberot-service.log").display());
    
    Ok(())
}

fn get_log_dir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let log_dir = dirs::data_local_dir()
        .ok_or("Could not find local data directory")?;
    
    let mut path = log_dir;
    path.push("viberot-service");
    path.push("logs");
    
    if let Err(e) = std::fs::create_dir_all(&path) {
        return Err(format!("Failed to create log directory at {:?}: {}", path, e).into());
    }
    
    Ok(path)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_logging()?;

    info!("Starting VibeRot Core Service");

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

    // Create action orchestrator with config
    let action_orchestrator = {
        let config_guard = config.read().await;
        ActionOrchestrator::with_config(config_guard.clone())
    };

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
                        let actions = rule_engine.match_command(&event.command, &config_guard).await;
                        if !actions.is_empty() {
                            info!("Rule matched, starting {} action(s): {:?}", actions.len(), actions);
                            
                            // Start all matching actions
                            if let Err(e) = action_orchestrator.start_actions(actions, &event).await {
                                error!("Failed to start actions: {}", e);
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
    path.push("viberot-service");

    if let Err(e) = std::fs::create_dir_all(&path) {
        return Err(format!("Failed to create config directory at {:?}: {}", path, e).into());
    }
    
    path.push("config.toml");
    info!("Using config file: {:?}", path);
    Ok(path)
}