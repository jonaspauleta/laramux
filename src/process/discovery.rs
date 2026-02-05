use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::config::LaramuxConfig;
use crate::error::{LaraMuxError, Result};
use crate::process::types::{ProcessConfig, ProcessId, ProcessKind, ProcessRegistry};

/// A discovered quality tool (PHPStan, Pint, Rector, Pest, etc.)
#[derive(Debug, Clone)]
pub struct QualityTool {
    /// Display name (e.g., "PHPStan")
    pub display_name: String,
    /// Command to run (e.g., "./vendor/bin/phpstan")
    pub command: String,
    /// Default arguments
    pub args: Vec<String>,
}

/// Result of service discovery
pub struct DiscoveryResult {
    pub is_sail: bool,
    pub configs: Vec<ProcessConfig>,
    pub registry: ProcessRegistry,
    pub artisan_commands: Vec<FullArtisanCommand>,
    pub artisan_make_commands: Vec<FullArtisanCommand>,
    pub quality_tools: Vec<QualityTool>,
    pub testing_tools: Vec<QualityTool>,
}

/// Check if Laravel Herd is installed (macOS or Windows)
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

    #[cfg(target_os = "windows")]
    {
        // Check for Herd executable in common locations
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let herd_exe = Path::new(&local_app_data).join("Programs/Herd/Herd.exe");
            if herd_exe.exists() {
                return true;
            }
        }

        // Check for Herd config directory
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let herd_config = Path::new(&app_data).join("Herd");
            if herd_config.exists() {
                return true;
            }
        }

        // Check if herd is in PATH
        if let Ok(output) = std::process::Command::new("where").arg("herd").output() {
            if output.status.success() {
                return true;
            }
        }

        false
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Check if Laravel Sail is present in the project
fn is_sail_project(working_dir: &Path) -> bool {
    working_dir.join("vendor/bin/sail").exists()
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

/// Helper to apply config overrides to a ProcessConfig
fn apply_overrides(
    mut config: ProcessConfig,
    kind: ProcessKind,
    laramux_config: Option<&LaramuxConfig>,
    working_dir: &Path,
) -> ProcessConfig {
    if let Some(cfg) = laramux_config {
        if let Some(override_cfg) = cfg.get_override(kind.config_name()) {
            if let Some(ref cmd) = override_cfg.command {
                config.command = cmd.clone();
            }
            if let Some(ref args) = override_cfg.args {
                config.args = args.clone();
            }
            if let Some(ref wd) = override_cfg.working_dir {
                config.working_dir = working_dir.join(wd);
            }
            if let Some(ref env) = override_cfg.env {
                config.env = env.clone();
            }
            if let Some(restart_policy) = override_cfg.restart_policy {
                config.restart_policy = restart_policy;
            }
        }
    }
    config
}

/// Discover available Laravel services in the project
pub fn discover_services(
    working_dir: &Path,
    config: Option<&LaramuxConfig>,
) -> Result<DiscoveryResult> {
    let mut configs = Vec::new();
    let mut registry = ProcessRegistry::new();

    // Detect Laravel Sail (project-level, takes precedence over Herd)
    let is_sail = match config.and_then(|c| c.sail) {
        Some(forced) => forced,
        None => is_sail_project(working_dir),
    };

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

    // Helper to check if a process is disabled
    let is_disabled = |name: &str| config.map(|c| c.is_disabled(name)).unwrap_or(false);

    // Add artisan serve (unless Sail handles it, Herd is installed, or disabled)
    // Sail manages its own web server via Docker, so serve is not needed.
    if !is_sail && !is_herd_installed() && !is_disabled("serve") {
        let serve_config = ProcessConfig::new(ProcessKind::Serve, "php", working_dir.to_path_buf())
            .with_args(vec!["artisan".to_string(), "serve".to_string()]);
        configs.push(apply_overrides(
            serve_config,
            ProcessKind::Serve,
            config,
            working_dir,
        ));
    }

    // Check for Laravel Horizon (advanced queue dashboard)
    let has_horizon = composer
        .require
        .as_ref()
        .map(|r| r.contains_key("laravel/horizon"))
        .unwrap_or(false)
        || composer
            .require_dev
            .as_ref()
            .map(|r| r.contains_key("laravel/horizon"))
            .unwrap_or(false);

    // Use Horizon if installed, otherwise fall back to basic queue:work
    if has_horizon && !is_disabled("horizon") {
        let (cmd, args) = if is_sail {
            (
                "./vendor/bin/sail",
                vec!["artisan".to_string(), "horizon".to_string()],
            )
        } else {
            ("php", vec!["artisan".to_string(), "horizon".to_string()])
        };
        let horizon_config =
            ProcessConfig::new(ProcessKind::Horizon, cmd, working_dir.to_path_buf())
                .with_args(args);
        configs.push(apply_overrides(
            horizon_config,
            ProcessKind::Horizon,
            config,
            working_dir,
        ));
    } else if !is_disabled("queue") {
        let (cmd, args) = if is_sail {
            (
                "./vendor/bin/sail",
                vec![
                    "artisan".to_string(),
                    "queue:work".to_string(),
                    "--tries=3".to_string(),
                ],
            )
        } else {
            (
                "php",
                vec![
                    "artisan".to_string(),
                    "queue:work".to_string(),
                    "--tries=3".to_string(),
                ],
            )
        };
        let queue_config =
            ProcessConfig::new(ProcessKind::Queue, cmd, working_dir.to_path_buf()).with_args(args);
        configs.push(apply_overrides(
            queue_config,
            ProcessKind::Queue,
            config,
            working_dir,
        ));
    }

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

    if has_reverb && !is_disabled("reverb") {
        let (cmd, args) = if is_sail {
            (
                "./vendor/bin/sail",
                vec!["artisan".to_string(), "reverb:start".to_string()],
            )
        } else {
            (
                "php",
                vec!["artisan".to_string(), "reverb:start".to_string()],
            )
        };
        let reverb_config =
            ProcessConfig::new(ProcessKind::Reverb, cmd, working_dir.to_path_buf()).with_args(args);
        configs.push(apply_overrides(
            reverb_config,
            ProcessKind::Reverb,
            config,
            working_dir,
        ));
    }

    // Check for Vite (package.json)
    let package_path = working_dir.join("package.json");
    if package_path.exists() && !is_disabled("vite") {
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
            let pkg_manager = detect_package_manager(working_dir);

            let (cmd, args) = if is_sail {
                let sail_args = if pkg_manager == "yarn" {
                    vec![pkg_manager.clone(), "dev".to_string()]
                } else {
                    vec![pkg_manager.clone(), "run".to_string(), "dev".to_string()]
                };
                ("./vendor/bin/sail".to_string(), sail_args)
            } else if pkg_manager == "yarn" {
                (pkg_manager, vec!["dev".to_string()])
            } else {
                (pkg_manager, vec!["run".to_string(), "dev".to_string()])
            };

            let vite_config =
                ProcessConfig::new(ProcessKind::Vite, &cmd, working_dir.to_path_buf())
                    .with_args(args);
            configs.push(apply_overrides(
                vite_config,
                ProcessKind::Vite,
                config,
                working_dir,
            ));
        }
    }

    // Add custom processes from config
    if let Some(cfg) = config {
        for custom in cfg.enabled_custom_processes() {
            let custom_working_dir = custom
                .working_dir
                .as_ref()
                .map(|wd| working_dir.join(wd))
                .unwrap_or_else(|| working_dir.to_path_buf());

            let mut custom_config = ProcessConfig::new(
                ProcessId::custom(custom.name.clone()),
                custom.command.clone(),
                custom_working_dir,
            )
            .with_args(custom.args.clone());

            if let Some(ref env) = custom.env {
                custom_config = custom_config.with_env(env.clone());
            }
            if let Some(restart_policy) = custom.restart_policy {
                custom_config = custom_config.with_restart_policy(restart_policy);
            }

            configs.push(custom_config);

            // Register custom process metadata
            registry.register_custom(
                custom.name.clone(),
                custom.display_name.clone(),
                custom.hotkey,
            );
        }
    }

    // Discover all artisan commands
    let artisan_commands = discover_all_artisan_commands(working_dir, is_sail);

    // Discover artisan make commands
    let artisan_make_commands = discover_artisan_make_commands(working_dir, is_sail);

    // Load package.json if it exists
    let package_path = working_dir.join("package.json");
    let package: Option<PackageJson> = if package_path.exists() {
        std::fs::read_to_string(&package_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    } else {
        None
    };

    // Detect package manager
    let package_manager = detect_package_manager(working_dir);

    // Discover quality and testing tools from composer.json and package.json
    let (mut quality_tools, mut testing_tools) =
        discover_dev_tools(&composer, package.as_ref(), &package_manager, is_sail);

    // Apply quality config: filter disabled tools, merge default args, add custom tools
    if let Some(cfg) = config {
        // Filter out disabled tools
        quality_tools.retain(|tool| !cfg.is_tool_disabled(&tool.display_name));
        testing_tools.retain(|tool| !cfg.is_tool_disabled(&tool.display_name));

        // Merge default args into tools
        for tool in quality_tools.iter_mut().chain(testing_tools.iter_mut()) {
            if let Some(extra_args) = cfg.tool_default_args(&tool.display_name.to_lowercase()) {
                tool.args.extend(extra_args.iter().cloned());
            }
        }

        // Add custom quality tools
        for custom_tool in cfg.custom_quality_tools() {
            quality_tools.push(QualityTool {
                display_name: custom_tool.display_name.clone(),
                command: custom_tool.command.clone(),
                args: custom_tool.args.clone(),
            });
        }

        // Add custom testing tools
        for custom_tool in cfg.custom_testing_tools() {
            testing_tools.push(QualityTool {
                display_name: custom_tool.display_name.clone(),
                command: custom_tool.command.clone(),
                args: custom_tool.args.clone(),
            });
        }
    }

    Ok(DiscoveryResult {
        is_sail,
        configs,
        registry,
        artisan_commands,
        artisan_make_commands,
        quality_tools,
        testing_tools,
    })
}

/// Detect the package manager used in the project
fn detect_package_manager(working_dir: &Path) -> String {
    let bun_lock = working_dir.join("bun.lockb");
    let pnpm_lock = working_dir.join("pnpm-lock.yaml");
    let yarn_lock = working_dir.join("yarn.lock");

    if bun_lock.exists() {
        "bun".to_string()
    } else if pnpm_lock.exists() {
        "pnpm".to_string()
    } else if yarn_lock.exists() {
        "yarn".to_string()
    } else {
        "npm".to_string()
    }
}

/// Discover development tools from composer.json and package.json
fn discover_dev_tools(
    composer: &ComposerJson,
    package: Option<&PackageJson>,
    package_manager: &str,
    is_sail: bool,
) -> (Vec<QualityTool>, Vec<QualityTool>) {
    let mut quality_tools = Vec::new();
    let mut testing_tools = Vec::new();

    // Combine require and require-dev for checking
    let all_deps: std::collections::HashSet<&str> = composer
        .require
        .as_ref()
        .map(|r| r.keys().map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(
            composer
                .require_dev
                .as_ref()
                .map(|r| r.keys().map(|s| s.as_str()).collect::<Vec<_>>())
                .unwrap_or_default(),
        )
        .collect();

    // Helper to create a PHP vendor tool, wrapping with Sail if needed
    let php_tool = |display_name: &str, bin: &str, args: Vec<String>| -> QualityTool {
        if is_sail {
            let mut sail_args = vec!["php".to_string(), bin.to_string()];
            sail_args.extend(args);
            QualityTool {
                display_name: display_name.to_string(),
                command: "./vendor/bin/sail".to_string(),
                args: sail_args,
            }
        } else {
            QualityTool {
                display_name: display_name.to_string(),
                command: bin.to_string(),
                args,
            }
        }
    };

    // PHP Quality tools
    if all_deps.contains("phpstan/phpstan") || all_deps.contains("larastan/larastan") {
        quality_tools.push(php_tool(
            "PHPStan",
            "./vendor/bin/phpstan",
            vec!["analyse".to_string(), "--ansi".to_string()],
        ));
    }

    if all_deps.contains("laravel/pint") {
        quality_tools.push(php_tool(
            "Pint",
            "./vendor/bin/pint",
            vec!["--ansi".to_string()],
        ));
    }

    if all_deps.contains("friendsofphp/php-cs-fixer") {
        quality_tools.push(php_tool(
            "PHP CS Fixer",
            "./vendor/bin/php-cs-fixer",
            vec!["fix".to_string(), "--ansi".to_string()],
        ));
    }

    if all_deps.contains("rector/rector") || all_deps.contains("rectorphp/rector") {
        quality_tools.push(php_tool(
            "Rector",
            "./vendor/bin/rector",
            vec!["--ansi".to_string()],
        ));
    }

    if all_deps.contains("squizlabs/php_codesniffer") {
        quality_tools.push(php_tool(
            "PHP_CodeSniffer",
            "./vendor/bin/phpcs",
            vec!["--colors".to_string()],
        ));
    }

    if all_deps.contains("vimeo/psalm") {
        quality_tools.push(php_tool(
            "Psalm",
            "./vendor/bin/psalm",
            vec!["--output-format=console".to_string()],
        ));
    }

    // Helper to create a JS script tool, wrapping with Sail if needed
    let js_tool = |display_name: &str, script_name: &str| -> QualityTool {
        if is_sail {
            let mut sail_args = vec![package_manager.to_string()];
            if package_manager != "yarn" {
                sail_args.push("run".to_string());
            }
            sail_args.push(script_name.to_string());
            QualityTool {
                display_name: display_name.to_string(),
                command: "./vendor/bin/sail".to_string(),
                args: sail_args,
            }
        } else {
            let args = if package_manager == "yarn" {
                vec![script_name.to_string()]
            } else {
                vec!["run".to_string(), script_name.to_string()]
            };
            QualityTool {
                display_name: display_name.to_string(),
                command: package_manager.to_string(),
                args,
            }
        }
    };

    // NPM/PNPM/Yarn/Bun scripts from package.json
    if let Some(pkg) = package {
        if let Some(scripts) = &pkg.scripts {
            // Common quality tool script names
            let quality_scripts = [
                ("lint", "Lint"),
                ("lint:fix", "Lint Fix"),
                ("format", "Format"),
                ("format:check", "Format Check"),
                ("types", "Type Check"),
                ("typecheck", "Type Check"),
                ("type-check", "Type Check"),
                ("check", "Check"),
            ];

            for (script_name, display_name) in quality_scripts {
                if scripts.contains_key(script_name) {
                    quality_tools.push(js_tool(display_name, script_name));
                }
            }

            // Testing scripts from package.json
            let test_scripts = [
                ("test", "JS Test"),
                ("test:unit", "JS Unit Test"),
                ("test:e2e", "JS E2E Test"),
                ("test:coverage", "JS Test Coverage"),
            ];

            for (script_name, display_name) in test_scripts {
                if scripts.contains_key(script_name) {
                    testing_tools.push(js_tool(display_name, script_name));
                }
            }
        }
    }

    // PHP Testing tools
    if all_deps.contains("pestphp/pest") {
        testing_tools.push(php_tool(
            "Pest",
            "./vendor/bin/pest",
            vec!["--colors=always".to_string()],
        ));
        testing_tools.push(php_tool(
            "Pest Coverage",
            "./vendor/bin/pest",
            vec!["--coverage".to_string(), "--colors=always".to_string()],
        ));
    } else if all_deps.contains("phpunit/phpunit") {
        testing_tools.push(php_tool(
            "PHPUnit",
            "./vendor/bin/phpunit",
            vec!["--colors=always".to_string()],
        ));
    }

    // Laravel's artisan test is always available
    if is_sail {
        testing_tools.push(QualityTool {
            display_name: "Artisan Test".to_string(),
            command: "./vendor/bin/sail".to_string(),
            args: vec![
                "artisan".to_string(),
                "test".to_string(),
                "--ansi".to_string(),
            ],
        });
    } else {
        testing_tools.push(QualityTool {
            display_name: "Artisan Test".to_string(),
            command: "php".to_string(),
            args: vec![
                "artisan".to_string(),
                "test".to_string(),
                "--ansi".to_string(),
            ],
        });
    }

    if all_deps.contains("brianium/paratest") {
        testing_tools.push(php_tool(
            "Paratest",
            "./vendor/bin/paratest",
            vec!["--colors".to_string()],
        ));
    }

    (quality_tools, testing_tools)
}

/// JSON structure for artisan list output
#[derive(Debug, Deserialize)]
struct ArtisanListOutput {
    commands: Vec<ArtisanListCommand>,
}

#[derive(Debug, Deserialize)]
struct ArtisanListCommand {
    name: String,
    description: String,
    #[serde(default)]
    definition: Option<ArtisanCommandDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
struct ArtisanCommandDefinition {
    #[serde(default, deserialize_with = "deserialize_map_or_empty")]
    arguments: std::collections::HashMap<String, ArtisanArgument>,
    #[serde(default, deserialize_with = "deserialize_map_or_empty")]
    options: std::collections::HashMap<String, ArtisanOption>,
}

/// Deserialize a field that can be either a map or an empty array
fn deserialize_map_or_empty<'de, D, V>(
    deserializer: D,
) -> std::result::Result<std::collections::HashMap<String, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: serde::Deserialize<'de>,
{
    use serde::de::{self, MapAccess, SeqAccess, Visitor};
    use std::collections::HashMap;
    use std::fmt;
    use std::marker::PhantomData;

    struct MapOrEmptyArrayVisitor<V>(PhantomData<V>);

    impl<'de, V> Visitor<'de> for MapOrEmptyArrayVisitor<V>
    where
        V: serde::Deserialize<'de>,
    {
        type Value = HashMap<String, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map or an empty array")
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // Check if the sequence is empty; if not, it's an error
            if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom("expected empty array or map"));
            }
            Ok(HashMap::new())
        }

        fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut result = HashMap::new();
            while let Some((key, value)) = map.next_entry()? {
                result.insert(key, value);
            }
            Ok(result)
        }
    }

    deserializer.deserialize_any(MapOrEmptyArrayVisitor(PhantomData))
}

