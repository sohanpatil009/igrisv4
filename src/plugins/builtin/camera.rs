// src/plugins/builtin/camera.rs
// Camera plugin (FFmpeg-based)

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "camera".to_string(),
            version: "2.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "FFmpeg-based camera with photo & video recording".to_string(),
            keywords: vec!["camera", "photo", "video", "recording", "webcam", "selfie", "capture"]
                .into_iter().map(String::from).collect(),
            enabled: true,
        },
        commands: vec![
            // Camera control - uses FFmpeg
            cmd!("open camera", "Opens camera", &["open camera", "start camera", "camera on", "show camera"], ActionType::CameraMode, "ffmpeg_start"),
            cmd!("close camera", "Closes camera", &["close camera", "stop camera", "exit camera", "camera off", "hide camera"], ActionType::CameraMode, "ffmpeg_stop"),
            cmd!("take photo", "Takes a photo", &["take photo", "capture photo", "take picture", "snap", "click photo", "selfie"], ActionType::CameraMode, "ffmpeg_photo"),
            cmd!("start recording", "Starts video recording", &["start recording", "record video", "begin recording", "start video"], ActionType::CameraMode, "ffmpeg_record"),
            cmd!("stop recording", "Stops video recording and saves", &["stop recording", "end recording", "finish recording", "stop video"], ActionType::CameraMode, "ffmpeg_stop_recording"),
            cmd!("list cameras", "Lists available cameras", &["list cameras", "show cameras", "available cameras"], ActionType::CameraMode, "ffmpeg_list"),
        ],
    }
}
