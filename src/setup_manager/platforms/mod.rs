// src/setup_manager/platforms/mod.rs
// Platform-specific setup implementations

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use crate::setup_manager::SetupEvent;

/// Trait for platform-specific setup operations
pub trait PlatformSetup: Send + Sync {
    /// Get the package directory for this platform
    fn pkg_dir(&self) -> &PathBuf;
    
    /// Check if setup is already complete
    fn is_setup_complete(&self) -> bool;
    
    /// Create required directories
    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Install FFmpeg (download/extract on Windows, brew/apt on Mac/Linux)
    fn install_ffmpeg(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Install Piper TTS
    fn install_piper(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Install SenseVoice STT model
    fn install_whisper_model(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Install SBERT model
    fn install_sbert_model(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Install voice model for TTS
    fn install_voice_model(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Install LLVM/Clang (for sherpa-onnx / build dependencies)
    fn install_llvm(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
    
    /// Setup PATH environment variables
    fn setup_path(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Validate the installation
    fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Run full setup
    fn run_setup(&self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// Get the platform-specific setup implementation
#[cfg(target_os = "windows")]
pub fn get_platform_setup(
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
) -> impl PlatformSetup {
    windows::WindowsSetup::new(pkg_dir, event_sender)
}

#[cfg(target_os = "macos")]
pub fn get_platform_setup(
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
) -> impl PlatformSetup {
    macos::MacOSSetup::new(pkg_dir, event_sender)
}

#[cfg(target_os = "linux")]
pub fn get_platform_setup(
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
) -> impl PlatformSetup {
    linux::LinuxSetup::new(pkg_dir, event_sender)
}

/// Detect current platform
pub fn current_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";
    
    #[cfg(target_os = "macos")]
    return "macos";
    
    #[cfg(target_os = "linux")]
    return "linux";
}

/// Check if a command exists in PATH
pub fn command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("where")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Run a shell command and return output
pub fn run_command(cmd: &str, args: &[&str]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "Command failed: {} {:?}\n{}",
            cmd,
            args,
            String::from_utf8_lossy(&output.stderr)
        ).into())
    }
}
