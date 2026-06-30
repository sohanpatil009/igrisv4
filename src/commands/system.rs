// src/commands/system.rs
// High-level system control interface with NER integration

use crate::nlu::ner::GLOBAL_NER;
use crate::platform::get_system_controller;

/// Process system control commands with NER
pub fn process_system_command(command: &str) -> Option<String> {
    let cmd_lower = command.to_lowercase();
    let controller = get_system_controller();
    let _entities = GLOBAL_NER.extract_entities(&cmd_lower);
    
    // Shutdown commands
    if cmd_lower.contains("shutdown") || cmd_lower.contains("shut down") || cmd_lower.contains("power off") {
        return controller.shutdown().ok();
    }
    
    // Restart commands
    if cmd_lower.contains("restart") || cmd_lower.contains("reboot") {
        return controller.restart().ok();
    }
    
    // Sleep commands
    if cmd_lower.contains("sleep") && !cmd_lower.contains("go to sleep") {
        return controller.sleep().ok();
    }
    
    // Lock screen commands
    if cmd_lower.contains("lock") {
        return controller.lock_screen().ok();
    }
    
    // Volume commands
    if cmd_lower.contains("volume") {
        return handle_volume_command(&cmd_lower, &controller);
    }
    
    // Mute/unmute commands
    if cmd_lower.contains("mute") && !cmd_lower.contains("unmute") {
        return controller.mute().ok();
    }
    
    if cmd_lower.contains("unmute") {
        return controller.unmute().ok();
    }
    
    // Brightness commands
    if cmd_lower.contains("brightness") {
        return handle_brightness_command(&cmd_lower, &controller);
    }
    
    // WiFi commands
    if cmd_lower.contains("wifi") || cmd_lower.contains("wi-fi") || cmd_lower.contains("wireless") {
        if cmd_lower.contains("enable") || cmd_lower.contains("turn on") || cmd_lower.contains("start") {
            return controller.enable_wifi().ok();
        }
        if cmd_lower.contains("disable") || cmd_lower.contains("turn off") || cmd_lower.contains("stop") {
            return controller.disable_wifi().ok();
        }
    }
    
    // Bluetooth commands
    if cmd_lower.contains("bluetooth") || cmd_lower.contains("bt") {
        if cmd_lower.contains("enable") || cmd_lower.contains("turn on") || cmd_lower.contains("start") {
            return controller.enable_bluetooth().ok();
        }
        if cmd_lower.contains("disable") || cmd_lower.contains("turn off") || cmd_lower.contains("stop") {
            return controller.disable_bluetooth().ok();
        }
    }
    
    None
}

/// Handle volume-related commands with NER
fn handle_volume_command(command: &str, controller: &Box<dyn crate::platform::SystemController>) -> Option<String> {
    println!("[SYSTEM CMD] Processing volume command: {}", command);
    
    // Extract percentage using NER
    let percentage = GLOBAL_NER.parse_percentage(command);
    println!("[SYSTEM CMD] Extracted percentage: {:?}", percentage);
    
    // Set volume to specific level
    if command.contains("set") || command.contains("to") {
        if let Some(level) = percentage {
            println!("[SYSTEM CMD] Setting volume to {}%", level);
            return controller.set_volume(level).ok();
        }
    }
    
    // Increase volume
    if command.contains("increase") || command.contains("raise") || command.contains("up") {
        let amount = percentage.unwrap_or(10); // Default 10%
        println!("[SYSTEM CMD] Increasing volume by {}%", amount);
        return controller.increase_volume(amount).ok();
    }
    
    // Decrease volume
    if command.contains("decrease") || command.contains("lower") || command.contains("down") {
        let amount = percentage.unwrap_or(10); // Default 10%
        println!("[SYSTEM CMD] Decreasing volume by {}%", amount);
        return controller.decrease_volume(amount).ok();
    }
    
    // Max volume
    if command.contains("max") || command.contains("maximum") || command.contains("full") {
        return controller.set_volume(100).ok();
    }
    
    // Min volume
    if command.contains("min") || command.contains("minimum") {
        return controller.set_volume(0).ok();
    }
    
    None
}