#[derive(Debug, Deserialize, Clone)]
struct ArtisanArgument {
    name: String,
    #[serde(default)]
    is_required: bool,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ArtisanOption {
    name: String,
    #[serde(default)]
    shortcut: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    #[allow(dead_code)]
    accept_value: bool,
}

/// A full artisan command with its arguments and options
#[derive(Debug, Clone)]
pub struct FullArtisanCommand {
    /// The full command name (e.g., "optimize:clear")
    pub name: String,
    /// Description of what the command does
    pub description: String,
    /// Required and optional arguments: (name, is_required, description)
    pub arguments: Vec<(String, bool, String)>,
    /// Available options/flags: (name, shortcut, description)
    pub options: Vec<(String, String, String)>,
}

/// Discover available artisan make:* commands with full details
fn discover_artisan_make_commands(working_dir: &Path, is_sail: bool) -> Vec<FullArtisanCommand> {
    // Run php artisan list make --format=json (or via Sail)
    let output = if is_sail {
        Command::new("./vendor/bin/sail")
            .args(["artisan", "list", "make", "--format=json"])
            .current_dir(working_dir)
            .stdin(Stdio::null())
            .output()
    } else {
        Command::new("php")
            .args(["artisan", "list", "make", "--format=json"])
            .current_dir(working_dir)
            .stdin(Stdio::null())
            .output()
    };

    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "[discovery] artisan list make failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return default_make_commands();
        }
        Err(e) => {
            eprintln!("[discovery] failed to run artisan for make commands: {}", e);
            return default_make_commands();
        }
    };

    let json_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[discovery] invalid UTF-8 in artisan make output: {}", e);
            return default_make_commands();
        }
    };

    let parsed: ArtisanListOutput = match serde_json::from_str(&json_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[discovery] failed to parse artisan make JSON: {}", e);
            eprintln!("[discovery] JSON snippet: {:.500}", json_str);
            return default_make_commands();
        }
    };

    let mut commands: Vec<FullArtisanCommand> = parsed
        .commands
        .into_iter()
        .filter(|cmd| cmd.name.starts_with("make:"))
        .map(|cmd| {
            let arguments = cmd
                .definition
                .as_ref()
                .map(|def| {
                    let mut args: Vec<_> = def
                        .arguments
                        .values()
                        .filter(|arg| arg.name != "command") // Skip the command argument itself
                        .map(|arg| (arg.name.clone(), arg.is_required, arg.description.clone()))
                        .collect();
                    args.sort_by(|a, b| a.0.cmp(&b.0));
                    args
                })
                .unwrap_or_default();

            let options = cmd
                .definition
                .as_ref()
                .map(|def| {
                    let mut opts: Vec<_> = def
                        .options
                        .values()
                        .filter(|opt| {
                            // Skip common global options (name includes -- prefix)
                            !matches!(
                                opt.name.as_str(),
                                "--help"
                                    | "--quiet"
                                    | "--silent"
                                    | "--verbose"
                                    | "--version"
                                    | "--ansi"
                                    | "--no-ansi"
                                    | "--no-interaction"
                                    | "--env"
                            )
                        })
                        .map(|opt| {
                            (
                                opt.name.clone(),
                                opt.shortcut.clone(),
                                opt.description.clone(),
                            )
                        })
                        .collect();
                    opts.sort_by(|a, b| a.0.cmp(&b.0));
                    opts
                })
                .unwrap_or_default();

            FullArtisanCommand {
                name: cmd.name,
                description: cmd.description,
                arguments,
                options,
            }
        })
        .collect();

    // Sort alphabetically by name
    commands.sort_by(|a, b| a.name.cmp(&b.name));

    if commands.is_empty() {
        default_make_commands()
    } else {
        commands
    }
}

