use std::path::PathBuf;
use std::fs;
use std::time::SystemTime;
use tracing::{info, warn};

use crate::platform::PlatformProbeTrait;

/// Manages first-run setup for VibeRot
pub struct SetupManager;

impl SetupManager {
    /// Check if first-run setup is needed
    pub fn is_setup_needed() -> bool {
        if let Some(home_dir) = dirs::home_dir() {
            let setup_marker = home_dir.join(".viberot").join(".setup_complete");
            !setup_marker.exists()
        } else {
            // If we can't find home directory, assume setup is needed
            true
        }
    }

    /// Get the path to the viberot home directory
    fn get_viberot_home() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let home_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?;
        Ok(home_dir.join(".viberot"))
    }

    /// Get the appropriate rules template path based on the current OS
    fn get_rules_template_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Get the project root (where Cargo.toml is)
        let project_root = Self::find_project_root()?;
        
        let template_name = if cfg!(windows) {
            "rules.win.toml"
        } else {
            "rules.toml"
        };
        
        let template_path = project_root.join("rules").join(template_name);
        
        if !template_path.exists() {
            return Err(format!("Rules template not found at: {}", template_path.display()).into());
        }
        
        Ok(template_path)
    }

    /// Find the project root by looking for Cargo.toml
    fn find_project_root() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Try from executable path first
        if let Ok(exe_path) = std::env::current_exe() {
            let mut current = exe_path.parent();
            while let Some(dir) = current {
                if dir.join("Cargo.toml").exists() && 
                   dir.join("rules").exists() {
                    return Ok(dir.to_path_buf());
                }
                current = dir.parent();
            }
        }
        
        // Fallback: try current working directory
        let mut current = std::env::current_dir()?;
        for _ in 0..5 {
            if current.join("Cargo.toml").exists() && 
               current.join("rules").exists() {
                return Ok(current);
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        Err("Could not find project root. Expected to find Cargo.toml with rules/ directory.".into())
    }

    /// Perform first-run setup
    pub async fn run_setup<P: PlatformProbeTrait>(
        probe: &P
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Running first-run setup...");
        
        println!();
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║                    Welcome to VibeRot! 🎉                     ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("This appears to be your first time running VibeRot.");
        println!("Let's get you set up...");
        println!();

        // Create viberot home directory structure
        let viberot_home = Self::get_viberot_home()?;
        fs::create_dir_all(&viberot_home)?;
        info!("Created VibeRot home directory: {}", viberot_home.display());

        // Create logs directory
        let logs_dir = viberot_home.join("logs");
        fs::create_dir_all(&logs_dir)?;
        info!("Created logs directory: {}", logs_dir.display());

        // Copy appropriate rules template to config.toml
        let config_path = viberot_home.join("config.toml");
        if !config_path.exists() {
            match Self::get_rules_template_path() {
                Ok(template_path) => {
                    let template_content = fs::read_to_string(&template_path)?;
                    fs::write(&config_path, template_content)?;
                    println!("✓ Created default configuration at: {}", config_path.display());
                    info!("Copied rules template from {} to {}", 
                          template_path.display(), config_path.display());
                }
                Err(e) => {
                    warn!("Could not find rules template: {}", e);
                    println!("⚠ Warning: Could not find default rules template.");
                    println!("  You'll need to create your config manually at: {}", config_path.display());
                    
                    // Create a minimal config so the service can start
                    let minimal_config = r#"# VibeRot Configuration File
# Add your rules here.
# See https://github.com/endernoke/viberot/tree/main/rules in the project for examples.
"#;
                    fs::write(&config_path, minimal_config)?;
                    info!("Created minimal fallback config");
                }
            }
        } else {
            println!("✓ Configuration file already exists: {}", config_path.display());
        }

        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("Platform-specific setup:");
        println!("═══════════════════════════════════════════════════════════════");
        println!();

        // Run platform-specific setup
        if let Err(e) = probe.setup().await {
            warn!("Platform-specific setup encountered an error: {}", e);
            println!("⚠ Platform-specific setup completed with warnings.");
            println!("  The service will continue, but some features may not work.");
            println!();
        } else {
            println!("✓ Platform-specific setup completed successfully.");
            println!();
        }

        // Mark setup as complete
        let setup_marker = viberot_home.join(".setup_complete");
        fs::write(&setup_marker, format!("Setup completed at: {}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()))?;
        info!("Setup marked as complete");

        println!("═══════════════════════════════════════════════════════════════");
        println!("✨ Setup complete! VibeRot is now ready to use.");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("Configuration file: {}", config_path.display());
        println!("Logs directory: {}", logs_dir.display());
        println!();
        println!("The service will now start. Try running a command in a new terminal.");
        println!("You can now leave VibeRot running in the background.");
        println!("To keep VibeRot running after closing this terminal, consider using:");
        println!("    nohup /path/to/viberot &");
        println!();

        Ok(())
    }
}
