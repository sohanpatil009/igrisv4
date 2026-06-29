// src/plugins/builtin/files.rs
// File operations plugin

use super::*;

/// Custom action type for file operations
pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "files".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "File and folder operations".to_string(),
            keywords: vec!["file", "folder", "create", "delete", "open", "search", "find"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Create operations
            cmd!("create file", "Creates a new file", &["create file", "make file", "new file", "create a file"], ActionType::CustomFunction, "file:create"),
            cmd!("create folder", "Creates a new folder", &["create folder", "make folder", "new folder", "create directory"], ActionType::CustomFunction, "folder:create"),
            // Delete operations
            cmd!("delete file", "Deletes a file", &["delete file", "remove file", "delete the file"], ActionType::CustomFunction, "file:delete"),
            cmd!("delete folder", "Deletes a folder", &["delete folder", "remove folder", "delete directory"], ActionType::CustomFunction, "folder:delete"),
            // Open operations
            cmd!("open folder", "Opens a folder in explorer", &["open folder", "show folder", "open directory", "browse folder"], ActionType::CustomFunction, "folder:open"),
            cmd!("open file", "Opens a file", &["open file", "open the file", "show file"], ActionType::CustomFunction, "file:open"),
            // Search operations
            cmd!("search files", "Searches for files", &["search files", "find files", "search for file", "look for file", "find file"], ActionType::CustomFunction, "file:search"),
            cmd!("search folders", "Searches for folders", &["search folders", "find folders", "search for folder", "find folder"], ActionType::CustomFunction, "folder:search"),
        ],
    }
}
