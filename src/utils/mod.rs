// src/utils/mod.rs - Utility modules

pub mod hotkey;
pub mod greetings;
pub mod shared_memory;
pub mod process_tracker;

// Re-exports
pub use hotkey::register_global_hotkey;
pub use greetings::{messages, speak_invoke_greeting, speak_wake_response, speak_goodbye, contains_wake_word};
pub use shared_memory::init_shared_memory;
pub use process_tracker::{
    ProcessCategory, track_process,
    close_all_apps, close_all_camera, close_all_processes,
    get_process_count, get_tracked_app_names, get_total_tracked_count, get_tracked_app_count,
    PROCESS_TRACKER,
    track_site, find_browser_for_site, close_site, is_tracked_site,
    extract_site_name, get_tracked_sites, OPENED_SITES,
};
