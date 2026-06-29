// src/platform/system_control.rs
// Cross-platform system control operations

use std::process::Command;

/// System control operations trait
pub trait SystemController: Send + Sync {
    /// Shutdown the system
    fn shutdown(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Restart the system
    fn restart(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Sleep/suspend the system
    fn sleep(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Hibernate the system
    fn hibernate(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Lock the screen
    fn lock_screen(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Set system volume (0-100)
    fn set_volume(&self, level: u8) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Increase volume by percentage
    fn increase_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Decrease volume by percentage
    fn decrease_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Mute system audio
    fn mute(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Unmute system audio
    fn unmute(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Enable WiFi
    fn enable_wifi(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Disable WiFi
    fn disable_wifi(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Enable Bluetooth
    fn enable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Disable Bluetooth
    fn disable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Get current brightness (0-100)
    fn get_brightness(&self) -> Result<u8, Box<dyn std::error::Error>>;
    
    /// Set screen brightness (0-100)
    fn set_brightness(&self, level: u8) -> Result<String, Box<dyn std::error::Error>>;
}

// ============================================================================
// Windows Implementation
// ============================================================================

pub struct WindowsSystemController;

impl SystemController for WindowsSystemController {
    fn shutdown(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("shutdown")
            .args(["/s", "/t", "0"])
            .output()?;
        Ok("Shutting down system...".to_string())
    }
    
    fn restart(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("shutdown")
            .args(["/r", "/t", "0"])
            .output()?;
        Ok("Restarting system...".to_string())
    }
    
    fn sleep(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("rundll32.exe")
            .args(["powrprof.dll,SetSuspendState", "0,1,0"])
            .output()?;
        Ok("Entering sleep mode...".to_string())
    }
    
    fn hibernate(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("shutdown")
            .args(["/h"])
            .output()?;
        Ok("Hibernating system...".to_string())
    }
    
    fn lock_screen(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("rundll32.exe")
            .args(["user32.dll,LockWorkStation"])
            .output()?;
        Ok("Locking screen...".to_string())
    }
    
    fn set_volume(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        let _ps_cmd = format!(
            "$obj = New-Object -ComObject WScript.Shell; $obj.SendKeys([char]174); \
             Start-Sleep -Milliseconds 50; \
             (New-Object -ComObject WScript.Shell).SendKeys([char]175)"
        );
        
        // Use nircmd if available, otherwise use PowerShell
        let result = Command::new("nircmd.exe")
            .args(["setsysvolume", &((level as f32 / 100.0 * 65535.0) as u32).to_string()])
            .output();
            
        if result.is_ok() {
            Ok(format!("Volume set to {}%", level))
        } else {
            // Fallback to PowerShell
            Command::new("powershell")
                .args(["-Command", &format!(
                    "$obj = New-Object -ComObject WScript.Shell; \
                     for($i=0; $i -lt 50; $i++) {{ $obj.SendKeys([char]174) }}; \
                     for($i=0; $i -lt {}; $i++) {{ $obj.SendKeys([char]175) }}", 
                    level / 2
                )])
                .output()?;
            Ok(format!("Volume adjusted to approximately {}%", level))
        }
    }
    
    fn increase_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        println!("[SYSTEM] Increasing volume by {}%", amount);
        
        // Try nircmd first (more reliable)
        let result = Command::new("nircmd.exe")
            .args(["changesysvolume", &((amount as f32 / 100.0 * 65535.0) as i32).to_string()])
            .output();
        
        if result.is_ok() && result.as_ref().unwrap().status.success() {
            println!("[SYSTEM] Volume increased using nircmd");
            return Ok(format!("Volume increased by {}%", amount));
        }
        
        // Fallback to PowerShell key simulation
        println!("[SYSTEM] Using PowerShell fallback for volume");
        let steps = (amount / 2).max(1);
        let output = Command::new("powershell")
            .args(["-Command", &format!(
                "$obj = New-Object -ComObject WScript.Shell; \
                 for($i=0; $i -lt {}; $i++) {{ $obj.SendKeys([char]175); Start-Sleep -Milliseconds 50 }}", 
                steps
            )])
            .output()?;
        
        if output.status.success() {
            println!("[SYSTEM] Volume increased using PowerShell");
            Ok(format!("Volume increased by {}%", amount))
        } else {
            Err("Failed to increase volume".into())
        }
    }
    
    fn decrease_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        let steps = (amount / 2).max(1);
        Command::new("powershell")
            .args(["-Command", &format!(
                "$obj = New-Object -ComObject WScript.Shell; \
                 for($i=0; $i -lt {}; $i++) {{ $obj.SendKeys([char]174) }}", 
                steps
            )])
            .output()?;
        Ok(format!("Volume decreased by {}%", amount))
    }
    
    fn mute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("powershell")
            .args(["-Command", 
                "(New-Object -ComObject WScript.Shell).SendKeys([char]173)"])
            .output()?;
        Ok("Audio muted".to_string())
    }
    
    fn unmute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("powershell")
            .args(["-Command", 
                "(New-Object -ComObject WScript.Shell).SendKeys([char]173)"])
            .output()?;
        Ok("Audio unmuted".to_string())
    }
    
    fn enable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("netsh")
            .args(["interface", "set", "interface", "Wi-Fi", "enabled"])
            .output()?;
        Ok("WiFi enabled".to_string())
    }
    
    fn disable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("netsh")
            .args(["interface", "set", "interface", "Wi-Fi", "disabled"])
            .output()?;
        Ok("WiFi disabled".to_string())
    }
    
    fn enable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Windows Bluetooth control via PowerShell
        Command::new("powershell")
            .args(["-Command", 
                "Get-PnpDevice | Where-Object {$_.Class -eq 'Bluetooth' -and $_.Status -eq 'Error'} | Enable-PnpDevice -Confirm:$false"])
            .output()?;
        Ok("Bluetooth enabled".to_string())
    }
    
    fn disable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("powershell")
            .args(["-Command", 
                "Get-PnpDevice | Where-Object {$_.Class -eq 'Bluetooth' -and $_.Status -eq 'OK'} | Disable-PnpDevice -Confirm:$false"])
            .output()?;
        Ok("Bluetooth disabled".to_string())
    }
    
    fn get_brightness(&self) -> Result<u8, Box<dyn std::error::Error>> {
        println!("[SYSTEM] Getting current brightness...");
        let output = Command::new("powershell")
            .args(["-Command", 
                "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightness).CurrentBrightness"])
            .output()?;
        
        let brightness_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let brightness = brightness_str.parse::<u8>().unwrap_or(50);
        println!("[SYSTEM] Current brightness: {}%", brightness);
        Ok(brightness)
    }
    
    fn set_brightness(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        println!("[SYSTEM] Setting brightness to {}%", level);
        
        let output = Command::new("powershell")
            .args(["-Command", &format!(
                "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,{})", 
                level
            )])
            .output()?;
        
        if output.status.success() {
            println!("[SYSTEM] Brightness set successfully");
            Ok(format!("Brightness set to {}%", level))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("[SYSTEM] Brightness set failed: {}", stderr);
            Err(format!("Failed to set brightness: {}", stderr).into())
        }
    }
}

// ============================================================================
// Linux Implementation
// ============================================================================

pub struct LinuxSystemController;

impl SystemController for LinuxSystemController {
    fn shutdown(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("systemctl")
            .arg("poweroff")
            .output()?;
        Ok("Shutting down system...".to_string())
    }
    
