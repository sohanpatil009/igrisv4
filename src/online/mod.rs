// src/online/mod.rs - Online mode modules using NVIDIA NIM APIs

pub mod reasoning;
pub mod stt;
pub mod task_planner;
pub mod intent_router;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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
    match std::env::var("IGRIS_ONLINE_MODE") {
        Ok(val) if val == "true" || val == "1" => {
            println!("[Online] IGRIS_ONLINE_MODE=true found in env — enabling online mode");
            enable_online_mode();
        }
        Ok(val) => {
            println!("[Online] IGRIS_ONLINE_MODE={} — staying offline", val);
        }
        Err(_) => {
            println!("[Online] IGRIS_ONLINE_MODE not set — staying offline");
        }
    }
}

/// Check if the device has internet connectivity by trying to connect
/// to several well-known hosts with a short timeout.
pub async fn check_internet_connectivity() -> bool {
    let hosts = ["1.1.1.1:443", "8.8.8.8:443", "google.com:443"];
    for host in &hosts {
        if tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(host))
            .await
            .is_ok()
        {
            return true;
        }
    }
    false
}

pub use reasoning::{reason_online, OnlineReasoning};
pub use stt::{transcribe_online, OnlineStt};