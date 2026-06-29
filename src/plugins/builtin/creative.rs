// src/plugins/builtin/creative.rs
// Creative and design apps plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "creative".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Creative and design applications".to_string(),
            keywords: vec!["creative", "design", "photoshop", "illustrator", "figma", "blender", "gimp"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Adobe Creative Suite - Open commands
            PluginCommand {
                trigger: "open photoshop".to_string(),
                description: "Opens Adobe Photoshop".to_string(),
                examples: vec!["open photoshop".to_string(), "photoshop".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:photoshop".to_string(),
            },
            PluginCommand {
                trigger: "open illustrator".to_string(),
                description: "Opens Adobe Illustrator".to_string(),
                examples: vec!["open illustrator".to_string(), "illustrator".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:illustrator".to_string(),
            },
            PluginCommand {
                trigger: "open premiere".to_string(),
                description: "Opens Adobe Premiere Pro".to_string(),
                examples: vec!["open premiere".to_string(), "premiere".to_string(), "premiere pro".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:premiere".to_string(),
            },
            PluginCommand {
                trigger: "open after effects".to_string(),
                description: "Opens Adobe After Effects".to_string(),
                examples: vec!["open after effects".to_string(), "after effects".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:aftereffects".to_string(),
            },
            // Close Adobe apps
            PluginCommand {
                trigger: "close photoshop".to_string(),
                description: "Closes Photoshop".to_string(),
                examples: vec!["close photoshop".to_string(), "quit photoshop".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:photoshop".to_string(),
            },
            PluginCommand {
                trigger: "close illustrator".to_string(),
                description: "Closes Illustrator".to_string(),
                examples: vec!["close illustrator".to_string(), "quit illustrator".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:illustrator".to_string(),
            },
            PluginCommand {
                trigger: "close premiere".to_string(),
                description: "Closes Premiere".to_string(),
                examples: vec!["close premiere".to_string(), "quit premiere".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:premiere".to_string(),
            },
            // Free alternatives
            PluginCommand {
                trigger: "open gimp".to_string(),
                description: "Opens GIMP".to_string(),
                examples: vec!["open gimp".to_string(), "gimp".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:gimp".to_string(),
            },
            PluginCommand {
                trigger: "open blender".to_string(),
                description: "Opens Blender".to_string(),
                examples: vec!["open blender".to_string(), "blender".to_string(), "3d modeling".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:blender".to_string(),
            },
            PluginCommand {
                trigger: "open inkscape".to_string(),
                description: "Opens Inkscape".to_string(),
                examples: vec!["open inkscape".to_string(), "inkscape".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:inkscape".to_string(),
            },
            PluginCommand {
                trigger: "close gimp".to_string(),
                description: "Closes GIMP".to_string(),
                examples: vec!["close gimp".to_string(), "quit gimp".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:gimp".to_string(),
            },
            PluginCommand {
                trigger: "close blender".to_string(),
                description: "Closes Blender".to_string(),
                examples: vec!["close blender".to_string(), "quit blender".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:blender".to_string(),
            },
            // Web-based
            url_cmd!("open figma", "Opens Figma", &["open figma", "figma"], "https://www.figma.com"),
            url_cmd!("open canva", "Opens Canva", &["open canva", "canva"], "https://www.canva.com"),
        ],
    }
}
