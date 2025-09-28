use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub struct Rule {
    pub command: String,
    pub action: Action,
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
# Each rule consists of a command pattern and an action to execute.

# Example configuration structure:
#
# [[rules]]
# command = "*npm-cli.js* install *"
# # Use the above pattern to match "npm install" 
# # because when you run npm install, the actual 
# # expanded commandline is something like 
# # '"/path/to/nodejs" "/path/to/npm-cli.js" install ...'
# 
# # Execute a program or script
# [rules.action]
# type = "exec"
# path = "C:\\path\\to\\action.exe"  # Absolute path to executable
# args = ["--arg1", "--arg2"]  # Optional arguments (remove this line if no args needed)
# single_instance = true  # Optional: only allow one instance of this action to run at a time (default: false)
#
# [[rules]]
# command = "*pip.exe install *"
# 
# [rules.action]
# type = "exec"
# path = "python"  # Executable name (searched in PATH)
# args = ["C:\\path\\to\\python\\script", "--arg1"]  # Optional arguments (remove this line if no args needed)
# single_instance = false  # Optional: allow multiple instances (default: false)
#
# # Path Resolution:
# # - Executable names (e.g., "python", "notepad.exe") are found via PATH
# # - Absolute paths (e.g., "C:\path\to\action.exe") are used as-is
# # - Relative paths are resolved relative to the viberot project root
# # - Environment variables are supported:
# #   - ${VIBEROT_HOME}: viberot project root
# #   - ${VIBEROT_ACTIONS}: viberot/actions directory
# #   - Other standard environment variables work too
# #
# # Examples:
# # path = "actions/overlay/target/release/viberot-overlay.exe"  # Relative to project root
# # path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay.exe"
# # path = "${USERPROFILE}/my-scripts/notify.py"

[[rules]]
command = "*cargo.exe build*" # This rule matches any `cargo build` command
[rules.action]
type = "exec"
path = "${VIBEROT_ACTIONS}/overlay/target/release/viberot-overlay.exe" # enjoy your brainrot
args = ["--exit-on-stdin-close"]
single_instance = true # Only allow one overlay instance at a time

[[rules]]
command = "*cargo.exe run*"
[rules.action]
type = "exec"
path = "python"
args = ["actions/example/info.py"] # CWD is project root so python can find the script
single_instance = false # Allow multiple instances (default behavior)
"#;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: vec![],
        }
    }
}