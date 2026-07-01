// src/plugin_system.rs
// Plugin system for extensible voice commands

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::builtin;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub enabled: bool,
}

/// Plugin command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub trigger: String,
    pub description: String,
    pub examples: Vec<String>,
    pub action_type: ActionType,
    pub action_data: String,
}

/// Types of actions a plugin can perform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Execute a shell command
    ShellCommand,
    /// Open a URL
    OpenUrl,
    /// Run a script file
    RunScript,
    /// Send HTTP request
    HttpRequest,
    /// Custom Rust function (loaded from dynamic library)
    CustomFunction,
    /// Camera mode operations
    CameraMode,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::ShellCommand => write!(f, "ShellCommand"),
            ActionType::OpenUrl => write!(f, "OpenUrl"),
            ActionType::RunScript => write!(f, "RunScript"),
            ActionType::HttpRequest => write!(f, "HttpRequest"),
            ActionType::CustomFunction => write!(f, "CustomFunction"),
            ActionType::CameraMode => write!(f, "CameraMode"),
        }
    }
}

/// Plugin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub metadata: PluginMetadata,
    pub commands: Vec<PluginCommand>,
}

impl Plugin {
    /// Load plugin from JSON file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let plugin: Plugin = serde_json::from_str(&content)?;
        Ok(plugin)
    }

    /// Save plugin to JSON file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Check if plugin matches a command
    pub fn matches_command(&self, input: &str) -> Option<&PluginCommand> {
        if !self.metadata.enabled {
            return None;
        }

        // Clean input - remove punctuation and extra spaces
        let input_clean: String = input.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        let input_lower = input_clean.to_lowercase().trim().to_string();
        
        // First pass: look for exact trigger matches only
        for command in &self.commands {
            let trigger_lower = command.trigger.to_lowercase();
            
            // Exact match
            if input_lower == trigger_lower {
                return Some(command);
            }
        }
        
        // Second pass: look for exact example matches
        for command in &self.commands {
            for example in &command.examples {
                let example_clean: String = example.chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                    .collect();
                if input_lower == example_clean.to_lowercase().trim() {
                    return Some(command);
                }
            }
        }
        
        // Third pass: look for contains match in triggers
        for command in &self.commands {
            let trigger_lower = command.trigger.to_lowercase();
            if input_lower.contains(&trigger_lower) {
                return Some(command);
            }
        }
        
        // Fourth pass: look for contains match in examples
        for command in &self.commands {
            for example in &command.examples {
                let example_clean: String = example.chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                    .collect();
                let example_lower = example_clean.to_lowercase();
                if input_lower.contains(&example_lower) || example_lower.contains(&input_lower) {
                    return Some(command);
                }
            }
        }
        
        // Fifth pass: fuzzy match - check if key words match
        for command in &self.commands {
            let trigger_lower = command.trigger.to_lowercase();
            let trigger_words: Vec<&str> = trigger_lower.split_whitespace().collect();
            let input_words: Vec<&str> = input_lower.split_whitespace().collect();
            
            // Check if most trigger words are in input
            let matching_words = trigger_words.iter()
                .filter(|tw| input_words.iter().any(|iw| iw.contains(*tw) || tw.contains(iw)))
                .count();
            
            if matching_words >= trigger_words.len().saturating_sub(1) && matching_words > 0 {
                return Some(command);
            }
        }
        
        None
    }
}

