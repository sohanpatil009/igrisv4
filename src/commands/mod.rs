// src/commands/mod.rs - Command handler modules


pub mod app_utils;
pub mod system;
pub mod files;
pub mod web;
pub mod ffmpeg_camera;
pub mod about;
pub mod reminders;

// Re-exports for utilities (app launching now handled by plugin system)
pub use app_utils::{close_all_apps, list_running_apps, get_tracked_app_count};
pub use system::{process_system_command, is_system_command};
pub use files::{process_file_command, create_file, delete_file};
pub use web::{process_search_command, is_search_command, search_in_browser, search_and_read_results};
pub use ffmpeg_camera::handle_camera_command;
pub use about::{handle_about_command, is_about_command};
pub use reminders::{handle_alarm_command, handle_reminder_command};
