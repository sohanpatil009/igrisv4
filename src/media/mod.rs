// src/media/mod.rs - Media capture module (camera, audio, video)
// Uses FFmpeg for reliable photo and video+audio recording

pub mod ffmpeg_camera;

// Re-export FFmpeg camera functions
pub use ffmpeg_camera::{
    CAMERA_STATE,
    is_ffmpeg_available,
    open_camera,
    close_camera,
    take_photo,
    start_recording,
    stop_recording,
    list_cameras,
};

/// Camera device information
#[derive(Debug, Clone)]
pub struct CameraDevice {
    pub index: usize,
    pub name: String,
    pub friendly_name: String,
}