/// Plugin manager
pub struct PluginManager {
    plugins: HashMap<String, Plugin>,
    plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins: HashMap::new(),
            plugins_dir,
        }
    }

    /// Initialize plugin manager and load all plugins
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load built-in Rust plugins first (fastest - no parsing)
        for plugin in builtin::get_builtin_plugins() {
            println!("[PLUGIN] Built-in: {} v{}", plugin.metadata.name, plugin.metadata.version);
            self.plugins.insert(plugin.metadata.name.clone(), plugin);
        }
        
        // Load custom JSON plugins (user-defined, can override built-in)
        // Only load if plugins directory already exists
        self.load_all_plugins()?;
        
        Ok(())
    }

    /// Load all plugins from the plugins directory
    fn load_all_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.plugins_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match Plugin::load_from_file(&path) {
                    Ok(plugin) => {
                        println!("[PLUGIN] Loaded: {} v{}", plugin.metadata.name, plugin.metadata.version);
                        self.plugins.insert(plugin.metadata.name.clone(), plugin);
                    }
                    Err(e) => {
                        eprintln!("[PLUGIN] Failed to load {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Create an example plugin
    fn create_example_plugin(&self) -> Result<(), Box<dyn std::error::Error>> {
        let example_plugin = Plugin {
            metadata: PluginMetadata {
                name: "example_plugin".to_string(),
                version: "1.0.0".to_string(),
                author: "IGRIS".to_string(),
                description: "Example plugin demonstrating custom commands".to_string(),
                keywords: vec!["example".to_string(), "demo".to_string()],
                enabled: true,
            },
            commands: vec![
                PluginCommand {
                    trigger: "open youtube".to_string(),
                    description: "Opens YouTube in browser".to_string(),
                    examples: vec!["open youtube".to_string()],
                    action_type: ActionType::OpenUrl,
                    action_data: "https://www.youtube.com".to_string(),
                },
                PluginCommand {
                    trigger: "check weather".to_string(),
                    description: "Opens weather website".to_string(),
                    examples: vec!["check weather".to_string(), "show weather".to_string()],
                    action_type: ActionType::OpenUrl,
                    action_data: "https://www.weather.com".to_string(),
                },
                PluginCommand {
                    trigger: "open github".to_string(),
                    description: "Opens GitHub in browser".to_string(),
                    examples: vec!["open github".to_string()],
                    action_type: ActionType::OpenUrl,
                    action_data: "https://www.github.com".to_string(),
                },
            ],
        };

        let path = self.plugins_dir.join("example_plugin.json");
        example_plugin.save_to_file(&path)?;
        
        Ok(())
    }

    /// Process a command through plugins (OPTIMIZED)
    /// Uses multi-pass matching: exact trigger -> exact example -> contains -> fuzzy
    /// Pre-computes lowercase values for faster matching
    pub fn process_command(&self, input: &str) -> Option<PluginCommandResult> {
        // Clean and lowercase input ONCE
        let input_lower: String = input.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase();
        let input_lower = input_lower.trim();
        
        if input_lower.is_empty() {
            return None;
        }
        
        let input_words: Vec<&str> = input_lower.split_whitespace().collect();
        
        // Collect all enabled plugins with pre-computed lowercase triggers
        let enabled_plugins: Vec<_> = self.plugins.values()
            .filter(|p| p.metadata.enabled)
            .collect();
        
        // Pass 1: Exact trigger match (fastest)
        for plugin in &enabled_plugins {
            for command in &plugin.commands {
                let trigger_lower = command.trigger.to_lowercase();
                if input_lower == trigger_lower {
                    return Some(PluginCommandResult {
                        plugin_name: plugin.metadata.name.clone(),
                        command: command.clone(),
                    });
                }
            }
        }
        
        // Pass 2: Exact example match
        for plugin in &enabled_plugins {
            for command in &plugin.commands {
                for example in &command.examples {
                    let example_lower: String = example.chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                        .collect::<String>()
                        .to_lowercase();
                    if input_lower == example_lower.trim() {
                        return Some(PluginCommandResult {
                            plugin_name: plugin.metadata.name.clone(),
                            command: command.clone(),
                        });
                    }
                }
            }
        }
        
        // Pass 3: Contains trigger match
        for plugin in &enabled_plugins {
            for command in &plugin.commands {
                let trigger_lower = command.trigger.to_lowercase();
                if input_lower.contains(&trigger_lower) {
                    return Some(PluginCommandResult {
                        plugin_name: plugin.metadata.name.clone(),
                        command: command.clone(),
                    });
                }
            }
        }
        
        // Pass 4: Contains example match
        for plugin in &enabled_plugins {
            for command in &plugin.commands {
                for example in &command.examples {
                    let example_lower: String = example.chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                        .collect::<String>()
                        .to_lowercase();
                    let example_trimmed = example_lower.trim();
                    if input_lower.contains(example_trimmed) || example_trimmed.contains(input_lower) {
                        return Some(PluginCommandResult {
                            plugin_name: plugin.metadata.name.clone(),
                            command: command.clone(),
                        });
                    }
                }
            }
        }
        
        // Pass 5: Fuzzy match - ALL trigger words must match (word-boundary, min length 2)
        for plugin in &enabled_plugins {
            for command in &plugin.commands {
                let trigger_lower = command.trigger.to_lowercase();
                let trigger_words: Vec<&str> = trigger_lower.split_whitespace()
                    .filter(|w| w.len() >= 2)
                    .collect();
                
                if trigger_words.is_empty() {
                    continue;
                }
                
                let all_match = trigger_words.iter().all(|tw| {
                    input_words.iter().any(|iw| iw.len() >= 2 && iw == tw)
                });
                
                if all_match {
                    return Some(PluginCommandResult {
                        plugin_name: plugin.metadata.name.clone(),
                        command: command.clone(),
                    });
                }
            }
        }
        
        None
    }

    /// Execute a plugin command
    pub fn execute_command(&self, result: &PluginCommandResult) -> Result<String, Box<dyn std::error::Error>> {
        match result.command.action_type {
            ActionType::ShellCommand => {
                self.execute_shell_command(&result.command.action_data)
            }
            ActionType::OpenUrl => {
                self.open_url(&result.command.action_data)
            }
            ActionType::RunScript => {
                self.run_script(&result.command.action_data)
            }
            ActionType::HttpRequest => {
                self.send_http_request(&result.command.action_data)
            }
            ActionType::CustomFunction => {
                // Custom functions are handled specially in main.rs
                // Return a marker that main.rs will recognize
                Ok(format!("CUSTOM_FN:{}", result.command.action_data))
            }
            ActionType::CameraMode => {
                // Camera mode is handled specially in main.rs
                // Return a special marker that main.rs will recognize
                Ok(format!("CAMERA_MODE:{}", result.command.action_data))
            }
        }
    }

    /// Execute a shell command with process tracking
    fn execute_shell_command(&self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        use crate::utils::process_tracker::{track_process, ProcessCategory};
        
        let cmd_lower = command.to_lowercase();
        
        // Determine if this is an app launch (start command)
        let is_app_launch = cmd_lower.starts_with("start ");
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            
            if is_app_launch {
                // Extract app name and exe name for tracking
                let (app_name, exe_name) = self.extract_app_info(command);
                
                // Track the process BEFORE launching (by name, not PID)
                track_process(&app_name, &exe_name, ProcessCategory::App);
                
                // Launch the app
                let _ = std::process::Command::new("cmd")
                    .args(["/C", command])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .spawn()?;
                
                return Ok(format!("Opening {}...", app_name));
            } else {
                // For other commands, just execute
                let output = std::process::Command::new("cmd")
                    .args(["/C", command])
                    .creation_flags(0x08000000)
                    .output()?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        return Err(format!("Command failed: {}", stderr).into());
                    }
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if is_app_launch {
                let (app_name, exe_name) = self.extract_app_info(command);
                track_process(&app_name, &exe_name, ProcessCategory::App);
                
                let _ = std::process::Command::new("sh")
                    .args(["-c", command])
                    .spawn()?;
                
                return Ok(format!("Opening {}...", app_name));
            } else {
                let output = std::process::Command::new("sh")
                    .args(["-c", command])
                    .output()?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        return Err(format!("Command failed: {}", stderr).into());
                    }
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            if is_app_launch || cmd_lower.contains("open ") {
                let (app_name, exe_name) = self.extract_app_info(command);
                track_process(&app_name, &exe_name, ProcessCategory::App);
                
                let _ = std::process::Command::new("sh")
                    .args(["-c", command])
                    .spawn()?;
                
                return Ok(format!("Opening {}...", app_name));
            } else {
                let output = std::process::Command::new("sh")
                    .args(["-c", command])
                    .output()?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        return Err(format!("Command failed: {}", stderr).into());
                    }
                }
            }
        }
        
        Ok(self.get_friendly_message(command))
    }
    
    /// Extract app name and exe name from command
    fn extract_app_info(&self, command: &str) -> (String, String) {
        let cmd_lower = command.to_lowercase();
        
        // Windows: start "app" or start app.exe
        if cmd_lower.starts_with("start ") {
            let rest = command[6..].trim();
            // Remove quotes
            let clean = rest.trim_matches('"').trim_matches('\'');
            
            // Get exe name (with .exe)
            let exe_name = if clean.ends_with(".exe") {
                clean.to_string()
            } else {
                format!("{}.exe", clean)
            };
            
            // Get friendly name (without .exe and path)
            let app_name = clean
                .trim_end_matches(".exe")
                .split('/').last()
                .unwrap_or(clean)
                .split('\\').last()
                .unwrap_or(clean)
                .to_string();
            
            return (app_name, exe_name);
        }
        
        // macOS: open -a "App"
        if cmd_lower.contains("open -a") {
            if let Some(app) = command.split("-a").nth(1) {
                let app_name = app.trim().trim_matches('"').to_string();
                return (app_name.clone(), app_name);
            }
        }
        
        ("app".to_string(), "app.exe".to_string())
    }

    /// Generate a user-friendly message for shell commands
    fn get_friendly_message(&self, command: &str) -> String {
        let cmd_lower = command.to_lowercase();
        
        // App closing commands
        if cmd_lower.contains("taskkill") {
            if let Some(app_name) = cmd_lower.split("/im").nth(1) {
                let app = app_name.trim().split(".exe").next().unwrap_or("app").trim();
                return format!("Closing {}...", app);
            }
            return "Closing application...".to_string();
        }
        
        // App opening commands (start command)
        if cmd_lower.starts_with("start ") {
            let app = command[6..].trim().trim_matches('"');
            return format!("Opening {}...", app);
        }
        
        // Default
        "Done.".to_string()
    }

    /// Open a URL in the default browser, tracking the site → browser mapping
    fn open_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        use crate::utils::process_tracker::{track_site, extract_site_name};

        let site_name = extract_site_name(url);

        // Detect which browser is the default
        let (browser_exe, browser_name) = self.detect_default_browser();

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", url])
                .spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()?;
        }
        
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()?;
        }

        // Track the site → browser mapping so "close youtube" works
        track_site(url, &browser_exe, &browser_name);

        Ok(format!("Opened {} in {}", site_name, browser_name))
    }

    /// Detect the default browser for site tracking purposes.
    /// Returns (browser_exe, browser_display_name).
    /// The exe name must match what `pkill -f` or `taskkill /IM` expects.
    fn detect_default_browser(&self) -> (String, String) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", "com.apple.LaunchServices/com.apple.launchservices.secure", "LSHandlers"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Google Chrome") || stdout.contains("chrome") {
                    return ("Google Chrome".to_string(), "Google Chrome".to_string());
                }
                if stdout.contains("Firefox") || stdout.contains("firefox") {
                    return ("firefox".to_string(), "Firefox".to_string());
                }
                if stdout.contains("Safari") || stdout.contains("safari") {
                    return ("Safari".to_string(), "Safari".to_string());
                }
                if stdout.contains("Brave") || stdout.contains("brave") {
                    return ("Brave Browser".to_string(), "Brave Browser".to_string());
                }
                if stdout.contains("Edge") || stdout.contains("edge") {
                    return ("Microsoft Edge".to_string(), "Microsoft Edge".to_string());
                }
                if stdout.contains("Arc") || stdout.contains("arc") {
                    return ("Arc".to_string(), "Arc".to_string());
                }
                if stdout.contains("Opera") || stdout.contains("opera") {
                    return ("Opera".to_string(), "Opera".to_string());
                }
                if stdout.contains("Vivaldi") || stdout.contains("vivaldi") {
                    return ("Vivaldi".to_string(), "Vivaldi".to_string());
                }
            }
            ("Safari".to_string(), "Safari".to_string())
        }
        #[cfg(target_os = "windows")]
        {
            // Try to read the default browser from the registry
            let candidates = [
                ("chrome.exe", "Google Chrome"),
                ("firefox.exe", "Firefox"),
                ("msedge.exe", "Microsoft Edge"),
                ("brave.exe", "Brave Browser"),
                ("opera.exe", "Opera"),
                ("vivaldi.exe", "Vivaldi"),
            ];
            // Check which browsers are installed by looking for their executables
            for (exe, name) in &candidates {
                if std::path::Path::new(&format!("C:\\Program Files\\{}\\{}", name, exe)).exists()
                    || std::path::Path::new(&format!("C:\\Program Files (x86)\\{}\\{}", name, exe)).exists()
                {
                    return (exe.to_string(), name.to_string());
                }
            }
            ("chrome.exe".to_string(), "Google Chrome".to_string())
        }
        #[cfg(target_os = "linux")]
        {
            // Try xdg-settings first (most reliable on desktop Linux)
            if let Ok(output) = std::process::Command::new("xdg-settings")
                .args(["get", "default-web-browser"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
                if stdout.contains("google-chrome") || stdout.contains("chrome") {
                    return ("google-chrome".to_string(), "Google Chrome".to_string());
                }
                if stdout.contains("firefox") {
                    return ("firefox".to_string(), "Firefox".to_string());
                }
                if stdout.contains("brave") {
                    return ("brave-browser".to_string(), "Brave Browser".to_string());
                }
                if stdout.contains("microsoft-edge") || stdout.contains("msedge") {
                    return ("microsoft-edge".to_string(), "Microsoft Edge".to_string());
                }
                if stdout.contains("opera") {
                    return ("opera".to_string(), "Opera".to_string());
                }
                if stdout.contains("vivaldi") {
                    return ("vivaldi".to_string(), "Vivaldi".to_string());
                }
            }
            // Fallback: check common browser binaries
            for (exe, name) in &[
                ("google-chrome", "Google Chrome"),
                ("firefox", "Firefox"),
                ("brave-browser", "Brave Browser"),
                ("microsoft-edge", "Microsoft Edge"),
                ("opera", "Opera"),
                ("vivaldi", "Vivaldi"),
            ] {
                if std::process::Command::new("which").arg(exe).output().map_or(false, |o| o.status.success()) {
                    return (exe.to_string(), name.to_string());
                }
            }
            ("google-chrome".to_string(), "Google Chrome".to_string())
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            ("browser".to_string(), "Browser".to_string())
        }
    }

    /// Run a script file
    fn run_script(&self, script_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = PathBuf::from(script_path);
        
        if !path.exists() {
            return Err(format!("Script not found: {}", script_path).into());
        }
        
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", script_path])
                .spawn()?;
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("sh")
                .arg(script_path)
                .spawn()?;
        }
        
        Ok(format!("Running script: {}", script_path))
    }

    /// Send an HTTP request
    fn send_http_request(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        fn client() -> &'static reqwest::blocking::Client {
            static CLIENT: std::sync::OnceLock<&reqwest::blocking::Client> = std::sync::OnceLock::new();
            *CLIENT.get_or_init(|| {
                Box::leak(Box::new(
                    reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .expect("Failed to create HTTP client")
                ))
            })
        }
        let response = client().get(url).send()?;
        let status = response.status();
        
        Ok(format!("HTTP request to {} returned: {}", url, status))
    }

    /// Get all loaded plugins
    pub fn get_plugins(&self) -> Vec<Plugin> {
        self.plugins.values().cloned().collect()
    }

    /// Enable a plugin
    pub fn enable_plugin(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.metadata.enabled = true;
            
            // Save to file
            let path = self.plugins_dir.join(format!("{}.json", name));
            plugin.save_to_file(&path)?;
            
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", name).into())
        }
    }

    /// Disable a plugin
    pub fn disable_plugin(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.metadata.enabled = false;
            
            // Save to file
            let path = self.plugins_dir.join(format!("{}.json", name));
            plugin.save_to_file(&path)?;
            
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", name).into())
        }
    }

    /// Reload all plugins
    pub fn reload_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.plugins.clear();
        self.load_all_plugins()?;
        Ok(())
    }
}