/// Handle brightness-related commands with NER
fn handle_brightness_command(command: &str, controller: &Box<dyn crate::platform::SystemController>) -> Option<String> {
    println!("[SYSTEM CMD] Processing brightness command: {}", command);
    
    // Extract percentage using NER
    let percentage = GLOBAL_NER.parse_percentage(command);
    println!("[SYSTEM CMD] Extracted percentage: {:?}", percentage);
    
    // Set brightness to specific level
    if command.contains("set") || command.contains("to") {
        if let Some(level) = percentage {
            println!("[SYSTEM CMD] Setting brightness to {}%", level);
            return controller.set_brightness(level).ok();
        }
    }
    
    // Increase brightness
    if command.contains("increase") || command.contains("raise") || command.contains("up") {
        if let Ok(current) = controller.get_brightness() {
            let amount = percentage.unwrap_or(10);
            let new_level = (current + amount).min(100);
            println!("[SYSTEM CMD] Increasing brightness from {}% to {}%", current, new_level);
            return controller.set_brightness(new_level).ok();
        }
    }
    
    // Decrease brightness
    if command.contains("decrease") || command.contains("lower") || command.contains("down") {
        if let Ok(current) = controller.get_brightness() {
            let amount = percentage.unwrap_or(10);
            let new_level = current.saturating_sub(amount);
            println!("[SYSTEM CMD] Decreasing brightness from {}% to {}%", current, new_level);
            return controller.set_brightness(new_level).ok();
        }
    }
    
    // Max brightness
    if command.contains("max") || command.contains("maximum") || command.contains("full") {
        println!("[SYSTEM CMD] Setting brightness to maximum (100%)");
        return controller.set_brightness(100).ok();
    }
    
    // Min brightness
    if command.contains("min") || command.contains("minimum") {
        println!("[SYSTEM CMD] Setting brightness to minimum (10%)");
        return controller.set_brightness(10).ok(); // Keep at 10% minimum for visibility
    }
    
    None
}

/// Take a screenshot using platform tools
pub fn take_screenshot() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("/tmp/igris_screenshot_{}.png", timestamp);

    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("screencapture")
            .args(["-x", &filename])
            .output();
        match result {
            Ok(_) => format!("Screenshot saved to {}", filename),
            Err(e) => format!("Failed to take screenshot: {}", e),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let result = std::process::Command::new("import")
            .args(["-window", "root", &filename])
            .output();
        match result {
            Ok(_) => format!("Screenshot saved to {}", filename),
            Err(e) => format!("Failed to take screenshot: {}", e),
        }
    }
    #[cfg(target_os = "windows")]
    {
        let result = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Windows.Forms; \
                     [Windows.Forms.SendKeys]::SendWait('{{PRTSC}}'); \
                     Start-Sleep -Seconds 1; \
                     [Windows.Forms.Clipboard]::GetImage().Save('{}')",
                    filename
                ),
            ])
            .output();
        match result {
            Ok(_) => format!("Screenshot saved to {}", filename),
            Err(e) => format!("Failed to take screenshot: {}", e),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        format!("Screenshot not supported on this platform")
    }
}