/// Discover all artisan commands (excluding make:* which are handled separately)
fn discover_all_artisan_commands(working_dir: &Path, is_sail: bool) -> Vec<FullArtisanCommand> {
    // Run php artisan list --format=json (or via Sail)
    let output = if is_sail {
        Command::new("./vendor/bin/sail")
            .args(["artisan", "list", "--format=json"])
            .current_dir(working_dir)
            .stdin(Stdio::null())
            .output()
    } else {
        Command::new("php")
            .args(["artisan", "list", "--format=json"])
            .current_dir(working_dir)
            .stdin(Stdio::null())
            .output()
    };

    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "[discovery] artisan list failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return default_artisan_commands();
        }
        Err(e) => {
            eprintln!("[discovery] failed to run artisan: {}", e);
            return default_artisan_commands();
        }
    };

    let json_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[discovery] invalid UTF-8 in artisan output: {}", e);
            return default_artisan_commands();
        }
    };

    let parsed: ArtisanListOutput = match serde_json::from_str(&json_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[discovery] failed to parse artisan JSON: {}", e);
            eprintln!("[discovery] JSON snippet: {:.500}", json_str);
            return default_artisan_commands();
        }
    };

    let mut commands: Vec<FullArtisanCommand> = parsed
        .commands
        .into_iter()
        .filter(|cmd| {
            // Exclude make:* commands (handled separately), help, list, and internal commands
            !cmd.name.starts_with("make:")
                && !cmd.name.starts_with("_")
                && cmd.name != "help"
                && cmd.name != "list"
                && cmd.name != "completion"
        })
        .map(|cmd| {
            let arguments = cmd
                .definition
                .as_ref()
                .map(|def| {
                    let mut args: Vec<_> = def
                        .arguments
                        .values()
                        .filter(|arg| arg.name != "command") // Skip the command argument itself
                        .map(|arg| (arg.name.clone(), arg.is_required, arg.description.clone()))
                        .collect();
                    args.sort_by(|a, b| a.0.cmp(&b.0));
                    args
                })
                .unwrap_or_default();

            let options = cmd
                .definition
                .as_ref()
                .map(|def| {
                    let mut opts: Vec<_> = def
                        .options
                        .values()
                        .filter(|opt| {
                            // Skip common global options (name includes -- prefix)
                            !matches!(
                                opt.name.as_str(),
                                "--help"
                                    | "--quiet"
                                    | "--silent"
                                    | "--verbose"
                                    | "--version"
                                    | "--ansi"
                                    | "--no-ansi"
                                    | "--no-interaction"
                                    | "--env"
                            )
                        })
                        .map(|opt| {
                            (
                                opt.name.clone(),
                                opt.shortcut.clone(),
                                opt.description.clone(),
                            )
                        })
                        .collect();
                    opts.sort_by(|a, b| a.0.cmp(&b.0));
                    opts
                })
                .unwrap_or_default();

            FullArtisanCommand {
                name: cmd.name,
                description: cmd.description,
                arguments,
                options,
            }
        })
        .collect();

    // Sort alphabetically by name
    commands.sort_by(|a, b| a.name.cmp(&b.name));

    if commands.is_empty() {
        default_artisan_commands()
    } else {
        commands
    }
}

