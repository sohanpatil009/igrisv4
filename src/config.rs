// src/config.rs - Configuration System for IGRIS
// Handles loading, saving, and managing user settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Configuration file path
const CONFIG_PATH: &str = "./pkg/config.json";

/// Assistant personality presets
#[derive(Debug, Clone, PartialEq)]
pub enum Personality {
    Igris,
    Alita,
    Custom(String),
}

// Custom serialization to handle simple string format in config
impl Serialize for Personality {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Personality::Igris => serializer.serialize_str("Igris"),
            Personality::Alita => serializer.serialize_str("Alita"),
            Personality::Custom(name) => serializer.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for Personality {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Igris" => Ok(Personality::Igris),
            "Alita" => Ok(Personality::Alita),
            other => Ok(Personality::Custom(other.to_string())),
        }
    }
}

impl Default for Personality {
    fn default() -> Self {
        Personality::Igris
    }
}

impl Personality {
    pub fn name(&self) -> &str {
        match self {
            Personality::Igris => "IGRIS",
            Personality::Alita => "Alita",
            Personality::Custom(name) => name,
        }
    }
    
    pub fn speaker_id(&self) -> &str {
        match self {
            Personality::Igris => "051",  // Deep male voice
            Personality::Alita => "001",  // Female voice
            Personality::Custom(_) => "051",
        }
    }
    
    pub fn wake_word(&self) -> &str {
        match self {
            Personality::Igris => "hello",
            Personality::Alita => "alita",
            Personality::Custom(_) => "hello",
        }
    }
    
    /// Get all wake word variations for this personality
    pub fn wake_word_variations(&self) -> Vec<&str> {
        match self {
            Personality::Igris => vec!["hello", "hello igris", "hi igris", "igris"],
            Personality::Alita => vec!["alita", "hello alita", "hi alita", "hello"],
            Personality::Custom(_) => vec!["hello"],
        }
    }
    
    pub fn greeting(&self) -> &str {
        match self {
            Personality::Igris => "IGRIS at your service master. Say hello when you need me.",
            Personality::Alita => "Hey! Alita here. Just say hello when you need me!",
            Personality::Custom(name) => name,
        }
    }
    
    pub fn invoke_response(&self) -> &str {
        match self {
            Personality::Igris => "Yes, I'm listening. What can I do for you?",
            Personality::Alita => "Hey! What's up? How can I help?",
            Personality::Custom(_) => "I'm listening.",
        }
    }
}

/// Voice recognition settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    pub sensitivity: f32,        // 0.0 - 1.0, default 0.45
    pub max_listen_sec: u32,     // Max listening duration
    pub silence_timeout_ms: u32, // Silence before stopping
}

impl Default for RecognitionConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.45,
            max_listen_sec: 15,
            silence_timeout_ms: 600,
        }
    }
}

/// Text-to-speech settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub speed: f32,           // 0.5 - 2.0, default 1.0
    pub volume: f32,          // 0.0 - 1.0, default 0.8
    pub use_cache: bool,      // Cache audio files
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            speed: 1.0,
            volume: 0.8,
            use_cache: true,
        }
    }
}

/// Hotkey configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub modifier: String,     // "Ctrl+Shift", "Alt", etc.
    pub key: String,          // "Space", "I", etc.
    pub enabled: bool,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            modifier: "Ctrl+Shift".to_string(),
            key: "Space".to_string(),
            enabled: true,
        }
    }
}

impl HotkeyConfig {
    pub fn display(&self) -> String {
        format!("{}+{}", self.modifier, self.key)
    }
}

/// UI appearance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub show_logs: bool,
    pub show_apps: bool,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            show_logs: true,
            show_apps: true,
            window_width: 800,
            window_height: 600,
        }
    }
}

