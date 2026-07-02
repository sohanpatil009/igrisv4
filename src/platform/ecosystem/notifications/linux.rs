use crate::eco::errors::{EcoError, EcoResult};
use super::{PlatformNotification, RawNotification};
use std::process::Command;

pub struct LinuxNotification;

impl PlatformNotification for LinuxNotification {
    fn read_notifications(&self) -> EcoResult<Vec<RawNotification>> {
        // Use gdbus to read notifications via D-Bus
        let output = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest", "org.freedesktop.Notifications",
                "--object-path", "/org/freedesktop/Notifications",
                "--method", "org.freedesktop.Notifications.GetServerInformation",
            ])
            .output()
            .map_err(|e| EcoError::Notification(format!("D-Bus call failed: {}", e)))?;

        let _stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // D-Bus doesn't provide a direct "list all notifications" API
        // We need to monitor the NotificationsAdded signal instead
        // For now, return empty - the polling loop will use the signal monitor
        Ok(Vec::new())
    }

    fn reply_to_notification(&self, _notification_id: &str, reply: &str) -> EcoResult<()> {
        // Copy to clipboard and simulate paste via xdotool
        let _ = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .arg(reply)
            .output();

        // Simulate Ctrl+V
        let _ = Command::new("xdotool")
            .args(["key", "ctrl+v"])
            .output();

        Ok(())
    }

    fn has_permission(&self) -> bool {
        // Check if D-Bus notifications are available
        Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest", "org.freedesktop.Notifications",
                "--object-path", "/org/freedesktop/Notifications",
                "--method", "org.freedesktop.Notifications.GetServerInformation",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn request_permission(&self) -> EcoResult<()> {
        // Linux typically doesn't require explicit permission
        Ok(())
    }
}
