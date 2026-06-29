// src/plugins/builtin/editors.rs
// Code editors plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "editors".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Code editors and IDEs".to_string(),
            keywords: vec!["editor", "code", "vscode", "sublime", "atom", "ide", "jetbrains"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Open commands
            PluginCommand {
                trigger: "open vscode".to_string(),
                description: "Opens VS Code".to_string(),
                examples: vec!["open vscode".to_string(), "vscode".to_string(), "visual studio code".to_string(), "code".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:vscode".to_string(),
            },
            PluginCommand {
                trigger: "open sublime".to_string(),
                description: "Opens Sublime Text".to_string(),
                examples: vec!["open sublime".to_string(), "sublime".to_string(), "sublime text".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:sublime".to_string(),
            },
            PluginCommand {
                trigger: "open atom".to_string(),
                description: "Opens Atom".to_string(),
                examples: vec!["open atom".to_string(), "atom".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:atom".to_string(),
            },
            PluginCommand {
                trigger: "open notepad++".to_string(),
                description: "Opens Notepad++".to_string(),
                examples: vec!["open notepad++".to_string(), "notepad++".to_string(), "npp".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:notepad++".to_string(),
            },
            PluginCommand {
                trigger: "open intellij".to_string(),
                description: "Opens IntelliJ IDEA".to_string(),
                examples: vec!["open intellij".to_string(), "intellij".to_string(), "idea".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:intellij".to_string(),
            },
            PluginCommand {
                trigger: "open pycharm".to_string(),
                description: "Opens PyCharm".to_string(),
                examples: vec!["open pycharm".to_string(), "pycharm".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:pycharm".to_string(),
            },
            PluginCommand {
                trigger: "open webstorm".to_string(),
                description: "Opens WebStorm".to_string(),
                examples: vec!["open webstorm".to_string(), "webstorm".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:webstorm".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close vscode".to_string(),
                description: "Closes VS Code".to_string(),
                examples: vec!["close vscode".to_string(), "quit vscode".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:vscode".to_string(),
            },
            PluginCommand {
                trigger: "close sublime".to_string(),
                description: "Closes Sublime".to_string(),
                examples: vec!["close sublime".to_string(), "quit sublime".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:sublime".to_string(),
            },
            PluginCommand {
                trigger: "close intellij".to_string(),
                description: "Closes IntelliJ".to_string(),
                examples: vec!["close intellij".to_string(), "quit intellij".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:intellij".to_string(),
            },
        ],
    }
}
