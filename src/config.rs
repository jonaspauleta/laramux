use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::{LaraMuxError, Result};

/// Reserved hotkeys that cannot be assigned to custom processes
const RESERVED_HOTKEYS: &[char] = &['r', 'c'];

/// Configuration file name
const CONFIG_FILE: &str = ".laramux.json";

/// Configuration for disabling built-in processes
#[derive(Debug, Default, Deserialize)]
pub struct DisabledConfig {
    #[serde(default)]
    pub serve: bool,
    #[serde(default)]
    pub vite: bool,
    #[serde(default)]
    pub queue: bool,
    #[serde(default)]
    pub horizon: bool,
    #[serde(default)]
    pub reverb: bool,
}

/// Configuration for overriding a process command
#[derive(Debug, Clone, Deserialize)]
pub struct OverrideConfig {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
}

/// Configuration for a custom process
#[derive(Debug, Clone, Deserialize)]
pub struct CustomProcess {
    pub name: String,
    pub display_name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub hotkey: Option<char>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Main configuration structure
#[derive(Debug, Default, Deserialize)]
pub struct LaramuxConfig {
    #[serde(default)]
    pub disabled: DisabledConfig,
    #[serde(default)]
    pub overrides: HashMap<String, OverrideConfig>,
    #[serde(default)]
    pub custom: Vec<CustomProcess>,
}

impl LaramuxConfig {
    /// Load configuration from .laramux.json if it exists
    pub fn load(working_dir: &Path) -> Result<Option<Self>> {
        let config_path = working_dir.join(CONFIG_FILE);

        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: LaramuxConfig = serde_json::from_str(&content)?;

        config.validate()?;

        Ok(Some(config))
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        let mut custom_names = HashSet::new();
        let mut custom_hotkeys = HashSet::new();

        // Built-in hotkeys
        let builtin_hotkeys: HashSet<char> = ['s', 'v', 'q', 'h', 'b'].into_iter().collect();

        for process in &self.custom {
            // Check for duplicate names
            if !custom_names.insert(&process.name) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Duplicate custom process name: '{}'",
                    process.name
                )));
            }

            // Check for reserved names
            let reserved_names = ["serve", "vite", "queue", "horizon", "reverb"];
            if reserved_names.contains(&process.name.to_lowercase().as_str()) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom process name '{}' conflicts with built-in process",
                    process.name
                )));
            }

            // Validate hotkey
            if let Some(hotkey) = process.hotkey {
                // Check reserved hotkeys
                if RESERVED_HOTKEYS.contains(&hotkey) {
                    return Err(LaraMuxError::ConfigValidation(format!(
                        "Hotkey '{}' is reserved for system use",
                        hotkey
                    )));
                }

                // Check builtin hotkeys
                if builtin_hotkeys.contains(&hotkey) {
                    return Err(LaraMuxError::ConfigValidation(format!(
                        "Hotkey '{}' conflicts with built-in process hotkey",
                        hotkey
                    )));
                }

                // Check duplicate custom hotkeys
                if !custom_hotkeys.insert(hotkey) {
                    return Err(LaraMuxError::ConfigValidation(format!(
                        "Duplicate hotkey '{}' in custom processes",
                        hotkey
                    )));
                }

                // Ensure hotkey is a letter
                if !hotkey.is_ascii_lowercase() {
                    return Err(LaraMuxError::ConfigValidation(format!(
                        "Hotkey '{}' must be a lowercase letter",
                        hotkey
                    )));
                }
            }

            // Validate required fields
            if process.name.is_empty() {
                return Err(LaraMuxError::ConfigValidation(
                    "Custom process name cannot be empty".to_string(),
                ));
            }

            if process.display_name.is_empty() {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom process '{}' must have a display_name",
                    process.name
                )));
            }

            if process.command.is_empty() {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom process '{}' must have a command",
                    process.name
                )));
            }
        }

        Ok(())
    }

    /// Check if a built-in process is disabled
    pub fn is_disabled(&self, name: &str) -> bool {
        match name.to_lowercase().as_str() {
            "serve" => self.disabled.serve,
            "vite" => self.disabled.vite,
            "queue" => self.disabled.queue,
            "horizon" => self.disabled.horizon,
            "reverb" => self.disabled.reverb,
            _ => false,
        }
    }

    /// Get override for a process
    pub fn get_override(&self, name: &str) -> Option<&OverrideConfig> {
        self.overrides.get(name)
    }

    /// Get enabled custom processes
    pub fn enabled_custom_processes(&self) -> impl Iterator<Item = &CustomProcess> {
        self.custom.iter().filter(|p| p.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_config(dir: &Path, content: &str) {
        let path = dir.join(CONFIG_FILE);
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_no_config() {
        let dir = TempDir::new().unwrap();
        let config = LaramuxConfig::load(dir.path()).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn test_load_empty_config() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "{}");
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert!(!config.disabled.serve);
        assert!(config.custom.is_empty());
    }

    #[test]
    fn test_disabled_processes() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{"disabled": {"serve": true, "vite": false}}"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert!(config.is_disabled("serve"));
        assert!(!config.is_disabled("vite"));
        assert!(!config.is_disabled("queue"));
    }

    #[test]
    fn test_custom_processes() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {
                        "name": "scheduler",
                        "display_name": "Scheduler",
                        "hotkey": "d",
                        "command": "php",
                        "args": ["artisan", "schedule:work"]
                    }
                ]
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(config.custom.len(), 1);
        assert_eq!(config.custom[0].name, "scheduler");
        assert_eq!(config.custom[0].hotkey, Some('d'));
    }

    #[test]
    fn test_duplicate_name_validation() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {"name": "test", "display_name": "Test", "command": "echo"},
                    {"name": "test", "display_name": "Test2", "command": "echo"}
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_reserved_hotkey_validation() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {"name": "test", "display_name": "Test", "command": "echo", "hotkey": "r"}
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_hotkey_conflict() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {"name": "test", "display_name": "Test", "command": "echo", "hotkey": "s"}
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_overrides() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "overrides": {
                    "serve": {
                        "args": ["artisan", "serve", "--port=8080"]
                    }
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        let serve_override = config.get_override("serve").unwrap();
        assert!(serve_override.command.is_none());
        assert_eq!(
            serve_override.args.as_ref().unwrap(),
            &vec!["artisan", "serve", "--port=8080"]
        );
    }
}
