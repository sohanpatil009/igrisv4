// src/plugins/builtin/mod.rs
// Rust-based built-in plugins - compiled into binary for maximum speed

mod browsers;
mod utilities;
mod media;
mod office;
mod communication;
mod creative;
mod gaming;
mod editors;
mod camera;
mod files;
mod reminders;
mod system_control;

use super::{ActionType, Plugin, PluginCommand, PluginMetadata};

/// Get all built-in plugins (compiled into binary)
pub fn get_builtin_plugins() -> Vec<Plugin> {
    vec![
        browsers::plugin(),
        utilities::plugin(),
        media::plugin(),
        office::plugin(),
        communication::plugin(),
        creative::plugin(),
        gaming::plugin(),
        editors::plugin(),
        camera::plugin(),
        files::plugin(),
        reminders::plugin(),
        system_control::plugin(),
    ]
}

/// Helper macro to create commands quickly
#[macro_export]
macro_rules! cmd {
    ($trigger:expr, $desc:expr, $examples:expr, $action:expr, $data:expr) => {
        PluginCommand {
            trigger: $trigger.to_string(),
            description: $desc.to_string(),
            examples: $examples.iter().map(|s| s.to_string()).collect(),
            action_type: $action,
            action_data: $data.to_string(),
        }
    };
}

/// Helper macro to create shell command
#[macro_export]
macro_rules! shell_cmd {
    ($trigger:expr, $desc:expr, $examples:expr, $cmd:expr) => {
        cmd!($trigger, $desc, $examples, ActionType::ShellCommand, $cmd)
    };
}

/// Helper macro to create URL command
#[macro_export]
macro_rules! url_cmd {
    ($trigger:expr, $desc:expr, $examples:expr, $url:expr) => {
        cmd!($trigger, $desc, $examples, ActionType::OpenUrl, $url)
    };
}

pub use cmd;
pub use shell_cmd;
pub use url_cmd;
