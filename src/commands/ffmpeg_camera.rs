// src/commands/ffmpeg_camera.rs
// Voice command handler for FFmpeg-based camera

use crate::media::{
    CAMERA_STATE, open_camera, close_camera, take_photo, 
    start_recording, stop_recording
};

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Camera panel state - whether UI should be shown
#[derive(Clone, Debug, Default)]
pub struct CameraPanelState {
    pub is_open: bool,
}

pub static CAMERA_PANEL_STATE: Lazy<Arc<Mutex<CameraPanelState>>> =
    Lazy::new(|| Arc::new(Mutex::new(CameraPanelState::default())));

/// Handle camera voice commands
pub fn handle_camera_command(action: &str) -> Result<String, String> {
    match action {
        "start" | "open" | "enter" => {
            open_camera()?;
            // Open camera panel UI
            if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                state.is_open = true;
                println!("[CAMERA CMD] Panel state set to open: true");
            }
            Ok("Camera opened".to_string())
        }
        
        "stop" | "close" | "exit" => {
            close_camera();
            // Close camera panel UI
            if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                state.is_open = false;
            }
            Ok("Camera closed".to_string())
        }
        
        "photo" | "take_photo" | "capture" | "snapshot" => {
            if !CAMERA_STATE.is_open() {
                open_camera()?;
                // Open camera panel UI
                if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                    state.is_open = true;
                }
            }
            let _path = take_photo()?;
            // Better UX - short message instead of full path
            Ok("Photo captured".to_string())
        }
        
        "record" | "start_recording" | "video" => {
            if !CAMERA_STATE.is_open() {
                open_camera()?;
                // Open camera panel UI
                if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                    state.is_open = true;
                }
            }
            start_recording()?;
            Ok("Recording started".to_string())
        }
        
        "stop_recording" | "end_recording" => {
            let _path = stop_recording()?;
            // Better UX - short message instead of full path
            Ok("Video saved".to_string())
        }
        
        _ => Err(format!("Unknown camera action: {}", action))
    }
}