    fn restart(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("systemctl")
            .arg("reboot")
            .output()?;
        Ok("Restarting system...".to_string())
    }
    
    fn sleep(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("systemctl")
            .arg("suspend")
            .output()?;
        Ok("Entering sleep mode...".to_string())
    }
    
    fn hibernate(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("systemctl")
            .arg("hibernate")
            .output()?;
        Ok("Hibernating system...".to_string())
    }
    
    fn lock_screen(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Try multiple lock commands (different desktop environments)
        let commands = vec![
            vec!["loginctl", "lock-session"],
            vec!["gnome-screensaver-command", "-l"],
            vec!["xdg-screensaver", "lock"],
            vec!["dm-tool", "lock"],
        ];
        
        for cmd in commands {
            if Command::new(cmd[0]).args(&cmd[1..]).output().is_ok() {
                return Ok("Screen locked".to_string());
            }
        }
        
        Err("Could not lock screen. No compatible lock command found.".into())
    }
    
    fn set_volume(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        Command::new("amixer")
            .args(["set", "Master", &format!("{}%", level)])
            .output()?;
        Ok(format!("Volume set to {}%", level))
    }
    
    fn increase_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("amixer")
            .args(["set", "Master", &format!("{}%+", amount)])
            .output()?;
        Ok(format!("Volume increased by {}%", amount))
    }
    
