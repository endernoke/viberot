use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub command: String,
    pub timestamp: u64,
}

impl ProcessEvent {
    pub fn new(pid: u32, command: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            pid,
            command,
            timestamp,
        }
    }
}

pub struct EventBus {
    sender: broadcast::Sender<ProcessEvent>,
}

impl EventBus {
    pub fn new(sender: broadcast::Sender<ProcessEvent>) -> Self {
        Self { sender }
    }

    pub fn send_event(&self, event: ProcessEvent) -> Result<(), broadcast::error::SendError<ProcessEvent>> {
        self.sender.send(event).map(|_| ())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ProcessEvent> {
        self.sender.subscribe()
    }
}