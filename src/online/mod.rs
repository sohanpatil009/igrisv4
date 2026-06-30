// src/online/mod.rs - Online mode modules using NVIDIA NIM APIs

pub mod reasoning;
pub mod stt;

use std::sync::atomic::{AtomicBool, Ordering};

static ONLINE_MODE: AtomicBool = AtomicBool::new(false);

/// Check if online mode is enabled
pub fn is_online_mode() -> bool {
    ONLINE_MODE.load(Ordering::Relaxed)
}

/// Enable online mode (STT + Reasoning via NVIDIA NIM)
pub fn enable_online_mode() {
    ONLINE_MODE.store(true, Ordering::Relaxed);
    println!("[Online] Online mode ENABLED - STT & Reasoning via NVIDIA NIM");
}

/// Disable online mode (fallback to local models)
pub fn disable_online_mode() {
    ONLINE_MODE.store(false, Ordering::Relaxed);
    println!("[Online] Online mode DISABLED - Using local models");
}

/// Toggle online mode
pub fn toggle_online_mode() -> bool {
    let new_state = !ONLINE_MODE.load(Ordering::Relaxed);
    ONLINE_MODE.store(new_state, Ordering::Relaxed);
    if new_state {
        enable_online_mode();
    } else {
        disable_online_mode();
    }
    new_state
}

/// Initialize online mode from environment variable
pub fn init_from_env() {
    if let Ok(val) = std::env::var("IGRIS_ONLINE_MODE") {
        if val == "true" || val == "1" {
            enable_online_mode();
        }
    }
}

pub use reasoning::{reason_online, OnlineReasoning};
pub use stt::{transcribe_online, OnlineStt};