/// Result of processing a command through plugins
#[derive(Debug, Clone)]
pub struct PluginCommandResult {
    pub plugin_name: String,
    pub command: PluginCommand,
}

lazy_static::lazy_static! {
    /// Global plugin manager instance
    pub static ref GLOBAL_PLUGIN_MANAGER: Arc<Mutex<PluginManager>> = {
        let plugins_dir = PathBuf::from("./plugins");
        let mut manager = PluginManager::new(plugins_dir);
        
        if let Err(e) = manager.initialize() {
            eprintln!("[PLUGIN] Initialization error: {}", e);
        }
        
        Arc::new(Mutex::new(manager))
    };
}

/// Process command through global plugin manager
pub fn process_plugin_command(input: &str) -> Option<PluginCommandResult> {
    GLOBAL_PLUGIN_MANAGER.lock().unwrap().process_command(input)
}

/// Execute plugin command through global manager
pub fn execute_plugin_command(result: &PluginCommandResult) -> Result<String, Box<dyn std::error::Error>> {
    GLOBAL_PLUGIN_MANAGER.lock().unwrap().execute_command(result)
}

/// Get all loaded plugins
pub fn get_all_plugins() -> Vec<Plugin> {
    GLOBAL_PLUGIN_MANAGER.lock().unwrap().get_plugins()
}

/// Reload all plugins
pub fn reload_all_plugins() -> Result<(), Box<dyn std::error::Error>> {
    GLOBAL_PLUGIN_MANAGER.lock().unwrap().reload_plugins()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = Plugin {
            metadata: PluginMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                description: "Test plugin".to_string(),
                keywords: vec!["test".to_string()],
                enabled: true,
            },
            commands: vec![],
        };
        
        assert_eq!(plugin.metadata.name, "test");
    }

    #[test]
    fn test_command_matching() {
        let plugin = Plugin {
            metadata: PluginMetadata {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                description: "Test plugin".to_string(),
                keywords: vec!["youtube".to_string()],
                enabled: true,
            },
            commands: vec![
                PluginCommand {
                    trigger: "open youtube".to_string(),
                    description: "Opens YouTube".to_string(),
                    examples: vec![],
                    action_type: ActionType::OpenUrl,
                    action_data: "https://youtube.com".to_string(),
                },
            ],
        };
        
        assert!(plugin.matches_command("open youtube").is_some());
        assert!(plugin.matches_command("youtube").is_some());
        assert!(plugin.matches_command("random command").is_none());
    }
}
