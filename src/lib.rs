// src/lib.rs - IGRIS v3 library exports

// Allow dead code during development - many features are WIP
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use chrono::Timelike;

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

// Chat message types and global state for text chat UI
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: &str, content: &str) -> Self {
        Self { role: role.to_string(), content: content.to_string() }
    }
}

pub static CHAT_MESSAGES: once_cell::sync::Lazy<Arc<Mutex<Vec<ChatMessage>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(vec![
        ChatMessage::new("assistant", &time_aware_greeting()),
    ])));

pub fn add_chat_message(role: &str, content: &str) {
    if let Ok(mut msgs) = CHAT_MESSAGES.lock() {
        msgs.push(ChatMessage::new(role, content));
    }
}

pub fn clear_chat_history() {
    if let Ok(mut msgs) = CHAT_MESSAGES.lock() {
        msgs.clear();
        msgs.push(ChatMessage::new("assistant", &time_aware_greeting()));
    }
}

/// Return a greeting based on the current time of day.
pub fn time_aware_greeting() -> String {
    let hour = chrono::Local::now().hour();
    match hour {
        22..=23 | 0..=3 => "You're up late. I'm IGRIS. What do you need?".to_string(),
        4..=11 => "Good morning. I'm IGRIS. Early start today — what are we working on?".to_string(),
        12..=16 => "Good afternoon. I'm IGRIS. What do you need?".to_string(),
        _ => "Good evening. I'm IGRIS. What are we up to tonight?".to_string(),
    }
}

// Selected model for chat
pub static SELECTED_MODEL: once_cell::sync::Lazy<Arc<Mutex<String>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(
        std::env::var("NVIDIA_NIM_MODEL").unwrap_or_else(|_| "meta/llama-3.1-70b-instruct".to_string())
    )));

pub fn set_selected_model(model: &str) {
    if let Ok(mut m) = SELECTED_MODEL.lock() {
        *m = model.to_string();
    }
}

pub fn get_selected_model() -> String {
    SELECTED_MODEL.lock().map(|m| m.clone()).unwrap_or_default()
}

// Selected provider (nvidia, openai, groq, google)
pub static SELECTED_PROVIDER: once_cell::sync::Lazy<Arc<Mutex<String>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new("nvidia".to_string())));

pub fn set_selected_provider(provider: &str) {
    if let Ok(mut p) = SELECTED_PROVIDER.lock() {
        *p = provider.to_string();
    }
}

pub fn get_selected_provider() -> String {
    SELECTED_PROVIDER.lock().map(|p| p.clone()).unwrap_or_else(|_| "nvidia".to_string())
}

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


