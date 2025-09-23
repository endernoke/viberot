use crate::config::{Config, Action};
use globset::{Glob, GlobSetBuilder};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

pub struct RuleEngine {
    config: Arc<RwLock<Config>>,
}

impl RuleEngine {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self { config }
    }

    pub async fn match_command(&self, command: &str, config: &Config) -> Option<Action> {
        // Create a glob set from all rules
        let mut builder = GlobSetBuilder::new();
        for rule in config.rules.iter() {
            match Glob::new(&rule.command) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(e) => {
                    error!("Invalid glob pattern '{}': {}", rule.command, e);
                }
            }
        }

        let glob_set = match builder.build() {
            Ok(set) => set,
            Err(e) => {
                error!("Failed to build glob set: {}", e);
                return None;
            }
        };

        // Find matching rules
        let matches = glob_set.matches(command);
        if let Some(&first_match) = matches.first() {
            debug!("Command '{}' matched rule: '{}'", command, config.rules[first_match].command);
            return Some(config.rules[first_match].action.clone());
        }

        debug!("No rules matched command: '{}'", command);
        None
    }
}