    fn decrease_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("amixer")
            .args(["set", "Master", &format!("{}%-", amount)])
            .output()?;
        Ok(format!("Volume decreased by {}%", amount))
    }
    
    fn mute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("amixer")
            .args(["set", "Master", "mute"])
            .output()?;
        Ok("Audio muted".to_string())
    }
    
    fn unmute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("amixer")
            .args(["set", "Master", "unmute"])
            .output()?;
        Ok("Audio unmuted".to_string())
    }
    
    fn enable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("nmcli")
            .args(["radio", "wifi", "on"])
            .output()?;
        Ok("WiFi enabled".to_string())
    }
    
    fn disable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("nmcli")
            .args(["radio", "wifi", "off"])
            .output()?;
        Ok("WiFi disabled".to_string())
    }
    
    fn enable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("bluetoothctl")
            .arg("power")
            .arg("on")
            .output()?;
        Ok("Bluetooth enabled".to_string())
    }
    
    fn disable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("bluetoothctl")
            .arg("power")
            .arg("off")
            .output()?;
        Ok("Bluetooth disabled".to_string())
    }
    
    fn get_brightness(&self) -> Result<u8, Box<dyn std::error::Error>> {
        let output = Command::new("brightnessctl")
            .arg("get")
            .output()?;
        
        let brightness_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let brightness = brightness_str.parse::<u8>().unwrap_or(50);
        Ok(brightness)
    }
    
    fn set_brightness(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        Command::new("brightnessctl")
            .args(["set", &format!("{}%", level)])
            .output()?;
        Ok(format!("Brightness set to {}%", level))
    }
}

// ============================================================================
// macOS Implementation
// ============================================================================

pub struct MacOSSystemController;

impl SystemController for MacOSSystemController {
    fn shutdown(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", "tell app \"System Events\" to shut down"])
            .output()?;
        Ok("Shutting down system...".to_string())
    }
    
    fn restart(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", "tell app \"System Events\" to restart"])
            .output()?;
        Ok("Restarting system...".to_string())
    }
    
    fn sleep(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("pmset")
            .args(["sleepnow"])
            .output()?;
        Ok("Entering sleep mode...".to_string())
    }
    
    fn hibernate(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("pmset")
            .args(["hibernatemode", "25"])
            .output()?;
        Command::new("pmset")
            .args(["sleepnow"])
            .output()?;
        Ok("Hibernating system...".to_string())
    }
    
    fn lock_screen(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to keystroke \"q\" using {command down, control down}"])
            .output()?;
        Ok("Screen locked".to_string())
    }
    
    fn set_volume(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        Command::new("osascript")
            .args(["-e", &format!("set volume output volume {}", level)])
            .output()?;
        Ok(format!("Volume set to {}%", level))
    }
    
    fn increase_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", &format!("set volume output volume (output volume of (get volume settings) + {})", amount)])
            .output()?;
        Ok(format!("Volume increased by {}%", amount))
    }
    
    fn decrease_volume(&self, amount: u8) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", &format!("set volume output volume (output volume of (get volume settings) - {})", amount)])
            .output()?;
        Ok(format!("Volume decreased by {}%", amount))
    }
    
    fn mute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", "set volume with output muted"])
            .output()?;
        Ok("Audio muted".to_string())
    }
    
    fn unmute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("osascript")
            .args(["-e", "set volume without output muted"])
            .output()?;
        Ok("Audio unmuted".to_string())
    }
    
    fn enable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("networksetup")
            .args(["-setairportpower", "en0", "on"])
            .output()?;
        Ok("WiFi enabled".to_string())
    }
    
    fn disable_wifi(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("networksetup")
            .args(["-setairportpower", "en0", "off"])
            .output()?;
        Ok("WiFi disabled".to_string())
    }
    
    fn enable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("blueutil")
            .args(["--power", "1"])
            .output()?;
        Ok("Bluetooth enabled".to_string())
    }
    
    fn disable_bluetooth(&self) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("blueutil")
            .args(["--power", "0"])
            .output()?;
        Ok("Bluetooth disabled".to_string())
    }
    
    fn get_brightness(&self) -> Result<u8, Box<dyn std::error::Error>> {
        let output = Command::new("brightness")
            .arg("-l")
            .output()?;
        
        let brightness_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let brightness = (brightness_str.parse::<f32>().unwrap_or(0.5) * 100.0) as u8;
        Ok(brightness)
    }
    
    fn set_brightness(&self, level: u8) -> Result<String, Box<dyn std::error::Error>> {
        let level = level.min(100);
        let level_float = level as f32 / 100.0;
        Command::new("brightness")
            .arg(level_float.to_string())
            .output()?;
        Ok(format!("Brightness set to {}%", level))
    }
}

/// Get the system controller for the current platform
pub fn get_system_controller() -> Box<dyn SystemController> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsSystemController)
    }
    
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxSystemController)
    }
    
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSSystemController)
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        compile_error!("Unsupported platform")
    }
}
