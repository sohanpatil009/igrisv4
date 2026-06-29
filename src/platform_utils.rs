// src/platform_utils.rs - Convenient wrappers for platform abstractions

use crate::platform::{AppLauncherImpl, FileSystemProviderImpl};
use std::sync::OnceLock;

static APP_LAUNCHER: OnceLock<Box<dyn crate::platform::AppLauncher>> = OnceLock::new();
static FILE_SYSTEM: OnceLock<Box<dyn crate::platform::FileSystemProvider>> = OnceLock::new();

/// Get the platform-specific app launcher
pub fn get_app_launcher() -> &'static dyn crate::platform::AppLauncher {
    APP_LAUNCHER
        .get_or_init(|| AppLauncherImpl::new())
        .as_ref()
}

/// Get the platform-specific file system provider
pub fn get_file_system() -> &'static dyn crate::platform::FileSystemProvider {
    FILE_SYSTEM
        .get_or_init(|| FileSystemProviderImpl::new())
        .as_ref()
}

// Re-export ProcessBuilderExt for convenience
pub use crate::platform::process_builder::ProcessBuilderExt;

/// Get the device's computer name
pub fn get_device_name() -> String {
    #[cfg(target_os = "windows")]
    {
        // Try to get computer name from environment variable
        if let Ok(name) = std::env::var("COMPUTERNAME") {
            return name;
        }
        
        // Fallback: use hostname command
        if let Ok(output) = std::process::Command::new("hostname").output() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // Try to get computer name using scutil
        if let Ok(output) = std::process::Command::new("scutil")
            .arg("--get")
            .arg("ComputerName")
            .output()
        {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        
        // Fallback: use hostname
        if let Ok(output) = std::process::Command::new("hostname").output() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim().trim_end_matches(".local");
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Try to get hostname
        if let Ok(output) = std::process::Command::new("hostname").output() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        
        // Fallback: read from /etc/hostname
        if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    
    // Ultimate fallback
    "IGRIS".to_string()
}
