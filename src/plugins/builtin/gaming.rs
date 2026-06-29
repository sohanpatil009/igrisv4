// src/plugins/builtin/gaming.rs
// Gaming platforms plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "gaming".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Gaming platforms and launchers".to_string(),
            keywords: vec!["gaming", "games", "steam", "epic", "xbox", "origin", "ubisoft", "gog"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Game launchers - Open commands
            PluginCommand {
                trigger: "open steam".to_string(),
                description: "Opens Steam".to_string(),
                examples: vec!["open steam".to_string(), "steam".to_string(), "launch steam".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:steam".to_string(),
            },
            PluginCommand {
                trigger: "open epic games".to_string(),
                description: "Opens Epic Games Launcher".to_string(),
                examples: vec!["open epic games".to_string(), "epic games".to_string(), "epic launcher".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:epic".to_string(),
            },
            PluginCommand {
                trigger: "open origin".to_string(),
                description: "Opens EA Origin".to_string(),
                examples: vec!["open origin".to_string(), "origin".to_string(), "ea origin".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:origin".to_string(),
            },
            PluginCommand {
                trigger: "open ubisoft connect".to_string(),
                description: "Opens Ubisoft Connect".to_string(),
                examples: vec!["open ubisoft connect".to_string(), "ubisoft".to_string(), "uplay".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:ubisoft".to_string(),
            },
            PluginCommand {
                trigger: "open gog galaxy".to_string(),
                description: "Opens GOG Galaxy".to_string(),
                examples: vec!["open gog galaxy".to_string(), "gog".to_string(), "gog galaxy".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:gog".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close steam".to_string(),
                description: "Closes Steam".to_string(),
                examples: vec!["close steam".to_string(), "quit steam".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:steam".to_string(),
            },
            PluginCommand {
                trigger: "close epic games".to_string(),
                description: "Closes Epic Games".to_string(),
                examples: vec!["close epic games".to_string(), "quit epic".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:epic".to_string(),
            },
            PluginCommand {
                trigger: "close origin".to_string(),
                description: "Closes Origin".to_string(),
                examples: vec!["close origin".to_string(), "quit origin".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:origin".to_string(),
            },
            PluginCommand {
                trigger: "close ubisoft connect".to_string(),
                description: "Closes Ubisoft Connect".to_string(),
                examples: vec!["close ubisoft".to_string(), "quit ubisoft".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:ubisoft".to_string(),
            },
        ],
    }
}