/// Theme options
#[derive(Debug, Clone, PartialEq)]
pub enum Theme {
    Dark,
    Light,
    Cyber,
}

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Theme::Dark => serializer.serialize_str("Dark"),
            Theme::Light => serializer.serialize_str("Light"),
            Theme::Cyber => serializer.serialize_str("Cyber"),
        }
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Dark" => Ok(Theme::Dark),
            "Light" => Ok(Theme::Light),
            "Cyber" => Ok(Theme::Cyber),
            _ => Ok(Theme::Dark), // Default to Dark for unknown themes
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub personality: Personality,
    pub recognition: RecognitionConfig,
    pub tts: TtsConfig,
    pub hotkey: HotkeyConfig,
    pub ui: UiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            personality: Personality::Igris,
            recognition: RecognitionConfig::default(),
            tts: TtsConfig::default(),
            hotkey: HotkeyConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Self {
        let path = PathBuf::from(CONFIG_PATH);
        
        println!("📁 Loading config from: {}", path.display());
        println!("📁 Current dir: {:?}", std::env::current_dir());
        
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    println!("📄 Config file content length: {} bytes", content.len());
                    match serde_json::from_str(&content) {
                        Ok(config) => {
                            let cfg: AppConfig = config;
                            println!("✅ Configuration loaded - Personality: {:?}", cfg.personality);
                            return cfg;
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to parse config: {}, using defaults", e);
                            eprintln!("   Content: {}", &content[..content.len().min(200)]);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to read config: {}, using defaults", e);
                }
            }
        } else {
            println!("📁 Config file not found at {}, creating default", path.display());
        }
        
        // Create default config
        let config = AppConfig::default();
        let _ = config.save();
        println!("✅ Created default config - Personality: {:?}", config.personality);
        config
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<(), String> {
        // Ensure directory exists
        if let Some(parent) = PathBuf::from(CONFIG_PATH).parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(CONFIG_PATH, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        
        println!("✅ Configuration saved to {}", CONFIG_PATH);
        Ok(())
    }
    
    /// Get assistant name based on personality
    pub fn assistant_name(&self) -> &str {
        self.personality.name()
    }
    
    /// Get speaker ID for TTS
    pub fn speaker_id(&self) -> &str {
        self.personality.speaker_id()
    }
    
    /// Get wake word
    pub fn wake_word(&self) -> &str {
        self.personality.wake_word()
    }
}


/// Global configuration instance (thread-safe)
pub struct GlobalConfig {
    config: Arc<RwLock<AppConfig>>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfig::load())),
        }
    }
    
    /// Get a clone of current config
    pub fn get(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Update configuration
    pub fn update<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().unwrap();
        f(&mut config);
        config.save()
    }
    
    /// Set personality
    pub fn set_personality(&self, personality: Personality) -> Result<(), String> {
        self.update(|c| c.personality = personality)
    }
    
    /// Set recognition sensitivity
    pub fn set_sensitivity(&self, sensitivity: f32) -> Result<(), String> {
        self.update(|c| c.recognition.sensitivity = sensitivity.clamp(0.1, 1.0))
    }
    
    /// Set TTS volume
    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.update(|c| c.tts.volume = volume.clamp(0.0, 1.0))
    }
    
    /// Set TTS speed
    pub fn set_speed(&self, speed: f32) -> Result<(), String> {
        self.update(|c| c.tts.speed = speed.clamp(0.5, 2.0))
    }
    
    /// Toggle logs visibility
    pub fn toggle_logs(&self) -> Result<(), String> {
        self.update(|c| c.ui.show_logs = !c.ui.show_logs)
    }
    
    /// Set theme
    pub fn set_theme(&self, theme: Theme) -> Result<(), String> {
        self.update(|c| c.ui.theme = theme)
    }
    
    /// Reset to defaults
    pub fn reset(&self) -> Result<(), String> {
        let mut config = self.config.write().unwrap();
        *config = AppConfig::default();
        config.save()
    }
    
    /// Get assistant name
    pub fn assistant_name(&self) -> String {
        self.config.read().unwrap().assistant_name().to_string()
    }
    
    /// Get speaker ID
    pub fn speaker_id(&self) -> String {
        self.config.read().unwrap().speaker_id().to_string()
    }
    
    /// Get wake word
    pub fn wake_word(&self) -> String {
        self.config.read().unwrap().wake_word().to_string()
    }
    
    /// Get greeting message
    pub fn greeting(&self) -> String {
        self.config.read().unwrap().personality.greeting().to_string()
    }
    
    /// Get invoke response
    pub fn invoke_response(&self) -> String {
        self.config.read().unwrap().personality.invoke_response().to_string()
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self::new()
    }
}


lazy_static::lazy_static! {
    pub static ref CONFIG: GlobalConfig = GlobalConfig::new();
}
