// src/platform/app_launcher.rs - Cross-platform app launching abstraction

use std::error::Error;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

lazy_static! {
    static ref RUNNING_PROCESSES: Mutex<HashMap<String, u32>> = Mutex::new(HashMap::new());
}

/// Trait for cross-platform app launching and management
pub trait AppLauncher: Send + Sync {
    /// Open an application by name
    fn open_app(&self, app_name: &str) -> Result<String, Box<dyn Error>>;
    
    /// Close an application by name
    fn close_app(&self, app_name: &str) -> Result<String, Box<dyn Error>>;
    
    /// List running applications
    fn list_running_apps(&self) -> Vec<String>;
    
    /// Open a folder in the system file manager
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>>;
}

/// Platform-specific implementation selector
pub struct AppLauncherImpl;

impl AppLauncherImpl {
    pub fn new() -> Box<dyn AppLauncher> {
        #[cfg(target_os = "windows")]
        {
            Box::new(WindowsAppLauncher)
        }
        
        #[cfg(target_os = "linux")]
        {
            Box::new(LinuxAppLauncher)
        }
        
        #[cfg(target_os = "macos")]
        {
            Box::new(MacOSAppLauncher)
        }
    }
}

// ============================================================================
// WINDOWS IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "windows")]
struct WindowsAppLauncher;

#[cfg(target_os = "windows")]
impl AppLauncher for WindowsAppLauncher {
    fn open_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let display_name = get_canonical_app_name(app_name);
        let (_, cmd_name) = get_windows_process_name(app_name);
        
        let commands = vec![cmd_name, display_name.clone()];
        
        for cmd in commands {
            match Command::new("cmd")
                .args(&["/C", "start", "", &cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
            {
                Ok(child) => {
                    let pid = child.id();
                    let mut procs = RUNNING_PROCESSES.lock().unwrap();
                    procs.insert(display_name.clone(), pid);
                    return Ok(format!("Opening {}", display_name));
                }
                Err(_) => continue,
            }
        }
        
        Err(format!("I couldn't find {}. Please check the app name and try again.", display_name).into())
    }
    
    fn close_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let display_name = get_canonical_app_name(app_name);
        let (_, exe_name) = get_windows_process_name(app_name);
        
        match Command::new("taskkill")
            .args(&["/IM", &exe_name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(_) => {
                let mut procs = RUNNING_PROCESSES.lock().unwrap();
                procs.remove(&display_name);
                Ok(format!("Closed {}", display_name))
            }
            Err(e) => Err(format!("I couldn't close {}. It might not be running.", display_name).into()),
        }
    }
    
    fn list_running_apps(&self) -> Vec<String> {
        let procs = RUNNING_PROCESSES.lock().unwrap();
        procs.keys().cloned().collect()
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::path::Path;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("explorer").arg(folder_path).spawn()?;
        Ok(format!("Opened folder in Explorer: {}", folder_path))
    }
}

// ============================================================================
// LINUX IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "linux")]
struct LinuxAppLauncher;

#[cfg(target_os = "linux")]
impl AppLauncher for LinuxAppLauncher {
    fn open_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        let display_name = get_canonical_app_name(app_name);
        let cmd_name = get_linux_command_name(app_name);
        
        match Command::new("sh")
            .arg("-c")
            .arg(format!("{} &", cmd_name))
            .spawn()
        {
            Ok(_) => {
                let mut procs = RUNNING_PROCESSES.lock().unwrap();
                procs.insert(display_name.clone(), 0);
                Ok(format!("Opening {}", display_name))
            }
            Err(e) => Err(format!("I couldn't find {}. Please check the app name and try again.", display_name).into()),
        }
    }
    
    fn close_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        let display_name = get_canonical_app_name(app_name);
        let cmd_name = get_linux_command_name(app_name);
        
        match Command::new("killall").arg(&cmd_name).output() {
            Ok(_) => {
                let mut procs = RUNNING_PROCESSES.lock().unwrap();
                procs.remove(&display_name);
                Ok(format!("Closed {}", display_name))
            }
            Err(e) => Err(format!("I couldn't close {}. It might not be running.", display_name).into()),
        }
    }
    
    fn list_running_apps(&self) -> Vec<String> {
        let procs = RUNNING_PROCESSES.lock().unwrap();
        procs.keys().cloned().collect()
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::path::Path;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("xdg-open").arg(folder_path).spawn()?;
        Ok(format!("Opened folder: {}", folder_path))
    }
}

