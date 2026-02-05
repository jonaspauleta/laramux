use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::{LaraMuxError, Result};

/// Reserved hotkeys that cannot be assigned to custom processes
const RESERVED_HOTKEYS: &[char] = &['r', 'c'];

/// Configuration file name
const CONFIG_FILE: &str = ".laramux.json";

/// Default max log lines
const DEFAULT_MAX_LOG_LINES: u32 = 100;

/// Valid log levels for filtering
const VALID_LOG_LEVELS: &[&str] = &[
    "debug",
    "info",
    "notice",
    "warning",
    "error",
    "critical",
    "alert",
    "emergency",
];

/// Restart policy for processes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    #[default]
    Never,
    OnFailure,
    Always,
}

/// Configuration for disabling built-in processes
#[derive(Debug, Default, Deserialize, serde::Serialize)]
pub struct DisabledConfig {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub serve: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub vite: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub queue: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub horizon: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reverb: bool,
}

/// Configuration for overriding a process command
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct OverrideConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<RestartPolicy>,
}

/// Configuration for a custom process
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct CustomProcess {
    pub name: String,
    pub display_name: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<char>,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<RestartPolicy>,
}

fn is_true(b: &bool) -> bool {
    *b
}

fn default_enabled() -> bool {
    true
}

/// Custom quality/testing tool configuration
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct CustomTool {
    pub name: String,
    pub display_name: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    pub category: String,
}

/// Quality tools configuration
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct QualityConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_tools: Vec<CustomTool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub default_args: HashMap<String, Vec<String>>,
}

/// Artisan configuration
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct ArtisanConfig {
    /// List of favorite artisan command names (e.g., "migrate:fresh", "cache:clear")
    #[serde(default)]
    pub favorites: Vec<String>,
}

/// Make commands configuration
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct MakeConfig {
    /// List of favorite make command names (e.g., "make:model", "make:controller")
    #[serde(default)]
    pub favorites: Vec<String>,
}

/// Log configuration
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct LogConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_filter: Option<String>,
}

/// Main configuration structure
#[derive(Debug, Default, Deserialize, serde::Serialize)]
pub struct LaramuxConfig {
    /// Override Sail auto-detection: None = auto-detect, Some(true) = force, Some(false) = disable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sail: Option<bool>,
    #[serde(default, skip_serializing_if = "is_default_disabled")]
    pub disabled: DisabledConfig,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub overrides: HashMap<String, OverrideConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<CustomProcess>,
    #[serde(default, skip_serializing_if = "is_default_quality")]
    pub quality: QualityConfig,
    #[serde(default, skip_serializing_if = "is_default_logs")]
    pub logs: LogConfig,
    #[serde(default, skip_serializing_if = "is_default_artisan")]
    pub artisan: ArtisanConfig,
    #[serde(default, skip_serializing_if = "is_default_make")]
    pub make: MakeConfig,
}

fn is_default_disabled(d: &DisabledConfig) -> bool {
    !d.serve && !d.vite && !d.queue && !d.horizon && !d.reverb
}

fn is_default_quality(q: &QualityConfig) -> bool {
    q.disabled_tools.is_empty() && q.custom_tools.is_empty() && q.default_args.is_empty()
}

fn is_default_logs(l: &LogConfig) -> bool {
    l.max_lines.is_none() && l.files.is_none() && l.default_filter.is_none()
}

fn is_default_artisan(a: &ArtisanConfig) -> bool {
    a.favorites.is_empty()
}

