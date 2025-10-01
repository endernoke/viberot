use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

use crate::platform::{PlatformProbeTrait, ProcessLifecycleEvent, ProcessEvent, ProbeSource, PlatformCapability};

/// Atomic counter for generating synthetic PIDs starting from 1,000,000
/// to avoid collision with real system PIDs
static SYNTHETIC_PID_COUNTER: AtomicU32 = AtomicU32::new(1_000_000);

/// Linux shell-based process probe
/// Uses shell hooks to monitor command execution in bash/zsh
pub struct LinuxShellProbe {
    lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>,
    socket_path: PathBuf,
    listener: Arc<Mutex<Option<UnixListener>>>,
    /// Track active shell sessions mapping to their synthetic PIDs
    active_sessions: Arc<Mutex<HashMap<String, u32>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShellMessage {
    pub session_id: String,
    pub event_type: ShellEventType,
    pub command: Option<String>,
    pub command_b64: Option<String>,
    pub exit_code: Option<i32>,
    pub working_directory: Option<String>,
    pub working_directory_b64: Option<String>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ShellEventType {
    CommandStart,
    CommandEnd,
}

impl LinuxShellProbe {
    pub fn new(lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>) -> Self {
        let socket_path = Self::get_socket_path();
        
        Self {
            lifecycle_sender,
            socket_path,
            listener: Arc::new(Mutex::new(None)),
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a unique synthetic PID for shell commands
    /// Uses range starting from 1,000,000 to avoid real system PID collisions
    fn generate_synthetic_pid() -> u32 {
        SYNTHETIC_PID_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Extract complete JSON messages from a buffer, handling newline-separated single-line JSON
    fn extract_json_messages(buffer: &mut String) -> Vec<String> {
        let mut messages = Vec::new();
        let mut remaining_buffer = String::new();
        
        for line in buffer.lines() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            
            // Check if line looks like a complete JSON message
            if line.starts_with('{') && line.ends_with('}') {
                messages.push(line.to_string());
            } else if !line.is_empty() {
                // Keep incomplete lines for next iteration
                if !remaining_buffer.is_empty() {
                    remaining_buffer.push('\n');
                }
                remaining_buffer.push_str(line);
            }
        }
        
        *buffer = remaining_buffer;
        messages
    }

    fn get_socket_path() -> PathBuf {
        if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime_dir).join("viberot-shell.sock")
        } else {
            dirs::runtime_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("viberot-shell.sock")
        }
    }

    fn get_shell_script_content() -> String {
        format!(r#"
# VibeRot Shell Hook Integration
# This enables VibeRot to monitor commands executed in your shell

# Function to base64 encode strings safely
_viberot_base64_encode() {{
    local input="$1"
    if command -v base64 >/dev/null 2>&1; then
        printf '%s' "$input" | base64 -w 0 2>/dev/null || printf '%s' "$input" | base64
    else
        # Fallback: use openssl if base64 is not available
        printf '%s' "$input" | openssl base64 -A 2>/dev/null || echo "$input"
    fi
}}

# Function to communicate with VibeRot service
_viberot_send_message() {{
    local message="$1"
    # Send single-line JSON message terminated with newline
    if command -v nc >/dev/null 2>&1; then
        (timeout 1 printf '%s\n' "$message" | nc -U "{socket_path}" 2>/dev/null || true &)
    elif command -v socat >/dev/null 2>&1; then
        (timeout 1 printf '%s\n' "$message" | socat - UNIX-CONNECT:"{socket_path}" 2>/dev/null || true &)
    fi
}}

# Pre-command hook
_viberot_pre_command_hook() {{
    if [[ "$BASH_COMMAND" != _viberot_* ]] && [ -n "$BASH_COMMAND" ]; then
        # Base64 encode values that may contain special characters
        local encoded_command="$(_viberot_base64_encode "$BASH_COMMAND")"
        local encoded_pwd="$(_viberot_base64_encode "$PWD")"
        
        # Create single-line JSON message with base64-encoded fields
        local json_msg="{{\"session_id\":\"$$\",\"event_type\":\"CommandStart\",\"command_b64\":\"$encoded_command\",\"working_directory_b64\":\"$encoded_pwd\",\"environment\":{{}}}}"
        _viberot_send_message "$json_msg"
    fi
}}

# Post-command hook
_viberot_post_command_hook() {{
    local exit_code=$?
    # Create single-line JSON message
    local json_msg="{{\"session_id\":\"$$\",\"event_type\":\"CommandEnd\",\"exit_code\":$exit_code}}"
    _viberot_send_message "$json_msg"
}}

# Set up the hooks
if [ -n "$BASH_VERSION" ]; then
    # Bash setup
    trap '_viberot_pre_command_hook' DEBUG
    PROMPT_COMMAND="_viberot_post_command_hook"
elif [ -n "$ZSH_VERSION" ]; then
    # Zsh setup
    autoload -Uz add-zsh-hook
    _viberot_zsh_pre_hook() {{
        # Base64 encode values that may contain special characters
        local encoded_command="$(_viberot_base64_encode "$1")"
        local encoded_pwd="$(_viberot_base64_encode "$PWD")"
        
        # Create single-line JSON message with base64-encoded fields
        local json_msg="{{\"session_id\":\"$$\",\"event_type\":\"CommandStart\",\"command_b64\":\"$encoded_command\",\"working_directory_b64\":\"$encoded_pwd\",\"environment\":{{}}}}"
        _viberot_send_message "$json_msg"
    }}
    
    _viberot_zsh_post_hook() {{
        local exit_code=$?
        # Create single-line JSON message
        local json_msg="{{\"session_id\":\"$$\",\"event_type\":\"CommandEnd\",\"exit_code\":$exit_code}}"
        _viberot_send_message "$json_msg"
    }}
    
    add-zsh-hook preexec _viberot_zsh_pre_hook
    add-zsh-hook precmd _viberot_zsh_post_hook
fi
"#, socket_path = Self::get_socket_path().display())
    }

    async fn setup_shell_hooks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("\n🎉 Welcome to VibeRot Shell Integration Setup!");
        println!("═══════════════════════════════════════════════");
        println!();
        println!("To monitor shell commands, VibeRot needs to install hooks in your shell configuration.");
        println!("This will add a few functions to your .bashrc or .zshrc file.");
        println!();
        println!("Would you like to proceed with automatic installation? [y/N]: ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !input.trim().to_lowercase().starts_with('y') {
            println!();
            println!("📋 Manual Setup Instructions:");
            println!("════════════════════════════");
            println!();
            println!("Add the following to your shell configuration file:");
            
            let shell = env::var("SHELL").unwrap_or_else(|_| String::from("bash"));
            if shell.contains("zsh") {
                println!("File: ~/.zshrc");
            } else {
                println!("File: ~/.bashrc");
            }
            
            println!();
            println!("{}", Self::get_shell_script_content());
            println!();
            println!("After adding the code, restart your shell or run:");
            if shell.contains("zsh") {
                println!("  source ~/.zshrc");
            } else {
                println!("  source ~/.bashrc");
            }
            println!();
            
            return Err("Manual setup required - please add shell hooks manually".into());
        }

        // Automatic installation
        let shell = env::var("SHELL").unwrap_or_else(|_| String::from("bash"));
        let config_file = if shell.contains("zsh") {
            dirs::home_dir().ok_or("Could not find home directory")?.join(".zshrc")
        } else {
            dirs::home_dir().ok_or("Could not find home directory")?.join(".bashrc")
        };

        let script_content = Self::get_shell_script_content();
        let marker_start = "# === VibeRot Shell Integration START ===";
        let marker_end = "# === VibeRot Shell Integration END ===";
        
        // Check if already installed
        if config_file.exists() {
            let existing_content = fs::read_to_string(&config_file)?;
            if existing_content.contains(marker_start) {
                println!("✅ VibeRot shell integration is already installed!");
                return Ok(());
            }
        }

        // Append to shell config
        let integration_block = format!("\n{}\n{}\n{}\n", marker_start, script_content, marker_end);
        
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config_file)?
            .write_all(integration_block.as_bytes())?;

        println!("✅ Shell integration installed successfully!");
        println!();
        println!("🔄 Please restart your shell or run:");
        if shell.contains("zsh") {
            println!("  source ~/.zshrc");
        } else {
            println!("  source ~/.bashrc"); 
        }
        println!();

        Ok(())
    }

    async fn start_socket_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Remove existing socket file if it exists
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Shell probe listening on: {}", self.socket_path.display());

        // Accept connections in a loop
        let lifecycle_sender = self.lifecycle_sender.clone();
        let active_sessions = Arc::clone(&self.active_sessions);
        let listener_handle = Arc::new(Mutex::new(listener));
        
        // Store the listener for cleanup
        {
            let mut listener_guard = self.listener.lock().await;
            *listener_guard = None; // We don't need to store it since we can't really stop individual listeners cleanly
        }

        let listener_for_task = Arc::clone(&listener_handle);
        tokio::spawn(async move {
            loop {
                let listener = listener_for_task.lock().await;
                match listener.accept().await {
                    Ok((stream, _)) => {
                        drop(listener); // Release the lock
                        let sender = lifecycle_sender.clone();
                        let sessions = Arc::clone(&active_sessions);
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_connection(stream, sender, sessions).await {
                                debug!("Connection handling error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_connection(
        stream: UnixStream,
        lifecycle_sender: broadcast::Sender<ProcessLifecycleEvent>,
        active_sessions: Arc<Mutex<HashMap<String, u32>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut reader = AsyncBufReader::new(stream);
        let mut buffer = String::new();

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await?;
            
            if bytes_read == 0 {
                break; // EOF
            }
            
            buffer.push_str(&line);
            
            // Try to parse complete JSON messages from buffer
            let messages = Self::extract_json_messages(&mut buffer);
            
            for json_str in messages {
                match serde_json::from_str::<ShellMessage>(&json_str) {
                    Ok(msg) => {
                        match msg.event_type {
                            ShellEventType::CommandStart => {
                                // Decode command from base64 or use plain text
                                let command = if let Some(cmd_b64) = msg.command_b64 {
                                    match general_purpose::STANDARD.decode(&cmd_b64) {
                                        Ok(decoded_bytes) => String::from_utf8_lossy(&decoded_bytes).to_string(),
                                        Err(_) => msg.command.unwrap_or_else(|| "<decode error>".to_string()),
                                    }
                                } else {
                                    msg.command.unwrap_or_else(|| "<unknown command>".to_string())
                                };
                            
                                // Generate synthetic PID for this command
                                let synthetic_pid = Self::generate_synthetic_pid();
                            
                                let mut event = ProcessEvent::new(synthetic_pid, command, ProbeSource::LinuxShell)
                                    .with_shell_session_id(msg.session_id.clone());

                                // Decode working directory from base64 or use plain text
                                if let Some(wd_b64) = msg.working_directory_b64 {
                                    match general_purpose::STANDARD.decode(&wd_b64) {
                                        Ok(decoded_bytes) => {
                                            let wd = String::from_utf8_lossy(&decoded_bytes).to_string();
                                            event = event.with_working_directory(wd);
                                        },
                                        Err(_) => {
                                            if let Some(wd) = msg.working_directory {
                                                event = event.with_working_directory(wd);
                                            }
                                        }
                                    }
                                } else if let Some(wd) = msg.working_directory {
                                    event = event.with_working_directory(wd);
                                }

                                if let Some(env) = msg.environment {
                                    event = event.with_environment(env);
                                }

                                // Store the session-to-PID mapping for later matching
                                {
                                    let mut sessions = active_sessions.lock().await;
                                    sessions.insert(msg.session_id.clone(), synthetic_pid);
                                }

                                debug!("Shell command started with synthetic PID {}: {}", synthetic_pid, event.command);
                            
                                let lifecycle_event = ProcessLifecycleEvent::Started(event);
                                if let Err(e) = lifecycle_sender.send(lifecycle_event) {
                                    debug!("Failed to send start event: {}", e);
                                }
                            }
                            ShellEventType::CommandEnd => {
                                // Remove from active sessions and send end event with the stored PID
                                let mut sessions = active_sessions.lock().await;
                                if let Some(synthetic_pid) = sessions.remove(&msg.session_id) {
                                    debug!("Shell command ended with synthetic PID {}", synthetic_pid);
                                
                                    let lifecycle_event = ProcessLifecycleEvent::Ended {
                                        pid: synthetic_pid,
                                    };
                                    if let Err(e) = lifecycle_sender.send(lifecycle_event) {
                                        debug!("Failed to send end event: {}", e);
                                    }
                                } else {
                                    debug!("Received end event for unknown session: {}", msg.session_id);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse shell message '{}': {}", json_str, e);
                        // Don't return error - just log and continue processing other messages
                    }
                }
            }
        }

        Ok(())
    }
}

impl PlatformProbeTrait for LinuxShellProbe {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Linux shell probe for process monitoring");

        // First, set up shell hooks (with user approval)
        if let Err(e) = self.setup_shell_hooks().await {
            warn!("Shell hook setup failed: {}", e);
            return Err(e);
        }

        // Then start the socket server
        self.start_socket_server().await?;

        info!("Linux shell probe started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping Linux shell probe");

        // Remove socket file
        if self.socket_path.exists() {
            if let Err(e) = fs::remove_file(&self.socket_path) {
                warn!("Failed to remove socket file: {}", e);
            }
        }

        // Clear active sessions
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.clear();
        }

        info!("Linux shell probe stopped");
        Ok(())
    }

    fn get_capability(&self) -> PlatformCapability {
        PlatformCapability::ShellOnly
    }
}