use crate::config::Action;
use crate::platform::ProcessEvent;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

pub struct ActionOrchestrator {
    active_actions: Arc<RwLock<HashMap<u32, ActiveAction>>>,
}

pub struct ActiveAction {
    pub child: tokio::process::Child,
}

impl ActionOrchestrator {
    pub fn new() -> Self {
        Self {
            active_actions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_action(&self, action: Action, event: &ProcessEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match action {
            Action::Executable { path, args } => {
                self.start_executable_action(path, args, event).await
            }
            Action::Lua { script: _ } => {
                // TODO: Implement Lua execution in future milestones
                warn!("Lua actions not yet implemented");
                Ok(())
            }
        }
    }

    async fn start_executable_action(
        &self,
        path: String,
        args: Option<Vec<String>>,
        event: &ProcessEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cmd = Command::new(&path);
        
        if let Some(args) = args {
            cmd.args(args);
        }

        // Set environment variables
        cmd.env("VIBEROT_PID", event.pid.to_string());
        cmd.env("VIBEROT_COMMAND", &event.command);
        cmd.env("VIBEROT_TIMESTAMP", event.timestamp.to_string());

        // Configure stdio
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let child = cmd.spawn()?;
        let child_pid = child.id().unwrap_or(0);

        info!("Started action plugin '{}' with PID {} for monitored process {}", 
              path, child_pid, event.pid);

        // Store the active action
        let active_action = ActiveAction {
            child,
        };

        {
            let mut active_actions = self.active_actions.write().await;
            active_actions.insert(event.pid, active_action);
        }

        Ok(())
    }

    /// Gracefully shutdown all active actions
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Shutting down action orchestrator...");
        let mut active_actions = self.active_actions.write().await;
        
        for (pid, mut active_action) in active_actions.drain() {
            info!("Terminating action for process {}", pid);
            
            // Close stdin to signal the action plugin
            if let Some(stdin) = active_action.child.stdin.take() {
                drop(stdin);
            }
            
            // Give the process time to exit gracefully, then force kill
            // Tbh we should let the child decide what to do instead of killing it
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(3),
                active_action.child.wait()
            ).await {
                Ok(_) => {
                    // Process exited gracefully
                }
                Err(_) => {
                    info!("Action for process {} did not exit gracefully, force killing", pid);
                    let _ = active_action.child.kill().await;
                }
            }
        }
        
        info!("Action orchestrator shutdown complete");
        Ok(())
    }

    /// Called when the ETW probe detects that a monitored process has ended
    /// This replaces the previous polling-based approach
    pub async fn finish_action(&self, target_pid: u32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut active_actions = self.active_actions.write().await;
        
        if let Some(mut active_action) = active_actions.remove(&target_pid) {
            info!("Finishing action for process {}", target_pid);
            
            // Close stdin to signal the action plugin that the command is finished
            if let Some(stdin) = active_action.child.stdin.take() {
                drop(stdin); // Dropping stdin closes it
            }

            // Give the process a moment to clean up, then force kill if necessary
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                if let Err(_e) = active_action.child.kill().await {
                    // Process already exited, which is fine
                }
            });
        }

        Ok(())
    }
}