/// Default artisan commands if discovery fails
fn default_artisan_commands() -> Vec<FullArtisanCommand> {
    let defaults = [
        ("about", "Display basic information about your application"),
        ("clear-compiled", "Remove the compiled class file"),
        ("completion", "Dump the shell completion script"),
        ("db", "Start a new database CLI session"),
        ("down", "Put the application into maintenance mode"),
        ("env", "Display the current framework environment"),
        ("inspire", "Display an inspiring quote"),
        ("migrate", "Run the database migrations"),
        (
            "optimize",
            "Cache framework bootstrap, configuration, and metadata",
        ),
        ("optimize:clear", "Remove the cached bootstrap files"),
        (
            "serve",
            "Serve the application on the PHP development server",
        ),
        ("tinker", "Interact with your application"),
        ("up", "Bring the application out of maintenance mode"),
        ("cache:clear", "Flush the application cache"),
        (
            "config:cache",
            "Create a cache file for faster configuration loading",
        ),
        ("config:clear", "Remove the configuration cache file"),
        ("db:seed", "Seed the database with records"),
        (
            "event:cache",
            "Discover and cache the application's events and listeners",
        ),
        ("event:clear", "Clear all cached events and listeners"),
        ("key:generate", "Set the application key"),
        ("migrate:fresh", "Drop all tables and re-run all migrations"),
        ("migrate:refresh", "Reset and re-run all migrations"),
        ("migrate:reset", "Rollback all database migrations"),
        ("migrate:rollback", "Rollback the last database migration"),
        ("migrate:status", "Show the status of each migration"),
        (
            "queue:clear",
            "Delete all of the jobs from the specified queue",
        ),
        ("queue:failed", "List all of the failed queue jobs"),
        ("queue:flush", "Flush all of the failed queue jobs"),
        (
            "queue:restart",
            "Restart queue worker daemons after their current job",
        ),
        ("queue:retry", "Retry a failed queue job"),
        (
            "route:cache",
            "Create a route cache file for faster route registration",
        ),
        ("route:clear", "Remove the route cache file"),
        ("route:list", "List all registered routes"),
        ("schedule:list", "List all scheduled tasks"),
        (
            "storage:link",
            "Create the symbolic links configured for the application",
        ),
        (
            "view:cache",
            "Compile all of the application's Blade templates",
        ),
        ("view:clear", "Clear all compiled view files"),
    ];

    defaults
        .into_iter()
        .map(|(name, desc)| FullArtisanCommand {
            name: name.to_string(),
            description: desc.to_string(),
            arguments: Vec::new(),
            options: Vec::new(),
        })
        .collect()
}

/// Default make commands if discovery fails
fn default_make_commands() -> Vec<FullArtisanCommand> {
    let defaults = [
        ("make:controller", "Create a new controller class"),
        ("make:model", "Create a new Eloquent model class"),
        ("make:migration", "Create a new migration file"),
        ("make:seeder", "Create a new seeder class"),
        ("make:factory", "Create a new model factory"),
        ("make:request", "Create a new form request class"),
        ("make:resource", "Create a new resource"),
        ("make:event", "Create a new event class"),
        ("make:listener", "Create a new event listener class"),
        ("make:job", "Create a new job class"),
        ("make:command", "Create a new Artisan command"),
        ("make:mail", "Create a new email class"),
        ("make:notification", "Create a new notification class"),
        ("make:policy", "Create a new policy class"),
        ("make:rule", "Create a new validation rule"),
    ];

    defaults
        .into_iter()
        .map(|(name, desc)| FullArtisanCommand {
            name: name.to_string(),
            description: desc.to_string(),
            arguments: vec![(
                "name".to_string(),
                true,
                "The name of the class".to_string(),
            )],
            options: Vec::new(),
        })
        .collect()
}