/// Get system information
pub fn get_system_info(info_type: &str) -> String {
    match info_type.to_lowercase().as_str() {
        "os" => {
            #[cfg(target_os = "macos")]
            {
                let output = std::process::Command::new("sw_vers")
                    .arg("-productVersion")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| format!("macOS {}", s.trim()));
                #[cfg(target_arch = "aarch64")]
                let arch = "Apple Silicon";
                #[cfg(target_arch = "x86_64")]
                let arch = "Intel";
                match output {
                    Some(v) => format!("Running {} on {}", v.trim(), arch),
                    None => format!("macOS (unknown version) on {}", arch),
                }
            }
            #[cfg(target_os = "linux")]
            {
                let output = std::process::Command::new("uname")
                    .args(["-a"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Linux".to_string());
                format!("{}", output.trim())
            }
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("cmd")
                    .args(["/c", "ver"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Windows".to_string());
                format!("{}", output.trim())
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("Unknown OS")
            }
        }
        "memory" | "ram" => {
            #[cfg(target_os = "macos")]
            {
                let (used, total) = get_mac_memory();
                format!("Memory: {} GB used / {} GB total", used, total)
            }
            #[cfg(target_os = "linux")]
            {
                let output = std::process::Command::new("free")
                    .args(["-h"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Memory info unavailable".to_string());
                let line = output.lines().nth(1).unwrap_or("");
                format!("{}", line)
            }
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("wmic")
                    .args(["OS", "get", "TotalVisibleMemorySize,FreePhysicalMemory", "/format:csv"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Memory info unavailable".to_string());
                format!("{}", output.trim())
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("Memory info unavailable")
            }
        }
        "cpu" => {
            #[cfg(target_os = "macos")]
            {
                let cores = num_cpus::get();
                let brand = std::process::Command::new("sysctl")
                    .args(["-n", "machdep.cpu.brand_string"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Apple Silicon".to_string());
                format!("{} ({} cores)", brand.trim(), cores)
            }
            #[cfg(target_os = "linux")]
            {
                let brand = std::process::Command::new("grep")
                    .args(["model name", "/proc/cpuinfo"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| {
                        let line = s.lines().next().unwrap_or("");
                        line.split(':').nth(1).unwrap_or("").trim().to_string()
                    })
                    .unwrap_or_else(|| "CPU".to_string());
                let cores = num_cpus::get();
                format!("{} ({} cores)", brand, cores)
            }
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("wmic")
                    .args(["cpu", "get", "name", "/format:csv"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "CPU info unavailable".to_string());
                format!("{}", output.trim())
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("CPU info unavailable")
            }
        }
        "ip" | "ip_address" => {
            let output = std::process::Command::new("curl")
                .args(["-s", "https://api.ipify.org"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_else(|| "IP unavailable".to_string());
            format!("Public IP: {}", output.trim())
        }
        "uptime" => {
            #[cfg(target_os = "macos")]
            {
                let seconds = std::process::Command::new("sysctl")
                    .args(["-n", "kern.boottime"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| {
                        let re = regex::Regex::new(r"sec = (\d+)").ok()?;
                        let caps = re.captures(&s)?;
                        caps.get(1)?.as_str().parse::<u64>().ok()
                    })
                    .map(|boot_sec| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let uptime_secs = now.saturating_sub(boot_sec);
                        format_uptime(uptime_secs)
                    })
                    .unwrap_or_else(|| "Uptime unavailable".to_string());
                format!("System uptime: {}", seconds)
            }
            #[cfg(target_os = "linux")]
            {
                let output = std::process::Command::new("uptime")
                    .args(["-p"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Uptime unavailable".to_string());
                format!("{}", output.trim())
            }
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("wmic")
                    .args(["os", "get", "LastBootUpTime", "/format:csv"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .unwrap_or_else(|| "Uptime unavailable".to_string());
                format!("{}", output.trim())
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("Uptime unavailable")
            }
        }
        _ => {
            // "all" — return a summary of everything
            let os = get_system_info("os");
            let mem = get_system_info("memory");
            let cpu = get_system_info("cpu");
            let uptime = get_system_info("uptime");
            format!("{}. {}. {}. {}.", os, mem, cpu, uptime)
        }
    }
}

#[cfg(target_os = "macos")]
fn get_mac_memory() -> (f64, f64) {
    let output = std::process::Command::new("vm_stat")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());
    let pages = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let total_gb = pages / 1_073_741_824.0;

    if let Some(vm) = output {
        let page_size: f64 = 16384.0;
        let mut used_pages: f64 = 0.0;
        for line in vm.lines() {
            if line.contains("Pages active") {
                used_pages += line.split(':').nth(1)
                    .and_then(|s| s.trim().trim_end_matches('.').parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
            if line.contains("Pages wired") {
                used_pages += line.split(':').nth(1)
                    .and_then(|s| s.trim().trim_end_matches('.').parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        }
        let used_gb = (used_pages * page_size) / 1_073_741_824.0;
        ((used_gb * 10.0).round() / 10.0, (total_gb * 10.0).round() / 10.0)
    } else {
        (0.0, (total_gb * 10.0).round() / 10.0)
    }
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    if days > 0 {
        format!("{} days, {} hours, {} minutes", days, hours, minutes)
    } else if hours > 0 {
        format!("{} hours, {} minutes", hours, minutes)
    } else {
        format!("{} minutes", minutes)
    }
}

/// Read or write clipboard
pub fn clipboard_action(action: &str, text: &str) -> String {
    match action.to_lowercase().as_str() {
        "read" => {
            #[cfg(target_os = "macos")]
            {
                let output = std::process::Command::new("pbpaste")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Clipboard is empty.".to_string());
                format!("Clipboard: {}", output)
            }
            #[cfg(target_os = "linux")]
            {
                let output = std::process::Command::new("xclip")
                    .args(["-o", "-selection", "clipboard"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Clipboard is empty.".to_string());
                format!("Clipboard: {}", output)
            }
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("powershell")
                    .args(["-Command", "Get-Clipboard"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Clipboard is empty.".to_string());
                format!("Clipboard: {}", output)
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("Clipboard not supported on this platform")
            }
        }
        "write" => {
            if text.is_empty() {
                return "Nothing to copy.".to_string();
            }
            #[cfg(target_os = "macos")]
            {
                let mut child = std::process::Command::new("pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to run pbcopy");
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                format!("Copied to clipboard: {}", text)
            }
            #[cfg(target_os = "linux")]
            {
                let mut child = std::process::Command::new("xclip")
                    .args(["-selection", "clipboard"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to run xclip");
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                format!("Copied to clipboard: {}", text)
            }
            #[cfg(target_os = "windows")]
            {
                let mut child = std::process::Command::new("powershell")
                    .args(["-Command", &format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"))])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to run Set-Clipboard");
                let _ = child.wait();
                format!("Copied to clipboard: {}", text)
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                format!("Clipboard not supported on this platform")
            }
        }
        _ => "Unknown clipboard action. Use 'read' or 'write'.".to_string(),
    }
}

/// Check if command is a system control command
pub fn is_system_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase();
    
    let system_keywords = vec![
        "shutdown", "restart", "reboot", "sleep", "lock",
        "volume", "mute", "unmute", "brightness",
        "wifi", "wireless", "bluetooth",
    ];
    
    system_keywords.iter().any(|keyword| cmd_lower.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_system_command() {
        assert!(is_system_command("shutdown computer"));
        assert!(is_system_command("increase volume"));
        assert!(is_system_command("turn on wifi"));
        assert!(!is_system_command("open chrome"));
    }
}