// ============================================================================
// MACOS IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "macos")]
struct MacOSAppLauncher;

#[cfg(target_os = "macos")]
impl AppLauncher for MacOSAppLauncher {
    fn open_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        let display_name = get_canonical_app_name(app_name);
        let bundle_name = get_macos_bundle_name(app_name);
        
        println!("[macOS] Opening app: {} -> bundle: {}", app_name, bundle_name);
        
        // Use simple 'open' command - no permissions needed
        // -g flag brings to foreground
        // -n flag opens new instance if needed
        match Command::new("open")
            .args(&["-g", "-a", &bundle_name])
            .spawn()
        {
            Ok(_) => {
                println!("[macOS] Successfully opened {}", bundle_name);
                let mut procs = RUNNING_PROCESSES.lock().unwrap();
                procs.insert(display_name.clone(), 0);
                Ok(format!("Opening {}", display_name))
            }
            Err(e) => {
                println!("[macOS] Failed to open {}: {}", bundle_name, e);
                Err(format!("I couldn't find {}. Please check the app name and try again.", display_name).into())
            }
        }
    }
    
    fn close_app(&self, app_name: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        let display_name = get_canonical_app_name(app_name);
        let bundle_name = get_macos_bundle_name(app_name);
        
        // Use osascript to quit gracefully
        let script = format!(
            "tell application \"{}\" to quit",
            bundle_name
        );
        
        let output = Command::new("osascript")
            .args(&["-e", &script])
            .output();
        
        match output {
            Ok(out) if out.status.success() => {
                let mut procs = RUNNING_PROCESSES.lock().unwrap();
                procs.remove(&display_name);
                Ok(format!("Closed {}", display_name))
            }
            _ => {
                // Fallback to pkill with exact app name
                let output = Command::new("pkill")
                    .args(&["-x", "-i", &bundle_name])
                    .output();
                
                match output {
                    Ok(_) => {
                        let mut procs = RUNNING_PROCESSES.lock().unwrap();
                        procs.remove(&display_name);
                        Ok(format!("Closed {}", display_name))
                    }
                    Err(e) => Err(format!("I couldn't close {}. It might not be running.", display_name).into()),
                }
            }
        }
    }
    
    fn list_running_apps(&self) -> Vec<String> {
        let procs = RUNNING_PROCESSES.lock().unwrap();
        procs.keys().cloned().collect()
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::path::Path;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("open").arg(folder_path).spawn()?;
        Ok(format!("Opened folder: {}", folder_path))
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// App registry with canonical names and aliases for fuzzy matching
struct AppRegistry {
    apps: Vec<(String, Vec<&'static str>)>, // (canonical_name, [aliases])
}

impl AppRegistry {
    fn new() -> Self {
        Self {
            apps: vec![
                ("Chrome".to_string(), vec!["chrome", "google chrome", "browser"]),
                ("Firefox".to_string(), vec!["firefox", "mozilla", "browser"]),
                ("Edge".to_string(), vec!["edge", "microsoft edge", "browser"]),
                ("Notepad".to_string(), vec!["notepad", "text editor"]),
                ("Calculator".to_string(), vec!["calc", "calculator", "math"]),
                ("Paint".to_string(), vec!["paint", "drawing", "graphics"]),
                ("Word".to_string(), vec!["word", "document", "writing"]),
                ("Excel".to_string(), vec!["excel", "spreadsheet", "sheet"]),
                ("PowerPoint".to_string(), vec!["powerpoint", "presentation", "slides"]),
                ("Outlook".to_string(), vec!["outlook", "email", "mail"]),
                ("File Explorer".to_string(), vec!["explorer", "file", "files", "folder"]),
                ("Visual Studio Code".to_string(), vec!["vscode", "code", "editor", "vs code"]),
                ("Steam".to_string(), vec!["steam", "games", "gaming"]),
                ("Discord".to_string(), vec!["discord", "chat", "voice"]),
                ("Spotify".to_string(), vec!["spotify", "music", "audio"]),
                ("Terminal".to_string(), vec!["terminal", "cmd", "console"]),
                ("Safari".to_string(), vec!["safari", "browser"]),
            ],
        }
    }
    
    /// Find app by fuzzy matching with 75% threshold (score > 75 out of 100)
    fn find_app(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();
        let matcher = SkimMatcherV2::default();
        
        let mut best_match: Option<(String, i64)> = None;
        let threshold = 75i64; // 75% threshold
        
        for (canonical_name, aliases) in &self.apps {
            // Check against all aliases
            for alias in aliases {
                if let Some(score) = matcher.fuzzy_match(alias, &input_lower) {
                    // Convert to percentage (skim score is roughly 0-100 based on position)
                    // Calculate percentage based on match quality
                    let max_possible = (alias.len() * 100) as i64;
                    let percentage = if max_possible > 0 {
                        ((score * 100) / max_possible).min(100)
                    } else {
                        0
                    };
                    
                    if percentage > threshold {
                        if best_match.is_none() || score > best_match.as_ref().unwrap().1 {
                            best_match = Some((canonical_name.clone(), score));
                        }
                    }
                }
            }
        }
        
        best_match.map(|(name, _)| name)
    }
}

fn get_canonical_app_name(app_name: &str) -> String {
    let registry = AppRegistry::new();
    registry.find_app(app_name).unwrap_or_else(|| app_name.to_string())
}

#[cfg(target_os = "windows")]
fn get_windows_process_name(app_name: &str) -> (String, String) {
    let lower = app_name.to_lowercase();
    let display = get_canonical_app_name(app_name);
    
    let exe = if lower.contains("chrome") { "chrome.exe".to_string() }
    else if lower.contains("firefox") { "firefox.exe".to_string() }
    else if lower.contains("edge") { "msedge.exe".to_string() }
    else if lower.contains("brave") { "brave.exe".to_string() }
    
    // Communication
    else if lower.contains("discord") { "Discord.exe".to_string() }
    else if lower.contains("slack") { "slack.exe".to_string() }
    else if lower.contains("zoom") { "Zoom.exe".to_string() }
    else if lower.contains("skype") { "Skype.exe".to_string() }
    else if lower.contains("telegram") { "Telegram.exe".to_string() }
    
    // Editors
    else if lower.contains("vscode") || lower.contains("code") { "Code.exe".to_string() }
    else if lower.contains("sublime") { "sublime_text.exe".to_string() }
    else if lower.contains("atom") { "atom.exe".to_string() }
    else if lower.contains("notepad++") { "notepad++.exe".to_string() }
    else if lower.contains("intellij") { "idea64.exe".to_string() }
    else if lower.contains("pycharm") { "pycharm64.exe".to_string() }
    else if lower.contains("webstorm") { "webstorm64.exe".to_string() }
    
    // Media
    else if lower.contains("spotify") { "Spotify.exe".to_string() }
    else if lower.contains("vlc") { "vlc.exe".to_string() }
    
    // Gaming
    else if lower.contains("steam") { "steam.exe".to_string() }
    else if lower.contains("epic") { "EpicGamesLauncher.exe".to_string() }
    else if lower.contains("origin") { "Origin.exe".to_string() }
    else if lower.contains("ubisoft") { "UbisoftConnect.exe".to_string() }
    else if lower.contains("gog") { "GalaxyClient.exe".to_string() }
    
    // Creative
    else if lower.contains("photoshop") { "Photoshop.exe".to_string() }
    else if lower.contains("illustrator") { "Illustrator.exe".to_string() }
    else if lower.contains("premiere") { "Adobe Premiere Pro.exe".to_string() }
    else if lower.contains("aftereffects") { "AfterFX.exe".to_string() }
    else if lower.contains("gimp") { "gimp.exe".to_string() }
    else if lower.contains("blender") { "blender.exe".to_string() }
    else if lower.contains("inkscape") { "inkscape.exe".to_string() }
    
    // Office
    else if lower.contains("word") { "WINWORD.EXE".to_string() }
    else if lower.contains("excel") { "EXCEL.EXE".to_string() }
    else if lower.contains("powerpoint") { "POWERPNT.EXE".to_string() }
    else if lower.contains("outlook") { "OUTLOOK.EXE".to_string() }
    else if lower.contains("onenote") { "ONENOTE.EXE".to_string() }
    else if lower.contains("teams") { "Teams.exe".to_string() }
    
    // Utilities
    else if lower.contains("notepad") { "notepad.exe".to_string() }
    else if lower.contains("calc") { "calc.exe".to_string() }
    else if lower.contains("paint") { "mspaint.exe".to_string() }
    else if lower.contains("explorer") || lower.contains("file") { "explorer.exe".to_string() }
    
    else { format!("{}.exe", app_name) };
    
    (display, exe)
}

#[cfg(target_os = "linux")]
fn get_linux_command_name(app_name: &str) -> String {
    let lower = app_name.to_lowercase();
    
    // Browsers
    if lower.contains("chrome") { "google-chrome" }
    else if lower.contains("firefox") { "firefox" }
    else if lower.contains("edge") { "microsoft-edge" }
    else if lower.contains("brave") { "brave-browser" }
    
    // Communication
    else if lower.contains("discord") { "discord" }
    else if lower.contains("slack") { "slack" }
    else if lower.contains("zoom") { "zoom" }
    else if lower.contains("skype") { "skypeforlinux" }
    else if lower.contains("telegram") { "telegram-desktop" }
    
    // Editors
    else if lower.contains("vscode") || lower.contains("code") { "code" }
    else if lower.contains("sublime") { "subl" }
    else if lower.contains("atom") { "atom" }
    else if lower.contains("intellij") { "idea" }
    else if lower.contains("pycharm") { "pycharm" }
    else if lower.contains("webstorm") { "webstorm" }
    
    // Media
    else if lower.contains("spotify") { "spotify" }
    else if lower.contains("vlc") { "vlc" }
    
    // Gaming
    else if lower.contains("steam") { "steam" }
    
    // Creative
    else if lower.contains("gimp") { "gimp" }
    else if lower.contains("blender") { "blender" }
    else if lower.contains("inkscape") { "inkscape" }
    
    // Utilities
    else if lower.contains("terminal") { "gnome-terminal" }
    else if lower.contains("calculator") { "gnome-calculator" }
    
    else { app_name }
    .to_string()
}

#[cfg(target_os = "macos")]
fn get_macos_bundle_name(app_name: &str) -> String {
    let lower = app_name.to_lowercase();
    
    // Browsers
    if lower.contains("chrome") { "Google Chrome" }
    else if lower.contains("firefox") { "Firefox" }
    else if lower.contains("edge") { "Microsoft Edge" }
    else if lower.contains("brave") { "Brave Browser" }
    else if lower.contains("safari") { "Safari" }
    
    // Communication
    else if lower.contains("discord") { "Discord" }
    else if lower.contains("slack") { "Slack" }
    else if lower.contains("zoom") { "zoom.us" }
    else if lower.contains("skype") { "Skype" }
    else if lower.contains("telegram") { "Telegram" }
    
    // Editors
    else if lower.contains("vscode") || lower.contains("code") { "Visual Studio Code" }
    else if lower.contains("sublime") { "Sublime Text" }
    else if lower.contains("atom") { "Atom" }
    else if lower.contains("intellij") { "IntelliJ IDEA" }
    else if lower.contains("pycharm") { "PyCharm" }
    else if lower.contains("webstorm") { "WebStorm" }
    
    // Media
    else if lower.contains("spotify") { "Spotify" }
    else if lower.contains("vlc") { "VLC" }
    
    // Gaming
    else if lower.contains("steam") { "Steam" }
    else if lower.contains("epic") { "Epic Games Launcher" }
    else if lower.contains("origin") { "Origin" }
    else if lower.contains("ubisoft") { "Ubisoft Connect" }
    else if lower.contains("gog") { "GOG Galaxy" }
    
    // Creative
    else if lower.contains("photoshop") { "Adobe Photoshop 2024" }
    else if lower.contains("illustrator") { "Adobe Illustrator 2024" }
    else if lower.contains("premiere") { "Adobe Premiere Pro 2024" }
    else if lower.contains("aftereffects") { "Adobe After Effects 2024" }
    else if lower.contains("gimp") { "GIMP" }
    else if lower.contains("blender") { "Blender" }
    else if lower.contains("inkscape") { "Inkscape" }
    
    // Office
    else if lower.contains("word") { "Microsoft Word" }
    else if lower.contains("excel") { "Microsoft Excel" }
    else if lower.contains("powerpoint") { "Microsoft PowerPoint" }
    else if lower.contains("outlook") { "Microsoft Outlook" }
    else if lower.contains("onenote") { "Microsoft OneNote" }
    else if lower.contains("teams") { "Microsoft Teams" }
    
    // Utilities
    else if lower.contains("calculator") { "Calculator" }
    else if lower.contains("paint") { "Preview" } // macOS doesn't have Paint, use Preview
    else if lower.contains("terminal") { "Terminal" }
    else if lower.contains("notepad") { "TextEdit" } // macOS equivalent
    else if lower.contains("explorer") { "Finder" }
    
    else { app_name }
    .to_string()
}
