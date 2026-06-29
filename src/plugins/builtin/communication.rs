// src/plugins/builtin/communication.rs
// Communication apps plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "communication".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Communication and messaging apps".to_string(),
            keywords: vec!["chat", "message", "discord", "slack", "zoom", "whatsapp", "telegram", "skype"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Desktop apps - Open commands
            PluginCommand {
                trigger: "open discord".to_string(),
                description: "Opens Discord".to_string(),
                examples: vec!["open discord".to_string(), "discord".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:discord".to_string(),
            },
            PluginCommand {
                trigger: "open slack".to_string(),
                description: "Opens Slack".to_string(),
                examples: vec!["open slack".to_string(), "slack".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:slack".to_string(),
            },
            PluginCommand {
                trigger: "open zoom".to_string(),
                description: "Opens Zoom".to_string(),
                examples: vec!["open zoom".to_string(), "zoom".to_string(), "zoom meeting".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:zoom".to_string(),
            },
            PluginCommand {
                trigger: "open skype".to_string(),
                description: "Opens Skype".to_string(),
                examples: vec!["open skype".to_string(), "skype".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:skype".to_string(),
            },
            PluginCommand {
                trigger: "open telegram".to_string(),
                description: "Opens Telegram".to_string(),
                examples: vec!["open telegram".to_string(), "telegram".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:telegram".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close discord".to_string(),
                description: "Closes Discord".to_string(),
                examples: vec!["close discord".to_string(), "quit discord".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:discord".to_string(),
            },
            PluginCommand {
                trigger: "close slack".to_string(),
                description: "Closes Slack".to_string(),
                examples: vec!["close slack".to_string(), "quit slack".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:slack".to_string(),
            },
            PluginCommand {
                trigger: "close zoom".to_string(),
                description: "Closes Zoom".to_string(),
                examples: vec!["close zoom".to_string(), "quit zoom".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:zoom".to_string(),
            },
            PluginCommand {
                trigger: "close skype".to_string(),
                description: "Closes Skype".to_string(),
                examples: vec!["close skype".to_string(), "quit skype".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:skype".to_string(),
            },
            PluginCommand {
                trigger: "close telegram".to_string(),
                description: "Closes Telegram".to_string(),
                examples: vec!["close telegram".to_string(), "quit telegram".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:telegram".to_string(),
            },
            // Web versions
            url_cmd!("open whatsapp", "Opens WhatsApp Web", &["open whatsapp", "whatsapp", "whatsapp web"], "https://web.whatsapp.com"),
            url_cmd!("open messenger", "Opens Facebook Messenger", &["open messenger", "messenger", "facebook messenger"], "https://www.messenger.com"),
        ],
    }
}
