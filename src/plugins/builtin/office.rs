// src/plugins/builtin/office.rs
// Office and productivity apps plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "office".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Office and productivity applications".to_string(),
            keywords: vec!["office", "word", "excel", "powerpoint", "outlook", "teams", "docs"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Microsoft Office - Open commands
            PluginCommand {
                trigger: "open word".to_string(),
                description: "Opens Microsoft Word".to_string(),
                examples: vec!["open word".to_string(), "word".to_string(), "microsoft word".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:word".to_string(),
            },
            PluginCommand {
                trigger: "open excel".to_string(),
                description: "Opens Microsoft Excel".to_string(),
                examples: vec!["open excel".to_string(), "excel".to_string(), "spreadsheet".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:excel".to_string(),
            },
            PluginCommand {
                trigger: "open powerpoint".to_string(),
                description: "Opens Microsoft PowerPoint".to_string(),
                examples: vec!["open powerpoint".to_string(), "powerpoint".to_string(), "presentation".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:powerpoint".to_string(),
            },
            PluginCommand {
                trigger: "open outlook".to_string(),
                description: "Opens Microsoft Outlook".to_string(),
                examples: vec!["open outlook".to_string(), "outlook".to_string(), "email".to_string(), "mail".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:outlook".to_string(),
            },
            PluginCommand {
                trigger: "open onenote".to_string(),
                description: "Opens Microsoft OneNote".to_string(),
                examples: vec!["open onenote".to_string(), "onenote".to_string(), "notes".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:onenote".to_string(),
            },
            PluginCommand {
                trigger: "open teams".to_string(),
                description: "Opens Microsoft Teams".to_string(),
                examples: vec!["open teams".to_string(), "teams".to_string(), "microsoft teams".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:teams".to_string(),
            },
            // Close commands
            PluginCommand {
                trigger: "close word".to_string(),
                description: "Closes Microsoft Word".to_string(),
                examples: vec!["close word".to_string(), "quit word".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:word".to_string(),
            },
            PluginCommand {
                trigger: "close excel".to_string(),
                description: "Closes Microsoft Excel".to_string(),
                examples: vec!["close excel".to_string(), "quit excel".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:excel".to_string(),
            },
            PluginCommand {
                trigger: "close powerpoint".to_string(),
                description: "Closes PowerPoint".to_string(),
                examples: vec!["close powerpoint".to_string(), "quit powerpoint".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:powerpoint".to_string(),
            },
            PluginCommand {
                trigger: "close outlook".to_string(),
                description: "Closes Outlook".to_string(),
                examples: vec!["close outlook".to_string(), "quit outlook".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:outlook".to_string(),
            },
            PluginCommand {
                trigger: "close teams".to_string(),
                description: "Closes Microsoft Teams".to_string(),
                examples: vec!["close teams".to_string(), "quit teams".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:teams".to_string(),
            },
            // Google Workspace (web)
            url_cmd!("open google docs", "Opens Google Docs", &["open google docs", "google docs", "docs"], "https://docs.google.com"),
            url_cmd!("open google sheets", "Opens Google Sheets", &["open google sheets", "google sheets", "sheets"], "https://sheets.google.com"),
            url_cmd!("open google slides", "Opens Google Slides", &["open google slides", "google slides", "slides"], "https://slides.google.com"),
            url_cmd!("open google drive", "Opens Google Drive", &["open google drive", "google drive", "drive"], "https://drive.google.com"),
            url_cmd!("open gmail", "Opens Gmail", &["open gmail", "gmail", "check email"], "https://mail.google.com"),
        ],
    }
}
