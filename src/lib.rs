// src/lib.rs - IGRIS v3 library exports

// Allow dead code during development - many features are WIP
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

// Configuration system
pub mod config;

// UI components
pub mod ui;

// Core voice processing modules
pub mod core;

// Natural language understanding
pub mod nlu;

// Command handlers
pub mod commands;

// Plugin system
pub mod plugins;

// Utility modules
pub mod utils;

// Platform abstraction
pub mod platform;
pub mod platform_utils;

// Setup system
pub mod setup_manager;

// Media capture (camera, video)
pub mod media;

// FastSwap file sharing integration
pub mod fastswap;

// Online mode (NVIDIA NIM API-based STT & Reasoning)
pub mod online;

// Desktop Ecosystem (cross-device clipboard, discovery, sync)
pub mod eco;

// Global state for search results UI (shared across modules)
#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub is_open: bool,
    pub is_searching: bool,
    pub query: String,
    pub results: Vec<SearchResultData>,
}

#[derive(Clone, Debug)]
pub struct SearchResultData {
    pub path: String,
    pub name: String,
    pub drive: String,
    pub score: u32,
    pub is_folder: bool,
}

pub static SEARCH_STATE: once_cell::sync::Lazy<Arc<Mutex<SearchState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(SearchState::default())));

// Reset flag - set by global hotkey to restart voice loop from wake word detection
pub static RESET_FLAG: AtomicBool = AtomicBool::new(false);

// Re-export camera panel state from commands module
pub use commands::ffmpeg_camera::{CameraPanelState, CAMERA_PANEL_STATE};

// Re-export commonly used types
pub use config::{CONFIG, AppConfig, Personality, Theme};
pub use core::{stt, tts, vad, audio_capture, wake_word};
pub use nlu::{engine, ner, context, sbert};
pub use commands::{system, files, web};
pub use utils::{hotkey, greetings, shared_memory};
pub use ui::{SettingsPanel, SettingsButton};
pub use media::{CameraDevice, open_camera, close_camera, take_photo, start_recording, stop_recording, list_cameras};


