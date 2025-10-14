use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub struct Config {
    pub rules: Vec<Rule>,
    /// Optional override for viberot home directory
    /// If not specified, uses environment variable or platform defaults
    pub viberot_home: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
#[serde(untagged)]
pub enum Commands {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
#[serde(untagged)]
pub enum Actions {
    Single(Action),
    Multiple(Vec<Action>),
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub struct Rule {
    #[serde(alias = "commands")]
    pub command: Commands,
    #[serde(alias = "actions")]
    pub action: Actions,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "exec")]
    Executable { 
        path: String, 
        args: Option<Vec<String>>,
        #[serde(default)]
        single_instance: bool,
    },
    #[serde(rename = "lua")]
    Lua { 
        script: String,
        #[serde(default)]
        single_instance: bool,
    },
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(format!("Config file not found at {:?}. Please run first-time setup.", path).into());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        info!("Loaded config with {} rules", config.rules.len());
        Ok(config)
    }
}

impl Commands {
    pub fn as_vec(&self) -> Vec<&String> {
        match self {
            Commands::Single(cmd) => vec![cmd],
            Commands::Multiple(cmds) => cmds.iter().collect(),
        }
    }
}

impl Actions {
    pub fn as_vec(&self) -> Vec<&Action> {
        match self {
            Actions::Single(action) => vec![action],
            Actions::Multiple(actions) => actions.iter().collect(),
        }
    }
    
    pub fn into_vec(self) -> Vec<Action> {
        match self {
            Actions::Single(action) => vec![action],
            Actions::Multiple(actions) => actions,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: vec![],
            viberot_home: None,
        }
    }
}