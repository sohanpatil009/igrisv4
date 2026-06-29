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
