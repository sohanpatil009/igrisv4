#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

use crate::eco::errors::EcoResult;

/// Raw notification from the platform notification center.
#[derive(Clone, Debug)]
pub struct RawNotification {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub timestamp: i64,
}

/// Platform-specific notification access.
pub trait PlatformNotification: Send + Sync {
    /// Read current notifications from the system notification center.
    fn read_notifications(&self) -> EcoResult<Vec<RawNotification>>;

    /// Reply to a notification through its source app.
    fn reply_to_notification(&self, notification_id: &str, reply: &str) -> EcoResult<()>;

    /// Check if we have permission to read notifications.
    fn has_permission(&self) -> bool;

    /// Request permission from the user (platform-specific dialog).
    fn request_permission(&self) -> EcoResult<()>;
}

pub fn create_platform_notification() -> Box<dyn PlatformNotification> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacosNotification) }
    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxNotification) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsNotification) }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    { Box::new(DummyNotification) }
}

/// Fallback for unsupported platforms.
struct DummyNotification;

impl PlatformNotification for DummyNotification {
    fn read_notifications(&self) -> EcoResult<Vec<RawNotification>> {
        Ok(Vec::new())
    }
    fn reply_to_notification(&self, _id: &str, _reply: &str) -> EcoResult<()> {
        Ok(())
    }
    fn has_permission(&self) -> bool {
        false
    }
    fn request_permission(&self) -> EcoResult<()> {
        Ok(())
    }
}
