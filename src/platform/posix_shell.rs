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

/// POSIX shell-based process probe
/// Uses shell hooks to monitor command execution in bash/zsh
pub struct PosixShellProbe {
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

impl PosixShellProbe {
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

    fn is_shell_integration_configured() -> bool {
        // Just check if .viberot/shell_integration file exists in home directory
        // and assume integration is set up if it does
        if let Some(home_dir) = dirs::home_dir() {
            let integration_file = home_dir.join(".viberot/shell_integration.sh");
            if integration_file.exists() {
                return true;
            }
        }

        false
    }

    async fn setup_shell_hooks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Skip setup if already configured
        if Self::is_shell_integration_configured() {
            debug!("Shell integration already configured, skipping setup");
            return Ok(());
        }

        println!();
        println!("Welcome to VibeRot!");
        println!("=================================");
        println!();
        println!("To enable VibeRot to react to shell commands, please do the following.");
        println!();
        println!("  cp scripts/shell_integration.sh $HOME/.viberot/shell_integration.sh");
        println!();
        println!("  echo '. \"$HOME/.viberot/shell_integration.sh\"' >> ~/.bashrc # for bash");
        println!("  echo '. \"$HOME/.viberot/shell_integration.sh\"' >> ~/.zshrc  # for zsh");
        println!();
        println!("Or manually add the following line to your shell configuration:");
        println!("  . \"$HOME/.viberot/shell_integration.sh\"");
        println!();
        println!("IMPORTANT: For bash users:");
        println!("  You need to install preexec and precmd functions for bash.");
        println!("  See: https://github.com/rcaloras/bash-preexec");
        println!();
        println!("  curl https://raw.githubusercontent.com/rcaloras/bash-preexec/master/bash-preexec.sh -o ~/.bash-preexec.sh");
        println!("  source ~/.bash-preexec.sh");
        println!();
        
        print!("Would you like VibeRot to set things up automatically for you? [y/N]: ");
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase().starts_with('y') {
            self.setup_automatic_integration().await?;
            println!("Restart your shell or run the following to apply the changes:");
            println!("  . ~/.bashrc # for bash");
            println!("  . ~/.zshrc  # for zsh");
            println!();
        } else {
            println!("Cancelled.");
            println!("The service will continue running. You can set up shell integration later.");
            println!("Note: To suppress this prompt in the future without setting up integration, you can create an empty file at:");
            println!("  $HOME/.viberot/shell_integration.sh");
        }
        println!("=================================");
        println!();
        println!("You can now let VibeRot run in the background and it will launch brainrot when a configured command is executed.");
        println!("Tip: use nohup to keep it running after you close the terminal.");
        println!("  nohup target/release/viberot &");

        Ok(())
    }

    async fn setup_automatic_integration(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        let integration_file = home_dir.join(".viberot").join("shell_integration.sh");
        fs::create_dir_all(home_dir.join(".viberot"))?;
        let bash_config_file = home_dir.join(format!(".bashrc"));
        let zsh_config_file = home_dir.join(format!(".zshrc"));

        // Install preexec for bash
        let mut performed_bash_preexec_install = false;
        if bash_config_file.exists() {
            // Borrow some technical debt and just use system command to fetch the file
            if !home_dir.join(".bash-preexec.sh").exists() {
                println!("Installing bash-preexec for bash...");
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg("curl https://raw.githubusercontent.com/rcaloras/bash-preexec/master/bash-preexec.sh -o ~/.bash-preexec.sh")
                    .status()?;
                if !status.success() {
                    warn!("Failed to download bash-preexec.sh, please install it manually with the instructions above.");
                } else {
                    performed_bash_preexec_install = true;
                    let source_line = "\n# Load bash-preexec for VibeRot\nif [ -f \"$HOME/.bash-preexec.sh\" ]; then\n  source \"$HOME/.bash-preexec.sh\"\nfi\n";
                    fs::OpenOptions::new()
                        .create(false)
                        .append(true)
                        .open(&bash_config_file)?
                        .write_all(source_line.as_bytes())?;
                }
            } else {
                debug!("bash-preexec already imported in this session");
            }
        }
        
        // Get the absolute path to the viberot.sh script
        let current_dir = env::current_dir()?;
        let script_path = current_dir.join("scripts").join("viberot.sh");

        if !script_path.exists() {
            return Err(format!("Could not find script at expected path: {}", script_path.display()).into());
        }
        fs::write(&integration_file, fs::read_to_string(&script_path)?)?;
        
        // Add line to source the integration file in shell config
        let source_line = format!("\n# VibeRot shell integration\n. \"{}\"\n", integration_file.display());
        
        if bash_config_file.exists() {
            fs::OpenOptions::new()
                .create(false)
                .append(true)
                .open(&bash_config_file)?
                .write_all(source_line.as_bytes())?;
        }
        if zsh_config_file.exists() {
            fs::OpenOptions::new()
                .create(false)
                .append(true)
                .open(&zsh_config_file)?
                .write_all(source_line.as_bytes())?;
        }
        
        println!("\n✅ Shell integration installed successfully!");
        println!("  Created: {}", integration_file.display());
        if performed_bash_preexec_install {
            println!("  Created: $HOME/.bash-preexec.sh");
        }
        if bash_config_file.exists() {
            println!("  Updated: {}", bash_config_file.display());
        }
        if zsh_config_file.exists() {
            println!("  Updated: {}", zsh_config_file.display());
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
                            
                                let mut event = ProcessEvent::new(synthetic_pid, command, ProbeSource::PosixShell)
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

impl PlatformProbeTrait for PosixShellProbe {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting POSIX shell probe for process monitoring");

        // Write socket path to ~/.viberot/.socket file for shell integration
        if let Some(home_dir) = dirs::home_dir() {
            let viberot_dir = home_dir.join(".viberot");
            fs::create_dir_all(&viberot_dir)?;
            let socket_file = viberot_dir.join(".socket");
            fs::write(&socket_file, self.socket_path.to_string_lossy().as_bytes())?;
            info!("Wrote socket path to: {}", socket_file.display());
        } else {
            warn!("Could not find home directory, shell integration may not work");
        }

        // First, set up shell hooks (with user approval)
        // Continue running the service regardless of setup success/failure
        if let Err(e) = self.setup_shell_hooks().await {
            warn!("Shell hook setup failed: {}", e);
            println!("\n⚠️  Shell integration setup was not completed.");
            println!("The VibeRot service will continue running, but shell command monitoring will not work");
            println!("until you manually set up the integration as described above.\n");
        }

        // Then start the socket server
        self.start_socket_server().await?;

        info!("POSIX shell probe started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping POSIX shell probe");

        // Remove socket file
        if self.socket_path.exists() {
            if let Err(e) = fs::remove_file(&self.socket_path) {
                warn!("Failed to remove socket file: {}", e);
            }
        }
        // Remove socket path file
        if let Some(home_dir) = dirs::home_dir() {
            let socket_file = home_dir.join(".viberot").join(".socket");
            if socket_file.exists() {
                if let Err(e) = fs::remove_file(&socket_file) {
                    warn!("Failed to remove socket path file: {}", e);
                }
            }
        }

        // Clear active sessions
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.clear();
        }

        info!("POSIX shell probe stopped");
        Ok(())
    }

    fn get_capability(&self) -> PlatformCapability {
        PlatformCapability::ShellOnly
    }
}