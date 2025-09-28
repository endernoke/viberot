use crate::config::Action;
use crate::platform::ProcessEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::path::PathBuf;
use std::env;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, debug};

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

    /// Resolves a path string with environment variable expansion and predictable relative path handling
    fn resolve_action_path(&self, path: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let path_str = path.trim();
        
        // If it's just an executable name (no path separators), preserve PATH lookup behavior
        if !path_str.contains('/') && !path_str.contains('\\') && !path_str.contains('.') {
            debug!("Preserving PATH lookup for executable name: {}", path_str);
            return Ok(PathBuf::from(path_str));
        }
        
        // Expand environment variables
        let expanded_path = self.expand_environment_variables(path_str)?;
        debug!("After environment expansion: {} -> {}", path_str, expanded_path);
        
        let path_buf = PathBuf::from(&expanded_path);
        
        // If already absolute, return as-is
        if path_buf.is_absolute() {
            debug!("Using absolute path: {}", expanded_path);
            return Ok(path_buf);
        }
        
        // For relative paths, resolve against viberot project root
        let viberot_root = self.get_viberot_root()?;
        let resolved_path = viberot_root.join(&path_buf);
        
        debug!("Resolved relative path: {} -> {}", expanded_path, resolved_path.display());
        Ok(resolved_path)
    }
    
    /// Expands environment variables in the format ${VAR_NAME}
    fn expand_environment_variables(&self, path: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = path.to_string();
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = match var_name {
                    "VIBEROT_HOME" => self.get_viberot_root()?.to_string_lossy().to_string(),
                    "VIBEROT_ACTIONS" => self.get_viberot_root()?.join("actions").to_string_lossy().to_string(),
                    _ => env::var(var_name).unwrap_or_else(|_| {
                        warn!("Environment variable {} not found, leaving unexpanded", var_name);
                        format!("${{{}}}", var_name)
                    })
                };
                result.replace_range(start..start + end + 1, &var_value);
            } else {
                break; // Malformed ${, stop processing
            }
        }
        Ok(result)
    }
    
    /// Gets the viberot project root directory
    fn get_viberot_root(&self) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Try to find the project root by looking for Cargo.toml starting from current exe
        if let Ok(exe_path) = env::current_exe() {
            let mut current = exe_path.parent();
            while let Some(dir) = current {
                if dir.join("Cargo.toml").exists() {
                    // Check if this looks like the viberot root by looking for expected structure
                    if dir.join("src").exists() && dir.join("actions").exists() {
                        return Ok(dir.to_path_buf());
                    }
                }
                current = dir.parent();
            }
        }
        
        // Fallback: try current working directory and walk up
        let mut current = env::current_dir()?;
        for _ in 0..5 { // Limit search depth
            if current.join("Cargo.toml").exists() && 
               current.join("src").exists() && 
               current.join("actions").exists() {
                return Ok(current);
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        Err("Could not find viberot project root directory. Expected to find Cargo.toml with src/ and actions/ directories.".into())
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
        // Resolve the path with environment variable expansion and predictable relative path handling
        let resolved_path = self.resolve_action_path(&path)?;
        
        info!("Starting action: '{}' -> '{}'", path, resolved_path.display());
        
        let mut cmd = Command::new(&resolved_path);
        
        if let Some(args) = args {
            cmd.args(args);
        }

        // Set a predictable working directory (viberot project root)
        if let Ok(viberot_root) = self.get_viberot_root() {
            cmd.current_dir(&viberot_root);
            debug!("Set working directory to: {}", viberot_root.display());
        }

        // Set environment variables
        cmd.env("VIBEROT_PID", event.pid.to_string());
        cmd.env("VIBEROT_COMMAND", &event.command);
        cmd.env("VIBEROT_TIMESTAMP", event.timestamp.to_string());
        
        // Also provide the viberot root as an environment variable for the action
        if let Ok(viberot_root) = self.get_viberot_root() {
            cmd.env("VIBEROT_HOME", viberot_root.to_string_lossy().as_ref());
        }

        // Configure stdio
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let child = cmd.spawn().map_err(|e| {
            format!("Failed to spawn action '{}' (resolved to '{}'): {}", 
                   path, resolved_path.display(), e)
        })?;
        let child_pid = child.id().unwrap_or(0);

        info!("Started action plugin '{}' with PID {} for monitored process {}", 
              resolved_path.display(), child_pid, event.pid);

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