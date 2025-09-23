use tokio::net::windows::named_pipe::ServerOptions;
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, error, warn};

use crate::event_bus::ProcessEvent;

const PIPE_NAME: &str = r"\\.\pipe\command-sidekick-events";

pub struct WindowsProbe {
    event_sender: broadcast::Sender<ProcessEvent>,
}

impl WindowsProbe {
    pub fn new(event_sender: broadcast::Sender<ProcessEvent>) -> Self {
        Self { event_sender }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Windows probe - listening on named pipe: {}", PIPE_NAME);

        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .max_instances(1)
                .create(PIPE_NAME)?;

            info!("Waiting for .NET probe to connect...");
            server.connect().await?;
            info!(".NET probe connected");

            // Create an async buffered reader
            let reader = BufReader::new(server);
            let mut lines = reader.lines();
            let sender = self.event_sender.clone();

            // Read events line by line
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if let Err(e) = process_event_line(&line.trim(), &sender) {
                            error!("Failed to process event: {}", e);
                        }
                    }
                    Ok(None) => {
                        info!("Probe disconnected, waiting for reconnection...");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from pipe: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

fn process_event_line(line: &str, sender: &broadcast::Sender<ProcessEvent>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if line.is_empty() {
        return Ok(());
    }

    let event: ProcessEvent = serde_json::from_str(line)?;
    
    if let Err(e) = sender.send(event.clone()) {
        warn!("Failed to send event to bus: {}", e);
    }

    Ok(())
}