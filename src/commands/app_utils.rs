// src/commands/app_utils.rs - App utility functions for close_all and list operations
// Extracted from app.rs but now only handles system-level operations

use std::process::Command;

/// Close all running applications
pub fn close_all_apps() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(&["/F", "/IM", "*"])
            .output()?;
        Ok("Closed all running applications".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("killall")
            .arg("-9")
            .arg("*")
            .output()?;
        Ok("Closed all running applications".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("killall")
            .arg("-9")
            .arg("*")
            .output()?;
        Ok("Closed all running applications".to_string())
    }
}

/// List currently running applications
pub fn list_running_apps() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("tasklist")
            .args(&["/FO", "CSV", "/NH"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() > 1 {
                        Some(parts[0].trim_matches('"').to_string())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("ps")
            .args(&["aux"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .skip(1)
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 10 {
                        Some(parts[10].to_string())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Get count of tracked apps (delegates to process_tracker)
pub fn get_tracked_app_count() -> usize {
    crate::utils::get_tracked_app_count()
}
