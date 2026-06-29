// src/plugins/builtin/system_control.rs
// System control plugin (volume, brightness, power, network)

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "system_control".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Control system settings - volume, brightness, power, network".to_string(),
            keywords: vec![
                "volume", "brightness", "shutdown", "restart", "sleep", "lock",
                "mute", "unmute", "wifi", "bluetooth", "power", "screen"
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            enabled: true,
        },
        commands: vec![
            // Volume commands
            cmd!(
                "increase volume",
                "Increases system volume",
                &[
                    "increase volume",
                    "increase volume by 20",
                    "volume up",
                    "raise volume by 50"
                ],
                ActionType::CustomFunction,
                "system_volume_increase"
            ),
            cmd!(
                "decrease volume",
                "Decreases system volume",
                &[
                    "decrease volume",
                    "decrease volume by 20",
                    "volume down",
                    "lower volume by 30"
                ],
                ActionType::CustomFunction,
                "system_volume_decrease"
            ),
            cmd!(
                "set volume",
                "Sets volume to specific level",
                &[
                    "set volume to 50",
                    "volume 80",
                    "set volume 60 percent"
                ],
                ActionType::CustomFunction,
                "system_volume_set"
            ),
            cmd!(
                "mute",
                "Mutes system audio",
                &["mute", "mute audio", "mute sound"],
                ActionType::CustomFunction,
                "system_mute"
            ),
            cmd!(
                "unmute",
                "Unmutes system audio",
                &["unmute", "unmute audio", "unmute sound"],
                ActionType::CustomFunction,
                "system_unmute"
            ),
            
            // Brightness commands
            cmd!(
                "increase brightness",
                "Increases screen brightness",
                &[
                    "increase brightness",
                    "increase brightness by 20",
                    "brightness up",
                    "raise brightness by 30"
                ],
                ActionType::CustomFunction,
                "system_brightness_increase"
            ),
            cmd!(
                "decrease brightness",
                "Decreases screen brightness",
                &[
                    "decrease brightness",
                    "decrease brightness by 20",
                    "brightness down",
                    "lower brightness by 30"
                ],
                ActionType::CustomFunction,
                "system_brightness_decrease"
            ),
            cmd!(
                "set brightness",
                "Sets brightness to specific level",
                &[
                    "set brightness to 50",
                    "brightness 80",
                    "set brightness 60 percent"
                ],
                ActionType::CustomFunction,
                "system_brightness_set"
            ),
            
            // Power commands
            cmd!(
                "shutdown",
                "Shuts down the computer",
                &["shutdown", "shut down", "power off", "shutdown computer"],
                ActionType::CustomFunction,
                "system_shutdown"
            ),
            cmd!(
                "restart",
                "Restarts the computer",
                &["restart", "reboot", "restart computer", "reboot system"],
                ActionType::CustomFunction,
                "system_restart"
            ),
            cmd!(
                "sleep",
                "Puts computer to sleep",
                &["sleep", "go to sleep", "sleep mode"],
                ActionType::CustomFunction,
                "system_sleep"
            ),
            cmd!(
                "lock screen",
                "Locks the screen",
                &["lock", "lock screen", "lock computer"],
                ActionType::CustomFunction,
                "system_lock"
            ),
            
            // Network commands
            cmd!(
                "enable wifi",
                "Turns WiFi on",
                &["enable wifi", "turn on wifi", "wifi on", "start wifi"],
                ActionType::CustomFunction,
                "system_wifi_on"
            ),
            cmd!(
                "disable wifi",
                "Turns WiFi off",
                &["disable wifi", "turn off wifi", "wifi off", "stop wifi"],
                ActionType::CustomFunction,
                "system_wifi_off"
            ),
            cmd!(
                "enable bluetooth",
                "Turns Bluetooth on",
                &["enable bluetooth", "turn on bluetooth", "bluetooth on"],
                ActionType::CustomFunction,
                "system_bluetooth_on"
            ),
            cmd!(
                "disable bluetooth",
                "Turns Bluetooth off",
                &["disable bluetooth", "turn off bluetooth", "bluetooth off"],
                ActionType::CustomFunction,
                "system_bluetooth_off"
            ),
        ],
    }
}
