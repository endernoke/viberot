use crate::config::Action;
use crate::event_bus::ProcessEvent;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use sysinfo::{System, Pid};
use tracing::{info, warn, debug};

pub struct ActionOrchestrator {
    active_actions: Arc<RwLock<HashMap<u32, ActiveAction>>>,
    system: Arc<RwLock<System>>,
}

pub struct ActiveAction {
    pub child: tokio::process::Child,
    pub action: Action,
}

impl ActionOrchestrator {
    pub fn new() -> Self {
        let active_actions = Arc::new(RwLock::new(HashMap::new()));
        let system = Arc::new(RwLock::new(System::new()));
        
        Self {
            active_actions,
            system,
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
        cmd.env("SIDEKICK_PID", event.pid.to_string());
        cmd.env("SIDEKICK_COMMAND", &event.command);
        cmd.env("SIDEKICK_TIMESTAMP", event.timestamp.to_string());

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
            action: Action::Executable { path: path.clone(), args: None },
        };

        {
            let mut active_actions = self.active_actions.write().await;
            active_actions.insert(event.pid, active_action);
        }

        // Start monitoring the target process
        self.start_monitoring(event.pid).await;

        Ok(())
    }

    async fn start_monitoring(&self, pid: u32) {
        let active_actions = Arc::clone(&self.active_actions);
        let system = Arc::clone(&self.system);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                // Check if the process is still alive
                let process_exists = {
                    let mut sys = system.write().await;
                    sys.refresh_processes();
                    sys.process(Pid::from_u32(pid)).is_some()
                };

                if !process_exists {
                    info!("Process {} has exited, cleaning up action", pid);
                    
                    // Remove from active actions and signal completion
                    let mut actions = active_actions.write().await;
                    if let Some(mut active_action) = actions.remove(&pid) {
                        // Close stdin to signal the action plugin that the command is finished
                        if let Some(stdin) = active_action.child.stdin.take() {
                            drop(stdin); // Dropping stdin closes it
                        }

                        // Give the process a moment to clean up
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            if let Err(e) = active_action.child.kill().await {
                                debug!("Process already exited: {}", e);
                            }
                        });
                    }
                    
                    break;
                }
            }
        });
    }

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
                if let Err(e) = active_action.child.kill().await {
                    warn!("Failed to kill action process: {}", e);
                }
            });
        }

        Ok(())
    }
}