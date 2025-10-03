use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

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
            warn!("Config file not found at {:?}, creating default config", path);
            let default_config = Self::default();
            default_config.save_with_comments(path)?;
            return Ok(default_config);
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        info!("Loaded config with {} rules", config.rules.len());
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_with_comments<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = r#"# VibeRot Configuration File
# 
# This file defines rules for intercepting and handling commands.
# Each rule consists of command pattern(s) and action(s) to execute.
#
# Rules support the following:
# - Single command or array of commands
# - Single action or array of actions

# Global Configuration:
# viberot_home = "/custom/path/to/viberot"  # Optional: Override viberot installation directory
#                                           # If not set, uses VIBEROT_HOME env var or platform defaults

# Example configuration structures:

# Basic rule with single command and single action:
# [[rules]]
# command = "*npm-cli.js* install *"
# [rules.action]
# type = "exec"
# path = "C:\\path\\to\\action.exe"
# args = ["--arg1", "--arg2"]
# single_instance = true

# Rule with multiple commands mapping to the same action:
# [[rules]]
# command = ["*cargo.exe build*", "*cargo.exe check*", "*cargo.exe test*"]
# [rules.action]
# type = "exec"
# path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay.exe"
# args = ["--exit-on-stdin-close"]
# single_instance = true

# Rule with single command mapping to multiple actions:
# [[rules]]
# command = "*docker* build*"
# action = [
#   { type = "exec", path = "python", args = ["actions/example/info.py"] },
#   { type = "exec", path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay.exe", args = ["--exit-on-stdin-close"], single_instance = true }
# ]

# Rule with multiple commands mapping to multiple actions:
# [[rules]]
# command = ["*npm* install*", "*yarn* install*", "*pnpm* install*"]
# action = [
#   { type = "exec", path = "python", args = ["scripts/package-install-notify.py"] },
#   { type = "exec", path = "notepad.exe", args = ["package-install-log.txt"] }
# ]

# Alternative syntax using aliases:
# [[rules]]
# commands = ["*git* push*", "*git* pull*"]  # "commands" is an alias for "command"
# actions = [                               # "actions" is an alias for "action"
#   { type = "exec", path = "python", args = ["scripts/git-notify.py"] }
# ]

# Path Resolution:
# - Executable names (e.g., "python", "notepad.exe") are found via PATH
# - Absolute paths (e.g., "C:\path\to\action.exe") are used as-is
# - Relative paths are resolved relative to the viberot project root
# - Environment variables are supported:
#   - ${VIBEROT_HOME}: viberot project root
#   - ${VIBEROT_ACTIONS}: viberot/actions directory
#   - Other standard environment variables work too

# Single command and single action
[[rules]]
command = "*cargo build*"
[rules.action]
type = "exec"
path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay"
args = ["--exit-on-stdin-close"]
single_instance = true

# Multiple commands with same action
[[rules]]
command = ["*cargo build*", "*cargo run*"]
[rules.action]
type = "exec"
path = "python3"
args = ["actions/example/info.py"] # CWD is project root so python can find the script
single_instance = false # Allow multiple instances (default behavior)

# Single command with multiple actions
[[rules]]
command = "*cargo install*"
action = [
  { type = "exec", path = "sh", args = ["-c", "while true; do echo -ne '\\a'; sleep 0.1; done"], single_instance = true },
  { type = "exec", path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay", args = ["--exit-on-stdin-close"], single_instance = true }
]"#;
        std::fs::write(path, content)?;
        Ok(())
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