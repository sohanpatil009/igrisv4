#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

use crate::eco::errors::EcoResult;

pub trait PlatformClipboard: Send + Sync {
    fn get_text(&self) -> EcoResult<String>;
    fn set_text(&self, text: &str) -> EcoResult<()>;
}

pub fn create_platform_clipboard() -> Box<dyn PlatformClipboard> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacosClipboard) }
    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxClipboard) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsClipboard) }
}
