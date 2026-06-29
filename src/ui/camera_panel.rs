// src/ui/camera_panel.rs
// Camera UI panel for FFmpeg-based camera

use dioxus::prelude::*;
use crate::media::{CAMERA_STATE, open_camera, close_camera, take_photo, start_recording, stop_recording};

/// Camera panel component
#[component]
pub fn CameraPanel(on_close: EventHandler<()>) -> Element {
    let mut is_recording = use_signal(|| false);
    let mut status_message = use_signal(|| "Camera ready".to_string());
    let mut recording_time = use_signal(|| 0u64);
    let mut last_action_result = use_signal(|| None::<String>);
    let mut last_photo_path = use_signal(|| None::<String>);
    let mut last_video_path = use_signal(|| None::<String>);
    let mut show_preview = use_signal(|| false);
    
    // Update loop - sync state from camera
    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(500)).await;
                
                is_recording.set(CAMERA_STATE.is_recording());
                status_message.set(CAMERA_STATE.get_status());
                
                if let Some(duration) = CAMERA_STATE.get_recording_duration() {
                    recording_time.set(duration.as_secs());
                }
                
                // Update last photo/video paths
                if let Ok(photo) = CAMERA_STATE.last_photo_path.lock() {
                    if photo.is_some() && *photo != last_photo_path() {
                        last_photo_path.set(photo.clone());
                        show_preview.set(true);
                    }
                }
                
                if let Ok(video) = CAMERA_STATE.last_video_path.lock() {
                    if video.is_some() && *video != last_video_path() {
                        last_video_path.set(video.clone());
                        show_preview.set(true);
                    }
                }
                
                // Exit if camera closed externally
                if !CAMERA_STATE.is_open() {
                    break;
                }
            }
        });
    });
    
    let handle_close = move |_| {
        close_camera();
        on_close.call(());
    };
    
    let handle_photo = move |_| {
        status_message.set("Taking photo...".to_string());
        show_preview.set(false);
        match take_photo() {
            Ok(path) => {
                last_action_result.set(Some("📸 Photo captured!".to_string()));
                status_message.set("Photo captured!".to_string());
                last_photo_path.set(Some(path));
                show_preview.set(true);
            }
            Err(e) => {
                last_action_result.set(Some(format!("❌ {}", e)));
                status_message.set(format!("Error: {}", e));
            }
        }
    };
    
    let handle_record = move |_| {
        if is_recording() {
            // Stop recording
            status_message.set("Saving video...".to_string());
            show_preview.set(false);
            match stop_recording() {
                Ok(path) => {
                    last_action_result.set(Some("🎬 Video saved!".to_string()));
                    status_message.set("Video saved!".to_string());
                    last_video_path.set(Some(path));
                    show_preview.set(true);
                }
                Err(e) => {
                    last_action_result.set(Some(format!("❌ {}", e)));
                    status_message.set(format!("Error: {}", e));
                }
            }
        } else {
            // Start recording
            show_preview.set(false);
            match start_recording() {
                Ok(_) => {
                    last_action_result.set(Some("🔴 Recording started".to_string()));
                    status_message.set("Recording...".to_string());
                }
                Err(e) => {
                    last_action_result.set(Some(format!("❌ {}", e)));
                    status_message.set(format!("Error: {}", e));
                }
            }
        }
    };
    
    let recording = is_recording();
    let rec_time = recording_time();
    let status = status_message();
    let result = last_action_result();
    let preview = show_preview();
    
    // Format recording time
    let time_str = format!("{}:{:02}", rec_time / 60, rec_time % 60);
    
    // Handler to open file in system viewer
    let open_photo = move |_| {
        if let Ok(photo) = CAMERA_STATE.last_photo_path.lock() {
            if let Some(ref path) = *photo {
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", path])
                        .spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open")
                        .arg(path)
                        .spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("xdg-open")
                        .arg(path)
                        .spawn();
                }
            }
        }
    };
    
    let open_video = move |_| {
        if let Ok(video) = CAMERA_STATE.last_video_path.lock() {
            if let Some(ref path) = *video {
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", path])
                        .spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open")
                        .arg(path)
                        .spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("xdg-open")
                        .arg(path)
                        .spawn();
                }
            }
        }
    };
    
    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.95); z-index: 1000; display: flex; flex-direction: column; align-items: center; justify-content: center;",
            
            // Header
            div {
                style: "position: absolute; top: 20px; left: 20px; right: 20px; display: flex; justify-content: space-between; align-items: center;",
                
                div {
                    style: "color: white; font-size: 18px; font-weight: bold;",
                    "📷 IGRIS Camera"
                }
                
                // Recording indicator
                if recording {
                    div {
                        style: "display: flex; align-items: center; gap: 8px; color: #ef4444; font-size: 18px; font-weight: bold;",
                        div {
                            style: "width: 14px; height: 14px; background: #ef4444; border-radius: 50%; animation: pulse 1s infinite;",
                        }
                        "REC {time_str}"
                    }
                }
                
                // Close button
                button {
                    style: "background: rgba(255,255,255,0.1); border: none; color: white; padding: 10px 20px; border-radius: 8px; cursor: pointer; font-size: 14px;",
                    onclick: handle_close,
                    "✕ Close"
                }
            }
            
            // Main content area
            div {
                style: "display: flex; flex-direction: column; align-items: center; gap: 30px; max-width: 90%; max-height: 80%;",
                
                // Preview area - show last captured photo or video info
                if preview {
                    if let Some(path) = last_photo_path() {
                        div {
                            style: "display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 30px; background: rgba(168, 85, 247, 0.1); border-radius: 16px; border: 2px solid #a855f7; box-shadow: 0 8px 32px rgba(168, 85, 247, 0.3);",
                            
                            // Photo icon
                            div {
                                style: "font-size: 80px;",
                                "📸"
                            }
                            
                            // Success message
                            div {
                                style: "color: #a855f7; font-size: 20px; font-weight: bold;",
                                "Photo Captured!"
                            }
                            
                            // File name
                            div {
                                style: "color: #9ca3af; font-size: 14px; text-align: center; max-width: 400px; word-break: break-all;",
                                {
                                    let filename = std::path::Path::new(&path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("photo.jpg");
                                    filename
                                }
                            }
                            
                            // Open button
                            button {
                                style: "background: #a855f7; border: none; color: white; padding: 12px 24px; border-radius: 8px; cursor: pointer; font-size: 16px; margin-top: 10px; transition: all 0.2s;",
                                onclick: open_photo,
                                "🖼️ View Photo"
                            }
                        }
                    } else if let Some(path) = last_video_path() {
                        div {
                            style: "display: flex; flex-direction: column; align-items: center; gap: 15px; padding: 30px; background: rgba(168, 85, 247, 0.1); border-radius: 16px; border: 2px solid #a855f7; box-shadow: 0 8px 32px rgba(168, 85, 247, 0.3);",
                            
                            // Video icon
                            div {
                                style: "font-size: 80px;",
                                "🎬"
                            }
                            
                            // Success message
                            div {
                                style: "color: #a855f7; font-size: 20px; font-weight: bold;",
                                "Video Saved!"
                            }
                            
                            // File name
                            div {
                                style: "color: #9ca3af; font-size: 14px; text-align: center; max-width: 400px; word-break: break-all;",
                                {
                                    let filename = std::path::Path::new(&path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("video.mp4");
                                    filename
                                }
                            }
                            
                            // Open button
                            button {
                                style: "background: #a855f7; border: none; color: white; padding: 12px 24px; border-radius: 8px; cursor: pointer; font-size: 16px; margin-top: 10px; transition: all 0.2s;",
                                onclick: open_video,
                                "▶️ Play Video"
                            }
                        }
                    }
                } else {
                    // Camera icon when no preview
                    div {
                        style: "font-size: 120px; opacity: 0.8;",
                        if recording {
                            "🎥"
                        } else {
                            "📷"
                        }
                    }
                }
                
                // Status message
                div {
                    style: "color: #9ca3af; font-size: 18px; text-align: center;",
                    "{status}"
                }
                
                // Last action result
                if let Some(res) = result {
                    div {
                        style: "color: #22c55e; font-size: 16px; padding: 10px 20px; background: rgba(34, 197, 94, 0.1); border-radius: 8px;",
                        "{res}"
                    }
                }
                
                // Controls
                div {
                    style: "display: flex; gap: 20px; margin-top: 20px;",
                    
                    // Photo button
                    button {
                        style: "background: #3b82f6; border: none; color: white; padding: 16px 32px; border-radius: 50px; cursor: pointer; font-size: 18px; display: flex; align-items: center; gap: 10px; transition: transform 0.1s;",
                        onclick: handle_photo,
                        disabled: recording,
                        "📸 Take Photo"
                    }
                    
                    // Record button
                    button {
                        style: if recording { 
                            "background: #ef4444; border: none; color: white; padding: 16px 32px; border-radius: 50px; cursor: pointer; font-size: 18px; display: flex; align-items: center; gap: 10px; animation: pulse 1s infinite;"
                        } else {
                            "background: #22c55e; border: none; color: white; padding: 16px 32px; border-radius: 50px; cursor: pointer; font-size: 18px; display: flex; align-items: center; gap: 10px;"
                        },
                        onclick: handle_record,
                        if recording {
                            "⏹ Stop Recording"
                        } else {
                            "🔴 Start Recording"
                        }
                    }
                }
            }
            
            // Voice commands hint
            div {
                style: "position: absolute; bottom: 30px; color: #6b7280; font-size: 14px; text-align: center;",
                "Voice commands: \"take photo\" • \"start recording\" • \"stop recording\" • \"exit camera\""
            }
            
            // Pulse animation
            style {
                r#"
                @keyframes pulse {{
                    0%, 100% {{ opacity: 1; }}
                    50% {{ opacity: 0.5; }}
                }}
                "#
            }
        }
    }
}
