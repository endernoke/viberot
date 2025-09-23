use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub command: String,
    pub action: Action,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "exec")]
    Executable { path: String, args: Option<Vec<String>> },
    #[serde(rename = "lua")]
    Lua { script: String },
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();
        
        if !path.exists() {
            warn!("Config file not found at {:?}, creating default config", path);
            let default_config = Self::default();
            default_config.save(path)?;
            return Ok(default_config);
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        info!("Loaded config with {} rules", config.rules.len());
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: vec![
                Rule {
                    command: "npm install*".to_string(),
                    action: Action::Executable {
                        path: "spinner-plugin.exe".to_string(),
                        args: None,
                    },
                },
                Rule {
                    command: "npm ci*".to_string(),
                    action: Action::Executable {
                        path: "spinner-plugin.exe".to_string(),
                        args: None,
                    },
                },
            ],
        }
    }
}