// src/plugins/builtin/utilities.rs
// System utilities plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "utilities".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "System utilities and tools".to_string(),
            keywords: vec!["utility", "tools", "system", "calculator", "paint", "terminal", "notepad", "close all"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Open commands
            PluginCommand {
                trigger: "open calculator".to_string(),
                description: "Opens Calculator".to_string(),
                examples: vec!["open calculator".to_string(), "calculator".to_string(), "calc".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:calculator".to_string(),
            },
            PluginCommand {
                trigger: "open paint".to_string(),
                description: "Opens Paint".to_string(),
                examples: vec!["open paint".to_string(), "paint".to_string(), "drawing app".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:paint".to_string(),
            },
            PluginCommand {
                trigger: "open terminal".to_string(),
                description: "Opens Terminal".to_string(),
                examples: vec!["open terminal".to_string(), "terminal".to_string(), "open cmd".to_string(), "command prompt".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:terminal".to_string(),
            },
            PluginCommand {
                trigger: "open notepad".to_string(),
                description: "Opens Notepad".to_string(),
                examples: vec!["open notepad".to_string(), "notepad".to_string(), "text editor".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:notepad".to_string(),
            },
            PluginCommand {
                trigger: "open file explorer".to_string(),
                description: "Opens File Explorer".to_string(),
                examples: vec!["open file explorer".to_string(), "file explorer".to_string(), "explorer".to_string(), "my computer".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:explorer".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close calculator".to_string(),
                description: "Closes Calculator".to_string(),
                examples: vec!["close calculator".to_string(), "quit calculator".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:calculator".to_string(),
            },
            PluginCommand {
                trigger: "close paint".to_string(),
                description: "Closes Paint".to_string(),
                examples: vec!["close paint".to_string(), "quit paint".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:paint".to_string(),
            },
            PluginCommand {
                trigger: "close notepad".to_string(),
                description: "Closes Notepad".to_string(),
                examples: vec!["close notepad".to_string(), "quit notepad".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:notepad".to_string(),
            },
            // Close all - uses custom function to close all tracked apps
            PluginCommand {
                trigger: "close all".to_string(),
                description: "Closes all apps opened by IGRIS".to_string(),
                examples: vec!["close all".to_string(), "close all apps".to_string(), "close everything".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_all_apps".to_string(),
            },
        ],
    }
}
