use crate::eco::errors::{EcoError, EcoResult};
use super::{PlatformNotification, RawNotification};
use std::process::Command;

pub struct MacosNotification;

impl PlatformNotification for MacosNotification {
    fn read_notifications(&self) -> EcoResult<Vec<RawNotification>> {
        // Use osascript to read notifications from macOS Notification Center
        // via System Events accessibility API
        let script = r#"
        tell application "System Events"
            set notifList to {}
            try
                tell process "NotificationCenter"
                    set notifWindows to every window
                    repeat with w in notifWindows
                        try
                            set notifItems to every UI element of w
                            repeat with item in notifItems
                                try
                                    set itemTitle to value of static text 1 of item
                                    set itemBody to value of static text 2 of item
                                    set itemApp to name of first process
                                    copy (itemApp & "|||" & itemTitle & "|||" & itemBody) to end of notifList
                                end try
                            end repeat
                        end try
                    end repeat
                end tell
            end try
            return notifList
        end tell
        "#;

        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| EcoError::Notification(format!("Failed to run osascript: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut notifications = Vec::new();
        let entries: Vec<&str> = stdout.trim().split("\n").collect();

        for (i, entry) in entries.iter().enumerate() {
            let parts: Vec<&str> = entry.split("|||").collect();
            if parts.len() >= 2 {
                notifications.push(RawNotification {
                    id: format!("macos_notif_{}", i),
                    app_name: parts[0].trim().to_string(),
                    title: parts[1].trim().to_string(),
                    body: if parts.len() > 2 { parts[2].trim().to_string() } else { String::new() },
                    timestamp: chrono::Utc::now().timestamp_millis(),
                });
            }
        }

        Ok(notifications)
    }

    fn reply_to_notification(&self, notification_id: &str, reply: &str) -> EcoResult<()> {
        // Try to reply via AppleScript to the target app
        // First try Messages app
        let script = format!(
            r#"
            tell application "Messages"
                set targetService to 1st account whose service type = iMessage
                set targetBuddy to participant "{}"
                send "{}" to targetBuddy
            end tell
            "#,
            notification_id, reply
        );

        let result = Command::new("osascript")
            .args(["-e", &script])
            .output();

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                // Fallback: copy to clipboard and simulate paste
                let _ = Command::new("pbcopy")
                    .arg(reply)
                    .output();

                // Simulate Cmd+V to paste into whatever is focused
                let paste_script = r#"
                tell application "System Events"
                    keystroke "v" using command down
                    delay 0.1
                    key code 36
                end tell
                "#;

                Command::new("osascript")
                    .args(["-e", paste_script])
                    .output()
                    .map_err(|e| EcoError::Notification(format!("Paste fallback failed: {}", e)))?;

                Ok(())
            }
        }
    }

    fn has_permission(&self) -> bool {
        // Check if accessibility permissions are granted
        let script = r#"
        tell application "System Events"
            try
                set frontApp to name of first application process whose frontmost is true
                return "granted"
            on error
                return "denied"
            end try
        end tell
        "#;

        Command::new("osascript")
            .args(["-e", script])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                stdout.trim() == "granted"
            })
            .unwrap_or(false)
    }

    fn request_permission(&self) -> EcoResult<()> {
        // Open System Preferences to Accessibility pane
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .output();

        Ok(())
    }
}
