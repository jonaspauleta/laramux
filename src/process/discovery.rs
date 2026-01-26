use std::path::Path;

use serde::Deserialize;

use crate::error::{LaraMuxError, Result};
use crate::process::types::{ProcessConfig, ProcessKind};

/// Check if Laravel Herd is installed on macOS
fn is_herd_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check for Herd application
        let herd_app = Path::new("/Applications/Herd.app");
        if herd_app.exists() {
            return true;
        }

        // Check for Herd config directory
        if let Some(home) = std::env::var_os("HOME") {
            let herd_config = Path::new(&home).join("Library/Application Support/Herd");
            if herd_config.exists() {
                return true;
            }
        }

        false
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[derive(Debug, Deserialize)]
struct ComposerJson {
    require: Option<std::collections::HashMap<String, String>>,
    #[serde(rename = "require-dev")]
    require_dev: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    scripts: Option<std::collections::HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<std::collections::HashMap<String, String>>,
    dependencies: Option<std::collections::HashMap<String, String>>,
}

/// Discover available Laravel services in the project
pub fn discover_services(working_dir: &Path) -> Result<Vec<ProcessConfig>> {
    let mut configs = Vec::new();

    // Check for Laravel (composer.json)
    let composer_path = working_dir.join("composer.json");
    if !composer_path.exists() {
        return Err(LaraMuxError::FileNotFound(
            "composer.json not found. Is this a Laravel project?".to_string(),
        ));
    }

    let composer_content = std::fs::read_to_string(&composer_path)?;
    let composer: ComposerJson = serde_json::from_str(&composer_content)?;

    // Check for Laravel framework
    let has_laravel = composer
        .require
        .as_ref()
        .map(|r| r.contains_key("laravel/framework"))
        .unwrap_or(false);

    if !has_laravel {
        return Err(LaraMuxError::Config(
            "Not a Laravel project (laravel/framework not found in composer.json)".to_string(),
        ));
    }

    // Add artisan serve (unless Laravel Herd is handling it)
    if !is_herd_installed() {
        configs.push(
            ProcessConfig::new(ProcessKind::Serve, "php", working_dir.to_path_buf())
                .with_args(vec!["artisan".to_string(), "serve".to_string()]),
        );
    }

    // Check for queue worker capability (always available in Laravel)
    configs.push(
        ProcessConfig::new(ProcessKind::Queue, "php", working_dir.to_path_buf()).with_args(vec![
            "artisan".to_string(),
            "queue:work".to_string(),
            "--tries=3".to_string(),
        ]),
    );

    // Check for Reverb (Laravel Reverb websocket server)
    let has_reverb = composer
        .require
        .as_ref()
        .map(|r| r.contains_key("laravel/reverb"))
        .unwrap_or(false)
        || composer
            .require_dev
            .as_ref()
            .map(|r| r.contains_key("laravel/reverb"))
            .unwrap_or(false);

    if has_reverb {
        configs.push(
            ProcessConfig::new(ProcessKind::Reverb, "php", working_dir.to_path_buf())
                .with_args(vec!["artisan".to_string(), "reverb:start".to_string()]),
        );
    }

    // Check for Vite (package.json)
    let package_path = working_dir.join("package.json");
    if package_path.exists() {
        let package_content = std::fs::read_to_string(&package_path)?;
        let package: PackageJson = serde_json::from_str(&package_content)?;

        let has_vite = package
            .dev_dependencies
            .as_ref()
            .map(|d| d.contains_key("vite") || d.contains_key("laravel-vite-plugin"))
            .unwrap_or(false)
            || package
                .dependencies
                .as_ref()
                .map(|d| d.contains_key("vite"))
                .unwrap_or(false);

        let has_dev_script = package
            .scripts
            .as_ref()
            .map(|s| s.contains_key("dev"))
            .unwrap_or(false);

        if has_vite && has_dev_script {
            // Detect package manager
            let npm_lock = working_dir.join("package-lock.json");
            let yarn_lock = working_dir.join("yarn.lock");
            let pnpm_lock = working_dir.join("pnpm-lock.yaml");
            let bun_lock = working_dir.join("bun.lockb");

            let (cmd, args) = if bun_lock.exists() {
                ("bun", vec!["run".to_string(), "dev".to_string()])
            } else if pnpm_lock.exists() {
                ("pnpm", vec!["run".to_string(), "dev".to_string()])
            } else if yarn_lock.exists() {
                ("yarn", vec!["dev".to_string()])
            } else if npm_lock.exists() {
                ("npm", vec!["run".to_string(), "dev".to_string()])
            } else {
                // Default to npm
                ("npm", vec!["run".to_string(), "dev".to_string()])
            };

            configs.push(
                ProcessConfig::new(ProcessKind::Vite, cmd, working_dir.to_path_buf())
                    .with_args(args),
            );
        }
    }

    Ok(configs)
}
