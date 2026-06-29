// src/plugins/builtin/media.rs
// Media applications plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "media".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Media and entertainment apps".to_string(),
            keywords: vec!["media", "music", "video", "spotify", "vlc", "netflix", "youtube"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Music/Audio - Open commands
            PluginCommand {
                trigger: "open spotify".to_string(),
                description: "Opens Spotify".to_string(),
                examples: vec!["open spotify".to_string(), "spotify".to_string(), "play music".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:spotify".to_string(),
            },
            PluginCommand {
                trigger: "close spotify".to_string(),
                description: "Closes Spotify".to_string(),
                examples: vec!["close spotify".to_string(), "quit spotify".to_string(), "stop music".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:spotify".to_string(),
            },
            // Video players
            PluginCommand {
                trigger: "open vlc".to_string(),
                description: "Opens VLC Media Player".to_string(),
                examples: vec!["open vlc".to_string(), "vlc".to_string(), "media player".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "open_app:vlc".to_string(),
            },
            PluginCommand {
                trigger: "close vlc".to_string(),
                description: "Closes VLC".to_string(),
                examples: vec!["close vlc".to_string(), "quit vlc".to_string()],
                action_type: ActionType::CustomFunction,
                action_data: "close_app:vlc".to_string(),
            },
            // Streaming (web-based)
            url_cmd!("open youtube", "Opens YouTube", &["open youtube", "youtube", "watch videos"], "https://www.youtube.com"),
            url_cmd!("open netflix", "Opens Netflix", &["open netflix", "netflix", "watch netflix"], "https://www.netflix.com"),
            url_cmd!("open prime video", "Opens Amazon Prime Video", &["open prime video", "prime video", "amazon prime"], "https://www.primevideo.com"),
            url_cmd!("open disney plus", "Opens Disney+", &["open disney plus", "disney plus", "disney+"], "https://www.disneyplus.com"),
            url_cmd!("open twitch", "Opens Twitch", &["open twitch", "twitch", "watch streams"], "https://www.twitch.tv"),
        ],
    }
}
