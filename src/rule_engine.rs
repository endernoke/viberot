use crate::config::{Config, Action};
use globset::{Glob, GlobSetBuilder, GlobSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

pub struct RuleEngine {
    cached_glob_data: Arc<RwLock<Option<CachedGlobData>>>,
}

struct CachedGlobData {
    config_hash: u64,
    glob_set: GlobSet,
    rules: Vec<Action>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            cached_glob_data: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn match_command(&self, command: &str, config: &Config) -> Option<Action> {
        // Check if we need to rebuild the cache
        let config_hash = self.calculate_config_hash(config);
        
        {
            let cached_data = self.cached_glob_data.read().await;
            if let Some(ref data) = *cached_data {
                if data.config_hash == config_hash {
                    // Cache hit - use existing glob set
                    let matches = data.glob_set.matches(command);
                    if let Some(&first_match) = matches.first() {
                        return Some(data.rules[first_match].clone());
                    }
                    return None;
                }
            }
        }

        // Cache miss - rebuild glob set
        self.rebuild_cache(config, config_hash).await;
        
        // Try matching again with the new cache
        let cached_data = self.cached_glob_data.read().await;
        if let Some(ref data) = *cached_data {
            let matches = data.glob_set.matches(command);
            if let Some(&first_match) = matches.first() {
                return Some(data.rules[first_match].clone());
            }
        }

        None
    }

    async fn rebuild_cache(&self, config: &Config, config_hash: u64) {
        let mut builder = GlobSetBuilder::new();
        let mut rules = Vec::new();
        
        for rule in config.rules.iter() {
            match Glob::new(&rule.command) {
                Ok(glob) => {
                    builder.add(glob);
                    rules.push(rule.action.clone());
                }
                Err(e) => {
                    error!("Invalid glob pattern '{}': {}", rule.command, e);
                }
            }
        }

        match builder.build() {
            Ok(glob_set) => {
                let new_data = CachedGlobData {
                    config_hash,
                    glob_set,
                    rules,
                };
                
                let mut cached_data = self.cached_glob_data.write().await;
                *cached_data = Some(new_data);
            }
            Err(e) => {
                error!("Failed to build glob set: {}", e);
            }
        }
    }

    fn calculate_config_hash(&self, config: &Config) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        config.hash(&mut hasher);
        hasher.finish()
    }

    /// Force invalidate the cache (useful for testing or manual refresh)
    #[allow(dead_code)]
    pub async fn invalidate_cache(&self) {
        let mut cached_data = self.cached_glob_data.write().await;
        *cached_data = None;
    }
}