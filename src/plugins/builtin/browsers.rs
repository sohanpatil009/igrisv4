// src/plugins/builtin/browsers.rs
// Browser applications plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "browsers".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Web browser applications".to_string(),
            keywords: vec!["browser", "web", "internet", "chrome", "firefox", "edge", "brave", "opera"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Open commands - use CustomFunction to call AppLauncher (cross-platform)
            PluginCommand {
                trigger: "open chrome".to_string(),
                description: "Opens Google Chrome".to_string(),
                examples: vec!["open chrome".to_string(), "chrome".to_string(), "launch chrome".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:chrome".to_string(),
            },
            PluginCommand {
                trigger: "open firefox".to_string(),
                description: "Opens Mozilla Firefox".to_string(),
                examples: vec!["open firefox".to_string(), "firefox".to_string(), "launch firefox".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:firefox".to_string(),
            },
            PluginCommand {
                trigger: "open edge".to_string(),
                description: "Opens Microsoft Edge".to_string(),
                examples: vec!["open edge".to_string(), "edge".to_string(), "launch edge".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:edge".to_string(),
            },
            PluginCommand {
                trigger: "open brave".to_string(),
                description: "Opens Brave browser".to_string(),
                examples: vec!["open brave".to_string(), "brave".to_string(), "launch brave".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:brave".to_string(),
            },
            PluginCommand {
                trigger: "open safari".to_string(),
                description: "Opens Safari browser".to_string(),
                examples: vec!["open safari".to_string(), "safari".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:safari".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close chrome".to_string(),
                description: "Closes Google Chrome".to_string(),
                examples: vec!["close chrome".to_string(), "quit chrome".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:chrome".to_string(),
            },
            PluginCommand {
                trigger: "close firefox".to_string(),
                description: "Closes Mozilla Firefox".to_string(),
                examples: vec!["close firefox".to_string(), "quit firefox".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:firefox".to_string(),
            },
            PluginCommand {
                trigger: "close edge".to_string(),
                description: "Closes Microsoft Edge".to_string(),
                examples: vec!["close edge".to_string(), "quit edge".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:edge".to_string(),
            },
        ],
    }
}