fn is_default_make(m: &MakeConfig) -> bool {
    m.favorites.is_empty()
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

            // Validate working_dir if present
            if let Some(ref wd) = process.working_dir {
                Self::validate_working_dir(wd, &format!("custom process '{}'", process.name))?;
            }

            // Validate env keys if present
            if let Some(ref env) = process.env {
                Self::validate_env_keys(env, &format!("custom process '{}'", process.name))?;
            }
        }

        // Validate override configs
        for (name, override_cfg) in &self.overrides {
            if let Some(ref wd) = override_cfg.working_dir {
                Self::validate_working_dir(wd, &format!("override '{}'", name))?;
            }
            if let Some(ref env) = override_cfg.env {
                Self::validate_env_keys(env, &format!("override '{}'", name))?;
            }
        }

        // Validate quality config
        for tool in &self.quality.custom_tools {
            if tool.name.is_empty() {
                return Err(LaraMuxError::ConfigValidation(
                    "Custom tool name cannot be empty".to_string(),
                ));
            }
            if tool.display_name.is_empty() {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom tool '{}' must have a display_name",
                    tool.name
                )));
            }
            if tool.command.is_empty() {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom tool '{}' must have a command",
                    tool.name
                )));
            }
            if tool.category != "quality" && tool.category != "testing" {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Custom tool '{}' category must be 'quality' or 'testing', got '{}'",
                    tool.name, tool.category
                )));
            }
        }

        // Validate log config
        if let Some(max_lines) = self.logs.max_lines {
            if !(10..=10000).contains(&max_lines) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "logs.max_lines must be between 10 and 10000, got {}",
                    max_lines
                )));
            }
        }

        if let Some(ref filter) = self.logs.default_filter {
            let lower = filter.to_lowercase();
            if !VALID_LOG_LEVELS.contains(&lower.as_str()) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "logs.default_filter must be a valid log level, got '{}'. Valid levels: {}",
                    filter,
                    VALID_LOG_LEVELS.join(", ")
                )));
            }
        }

        // Validate artisan favorites (no empty strings, no duplicates)
        let mut seen = HashSet::new();
        for fav in &self.artisan.favorites {
            if fav.is_empty() {
                return Err(LaraMuxError::ConfigValidation(
                    "Artisan favorite command cannot be empty".to_string(),
                ));
            }
            if !seen.insert(fav) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Duplicate artisan favorite: '{}'",
                    fav
                )));
            }
        }

        // Validate make favorites (no empty strings, no duplicates)
        seen.clear();
        for fav in &self.make.favorites {
            if fav.is_empty() {
                return Err(LaraMuxError::ConfigValidation(
                    "Make favorite command cannot be empty".to_string(),
                ));
            }
            if !seen.insert(fav) {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "Duplicate make favorite: '{}'",
                    fav
                )));
            }
        }

        Ok(())
    }

    /// Validate working_dir: must be relative, no path traversal
    fn validate_working_dir(working_dir: &str, context: &str) -> Result<()> {
        let path = Path::new(working_dir);

        // Check for absolute path
        if path.is_absolute() {
            return Err(LaraMuxError::ConfigValidation(format!(
                "working_dir in {} must be a relative path, got '{}'",
                context, working_dir
            )));
        }

        // Check for path traversal
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "working_dir in {} cannot contain '..', got '{}'",
                    context, working_dir
                )));
            }
        }

        Ok(())
    }

    /// Validate env keys: non-empty, alphanumeric + underscore
    fn validate_env_keys(env: &HashMap<String, String>, context: &str) -> Result<()> {
        for key in env.keys() {
            if key.is_empty() {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "env key in {} cannot be empty",
                    context
                )));
            }
            if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(LaraMuxError::ConfigValidation(format!(
                    "env key '{}' in {} must be alphanumeric with underscores only",
                    key, context
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

    /// Get the configured max log lines or default
    pub fn log_max_lines(&self) -> usize {
        self.logs.max_lines.unwrap_or(DEFAULT_MAX_LOG_LINES) as usize
    }

    /// Get additional log files to watch
    pub fn additional_log_files(&self) -> &[String] {
        self.logs.files.as_deref().unwrap_or(&[])
    }

    /// Get the default log filter level
    pub fn default_log_filter(&self) -> Option<&str> {
        self.logs.default_filter.as_deref()
    }

    /// Check if a quality/testing tool is disabled
    pub fn is_tool_disabled(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.quality
            .disabled_tools
            .iter()
            .any(|t| t.to_lowercase() == name_lower)
    }

    /// Get default args for a tool
    pub fn tool_default_args(&self, name: &str) -> Option<&Vec<String>> {
        self.quality.default_args.get(name)
    }

    /// Get custom quality tools
    pub fn custom_quality_tools(&self) -> impl Iterator<Item = &CustomTool> {
        self.quality
            .custom_tools
            .iter()
            .filter(|t| t.category == "quality")
    }

    /// Get custom testing tools
    pub fn custom_testing_tools(&self) -> impl Iterator<Item = &CustomTool> {
        self.quality
            .custom_tools
            .iter()
            .filter(|t| t.category == "testing")
    }

    /// Get favorite artisan command names
    pub fn artisan_favorites(&self) -> &[String] {
        &self.artisan.favorites
    }

    /// Check if an artisan command is a favorite
    #[allow(dead_code)]
    pub fn is_artisan_favorite(&self, command: &str) -> bool {
        self.artisan.favorites.iter().any(|f| f == command)
    }

    /// Toggle an artisan command as favorite
    pub fn toggle_artisan_favorite(&mut self, command: &str) {
        if let Some(pos) = self.artisan.favorites.iter().position(|f| f == command) {
            self.artisan.favorites.remove(pos);
        } else {
            self.artisan.favorites.push(command.to_string());
        }
    }

    /// Get favorite make command names
    pub fn make_favorites(&self) -> &[String] {
        &self.make.favorites
    }

    /// Check if a make command is a favorite
    #[allow(dead_code)]
    pub fn is_make_favorite(&self, command: &str) -> bool {
        self.make.favorites.iter().any(|f| f == command)
    }

    /// Toggle a make command as favorite
    pub fn toggle_make_favorite(&mut self, command: &str) {
        if let Some(pos) = self.make.favorites.iter().position(|f| f == command) {
            self.make.favorites.remove(pos);
        } else {
            self.make.favorites.push(command.to_string());
        }
    }

    /// Save configuration to file
    pub fn save(&self, working_dir: &Path) -> Result<()> {
        let config_path = working_dir.join(CONFIG_FILE);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
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

    #[test]
    fn test_override_with_new_fields() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "overrides": {
                    "serve": {
                        "command": "php",
                        "args": ["artisan", "serve", "--port=8080"],
                        "working_dir": "backend",
                        "env": {"APP_DEBUG": "true"},
                        "restart_policy": "on_failure"
                    }
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        let serve_override = config.get_override("serve").unwrap();
        assert_eq!(serve_override.command.as_ref().unwrap(), "php");
        assert_eq!(serve_override.working_dir.as_ref().unwrap(), "backend");
        assert_eq!(
            serve_override
                .env
                .as_ref()
                .unwrap()
                .get("APP_DEBUG")
                .unwrap(),
            "true"
        );
        assert_eq!(
            serve_override.restart_policy,
            Some(RestartPolicy::OnFailure)
        );
    }

    #[test]
    fn test_custom_process_with_new_fields() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {
                        "name": "scheduler",
                        "display_name": "Scheduler",
                        "command": "php",
                        "args": ["artisan", "schedule:work"],
                        "hotkey": "d",
                        "working_dir": "backend",
                        "env": {"LOG_LEVEL": "debug"},
                        "restart_policy": "always"
                    }
                ]
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        let custom = &config.custom[0];
        assert_eq!(custom.working_dir.as_ref().unwrap(), "backend");
        assert_eq!(
            custom.env.as_ref().unwrap().get("LOG_LEVEL").unwrap(),
            "debug"
        );
        assert_eq!(custom.restart_policy, Some(RestartPolicy::Always));
    }

    #[test]
    fn test_invalid_working_dir_absolute() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {
                        "name": "test",
                        "display_name": "Test",
                        "command": "php",
                        "working_dir": "/absolute/path"
                    }
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("relative path"));
    }

    #[test]
    fn test_invalid_working_dir_traversal() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {
                        "name": "test",
                        "display_name": "Test",
                        "command": "php",
                        "working_dir": "../outside"
                    }
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".."));
    }

    #[test]
    fn test_invalid_env_key() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "custom": [
                    {
                        "name": "test",
                        "display_name": "Test",
                        "command": "php",
                        "env": {"invalid-key": "value"}
                    }
                ]
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("alphanumeric"));
    }

    #[test]
    fn test_quality_config() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "quality": {
                    "disabled_tools": ["phpcs"],
                    "custom_tools": [
                        {
                            "name": "custom-lint",
                            "display_name": "Custom Linter",
                            "command": "./scripts/lint.sh",
                            "category": "quality"
                        }
                    ],
                    "default_args": {
                        "phpstan": ["--memory-limit=512M"]
                    }
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert!(config.is_tool_disabled("phpcs"));
        assert!(!config.is_tool_disabled("phpstan"));
        assert_eq!(
            config.tool_default_args("phpstan").unwrap(),
            &vec!["--memory-limit=512M"]
        );
        let custom_tools: Vec<_> = config.custom_quality_tools().collect();
        assert_eq!(custom_tools.len(), 1);
        assert_eq!(custom_tools[0].name, "custom-lint");
    }

    #[test]
    fn test_invalid_custom_tool_category() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "quality": {
                    "custom_tools": [
                        {
                            "name": "test",
                            "display_name": "Test",
                            "command": "echo",
                            "category": "invalid"
                        }
                    ]
                }
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("quality' or 'testing"));
    }

    #[test]
    fn test_log_config() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "logs": {
                    "max_lines": 500,
                    "files": ["storage/logs/queue.log"],
                    "default_filter": "warning"
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(config.log_max_lines(), 500);
        assert_eq!(config.additional_log_files(), &["storage/logs/queue.log"]);
        assert_eq!(config.default_log_filter(), Some("warning"));
    }

    #[test]
    fn test_log_max_lines_validation() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), r#"{"logs": {"max_lines": 5}}"#);
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("10 and 10000"));

        write_config(dir.path(), r#"{"logs": {"max_lines": 50000}}"#);
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_log_default_filter_validation() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), r#"{"logs": {"default_filter": "invalid"}}"#);
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("valid log level"));
    }

    #[test]
    fn test_restart_policy_deserialization() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "overrides": {
                    "serve": {"restart_policy": "never"},
                    "queue": {"restart_policy": "on_failure"},
                    "horizon": {"restart_policy": "always"}
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(
            config.get_override("serve").unwrap().restart_policy,
            Some(RestartPolicy::Never)
        );
        assert_eq!(
            config.get_override("queue").unwrap().restart_policy,
            Some(RestartPolicy::OnFailure)
        );
        assert_eq!(
            config.get_override("horizon").unwrap().restart_policy,
            Some(RestartPolicy::Always)
        );
    }

    #[test]
    fn test_artisan_favorites() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "artisan": {
                    "favorites": ["migrate:fresh", "cache:clear", "optimize:clear"]
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(config.artisan_favorites().len(), 3);
        assert!(config.is_artisan_favorite("migrate:fresh"));
        assert!(config.is_artisan_favorite("cache:clear"));
        assert!(!config.is_artisan_favorite("migrate"));
    }

    #[test]
    fn test_artisan_favorites_empty_string() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "artisan": {
                    "favorites": ["migrate", ""]
                }
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_artisan_favorites_duplicate() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "artisan": {
                    "favorites": ["migrate", "migrate"]
                }
            }"#,
        );
        let result = LaramuxConfig::load(dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate artisan favorite"));
    }

    #[test]
    fn test_toggle_artisan_favorite() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "artisan": {
                    "favorites": ["migrate"]
                }
            }"#,
        );
        let mut config = LaramuxConfig::load(dir.path()).unwrap().unwrap();

        // Add a new favorite
        config.toggle_artisan_favorite("cache:clear");
        assert!(config.is_artisan_favorite("cache:clear"));
        assert_eq!(config.artisan_favorites().len(), 2);

        // Remove existing favorite
        config.toggle_artisan_favorite("migrate");
        assert!(!config.is_artisan_favorite("migrate"));
        assert_eq!(config.artisan_favorites().len(), 1);
    }

    #[test]
    fn test_make_favorites() {
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"{
                "make": {
                    "favorites": ["make:model", "make:controller"]
                }
            }"#,
        );
        let config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(config.make_favorites().len(), 2);
        assert!(config.is_make_favorite("make:model"));
        assert!(!config.is_make_favorite("make:migration"));
    }

    #[test]
    fn test_save_config() {
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), r#"{}"#);

        let mut config = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        config.toggle_artisan_favorite("migrate:fresh");
        config.toggle_make_favorite("make:model");
        config.save(dir.path()).unwrap();

        // Reload and verify
        let config2 = LaramuxConfig::load(dir.path()).unwrap().unwrap();
        assert!(config2.is_artisan_favorite("migrate:fresh"));
        assert!(config2.is_make_favorite("make:model"));
    }
}
