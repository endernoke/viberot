use crate::config::Action;
use crate::platform::ProcessEvent;
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::path::PathBuf;
use std::env;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, debug};

pub struct ActionOrchestrator {
    active_actions: Arc<RwLock<HashMap<u32, Vec<ActiveAction>>>>,
    running_single_instance_actions: Arc<RwLock<HashSet<String>>>,
}

pub struct ActiveAction {
    pub child: tokio::process::Child,
    pub action: Action,
}

impl ActionOrchestrator {
    pub fn new() -> Self {
        Self {
            active_actions: Arc::new(RwLock::new(HashMap::new())),
            running_single_instance_actions: Arc::new(RwLock::new(HashSet::new())),
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
    
    /// Checks if the action is configured to run as a single instance
    fn is_single_instance(&self, action: &Action) -> bool {
        match action {
            Action::Executable { single_instance, .. } => *single_instance,
            Action::Lua { single_instance, .. } => *single_instance,
        }
    }
    
    /// Gets a unique key for the action to track single instances
    fn get_action_key(&self, action: &Action) -> String {
        match action {
            Action::Executable { path, args, .. } => {
                let args_str = args.as_ref()
                    .map(|a| a.join(" "))
                    .unwrap_or_default();
                format!("exec:{}:{}", path, args_str)
            }
            Action::Lua { script, .. } => {
                format!("lua:{}", script)
            }
        }
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
        // Check if this is a single-instance action and if it's already running
        if self.is_single_instance(&action) {
            let action_key = self.get_action_key(&action);
            let mut running_actions = self.running_single_instance_actions.write().await;
            
            if running_actions.contains(&action_key) {
                info!("Single-instance action '{}' is already running, skipping", action_key);
                return Ok(());
            }
            
            // Mark this action as running
            running_actions.insert(action_key.clone());
        }
        
        match action.clone() {
            Action::Executable { path, args, single_instance: _ } => {
                self.start_executable_action(path, args, action, event).await
            }
            Action::Lua { script: _, single_instance: _ } => {
                // TODO: Implement Lua execution in future milestones
                warn!("Lua actions not yet implemented");
                Ok(())
            }
        }
    }

    pub async fn start_actions(&self, actions: Vec<Action>, event: &ProcessEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut errors = Vec::new();
        
        for action in actions {
            if let Err(e) = self.start_action(action, event).await {
                errors.push(e);
            }
        }
        
        if !errors.is_empty() {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.to_string()).collect();
            return Err(format!("Failed to start {} action(s): {}", error_messages.len(), error_messages.join("; ")).into());
        }
        
        Ok(())
    }

    async fn start_executable_action(
        &self,
        path: String,
        args: Option<Vec<String>>,
        action: Action,
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
        
        if let Some(ref wd) = event.working_directory {
            cmd.env("VIBEROT_WORKING_DIRECTORY", wd);
        }
        
        if let Some(ref session_id) = event.shell_session_id {
            cmd.env("VIBEROT_SHELL_SESSION_ID", session_id);
        }
        
        // Add note about synthetic PIDs for shell probe
        match event.probe_source {
            crate::platform::ProbeSource::PosixShell => {
                cmd.env("VIBEROT_PID_TYPE", "synthetic");
            }
            _ => {
                cmd.env("VIBEROT_PID_TYPE", "system");
            }
        }
        
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

        let pid_type = match event.probe_source {
            crate::platform::ProbeSource::PosixShell => "synthetic",
            _ => "system",
        };

        info!("Started action plugin '{}' with PID {} for monitored {} PID {}", 
              resolved_path.display(), child_pid, pid_type, event.pid);

        // Store the active action
        let active_action = ActiveAction {
            child,
            action,
        };

        // Store by PID (synthetic or real)
        {
            let mut active_actions = self.active_actions.write().await;
            active_actions.entry(event.pid).or_insert_with(Vec::new).push(active_action);
        }

        Ok(())
    }

    /// Gracefully shutdown all active actions
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Shutting down action orchestrator...");
        
        let mut active_actions = self.active_actions.write().await;
        for (pid, action_list) in active_actions.drain() {
            info!("Terminating {} action(s) for PID {}", action_list.len(), pid);
            for active_action in action_list {
                self.terminate_action(active_action, &format!("PID {}", pid), true).await;
            }
        }
        
        // Clear all single instance tracking
        {
            let mut running_actions = self.running_single_instance_actions.write().await;
            running_actions.clear();
        }
        
        info!("Action orchestrator shutdown complete");
        Ok(())
    }

    /// Terminates a child action process gracefully with fallback to force kill
    /// 
    /// # Arguments
    /// * `active_action` - The action to terminate
    /// * `target_name` - Human-readable identifier for logging
    /// * `wait_for_completion` - If true, waits for termination; if false, spawns async task
    async fn terminate_action(&self, mut active_action: ActiveAction, target_name: &str, wait_for_completion: bool) {
        // Close stdin to signal the action plugin
        if let Some(stdin) = active_action.child.stdin.take() {
            drop(stdin);
        }
        
        if wait_for_completion {
            // Synchronous termination for shutdown scenarios
            self.terminate_action_sync(&mut active_action, target_name).await;
        } else {
            // Asynchronous termination for runtime scenarios
            let target_name = target_name.to_string();
            tokio::spawn(async move {
                Self::terminate_action_async(active_action, &target_name).await;
            });
        }
    }
    
    /// Synchronous termination with timeout and force kill
    async fn terminate_action_sync(&self, active_action: &mut ActiveAction, target_name: &str) {
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            active_action.child.wait()
        ).await {
            Ok(_) => {
                debug!("Action for {} exited gracefully", target_name);
            }
            Err(_) => {
                info!("Action for {} did not exit gracefully, force killing", target_name);
                let _ = active_action.child.kill().await;
            }
        }
    }
    
    /// Asynchronous termination with delayed force kill
    async fn terminate_action_async(mut active_action: ActiveAction, target_name: &str) {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        if let Err(_e) = active_action.child.kill().await {
            // Process already exited, which is fine
            debug!("Action for {} already exited", target_name);
        }
    }

    /// Called when a probe detects that a monitored process has ended
    pub async fn finish_action(&self, target_pid: u32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut active_actions = self.active_actions.write().await;
        
        if let Some(action_list) = active_actions.remove(&target_pid) {
            info!("Finishing {} action(s) for PID {}", action_list.len(), target_pid);
            
            // Clean up single instance tracking for all actions
            {
                let mut running_actions = self.running_single_instance_actions.write().await;
                for active_action in &action_list {
                    if self.is_single_instance(&active_action.action) {
                        let action_key = self.get_action_key(&active_action.action);
                        running_actions.remove(&action_key);
                        debug!("Removed single-instance action '{}' from tracking", action_key);
                    }
                }
            }
            
            // Terminate all actions asynchronously to avoid blocking the event loop
            for active_action in action_list {
                self.terminate_action(active_action, &format!("PID {}", target_pid), false).await;
            }
        } else {
            debug!("No active actions found for PID {}", target_pid);
        }

        Ok(())
    }
}