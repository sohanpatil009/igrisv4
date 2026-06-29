// src/media/ffmpeg_camera/mod.rs
// FFmpeg-based camera for photo and video with audio recording
// Simple, reliable, UX-friendly approach

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::path::PathBuf;
use std::process::{Command, Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::io::{BufRead, BufReader};

/// Camera state
#[derive(Clone)]
pub struct CameraState {
    /// Is camera panel open
    pub is_open: Arc<AtomicBool>,
    /// Is recording video
    pub is_recording: Arc<AtomicBool>,
    /// FFmpeg process for recording
    recording_process: Arc<Mutex<Option<Child>>>,
    /// Current video file path
    pub current_video_path: Arc<Mutex<Option<String>>>,
    /// Last photo path
    pub last_photo_path: Arc<Mutex<Option<String>>>,
    /// Last video path
    pub last_video_path: Arc<Mutex<Option<String>>>,
    /// Recording start time
    pub recording_start: Arc<Mutex<Option<Instant>>>,
    /// Error message
    pub error: Arc<Mutex<Option<String>>>,
    /// Status message for UI
    pub status: Arc<Mutex<String>>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            is_open: Arc::new(AtomicBool::new(false)),
            is_recording: Arc::new(AtomicBool::new(false)),
            recording_process: Arc::new(Mutex::new(None)),
            current_video_path: Arc::new(Mutex::new(None)),
            last_photo_path: Arc::new(Mutex::new(None)),
            last_video_path: Arc::new(Mutex::new(None)),
            recording_start: Arc::new(Mutex::new(None)),
            error: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new("Ready".to_string())),
        }
    }
}

impl CameraState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }
    
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
    
    pub fn get_recording_duration(&self) -> Option<Duration> {
        let start = self.recording_start.lock().ok()?;
        start.map(|s| s.elapsed())
    }
    
    pub fn get_status(&self) -> String {
        self.status.lock().ok().map(|s| s.clone()).unwrap_or_default()
    }
    
    fn set_status(&self, msg: &str) {
        if let Ok(mut s) = self.status.lock() {
            *s = msg.to_string();
        }
    }
    
    fn set_error(&self, msg: &str) {
        if let Ok(mut e) = self.error.lock() {
            *e = Some(msg.to_string());
        }
        self.set_status(&format!("Error: {}", msg));
    }
}

lazy_static::lazy_static! {
    pub static ref CAMERA_STATE: CameraState = CameraState::new();
}

/// Get FFmpeg path
fn get_ffmpeg_path() -> Option<String> {
    // Check pkg/ffmpeg/bin first
    let paths = [
        "pkg/ffmpeg/bin/ffmpeg.exe",
        "pkg/ffmpeg/ffmpeg.exe",
        "pkg/ffmpeg/bin/ffmpeg",
        "pkg/ffmpeg/ffmpeg",
    ];
    
    for path in &paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    
    // Check system PATH
    let ffmpeg = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    if Command::new(ffmpeg).arg("-version").output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(ffmpeg.to_string());
    }
    
    None
}

/// Check if FFmpeg is available
pub fn is_ffmpeg_available() -> bool {
    get_ffmpeg_path().is_some()
}

/// Open camera (just sets state, actual capture happens on photo/video)
pub fn open_camera() -> Result<(), String> {
    if !is_ffmpeg_available() {
        return Err("FFmpeg not found. Please wait for setup to complete.".to_string());
    }
    
    CAMERA_STATE.is_open.store(true, Ordering::SeqCst);
    CAMERA_STATE.set_status("Camera ready");
    println!("[CAMERA] Opened");
    Ok(())
}

/// Close camera
pub fn close_camera() {
    // Stop recording if active
    if CAMERA_STATE.is_recording() {
        let _ = stop_recording();
    }
    
    CAMERA_STATE.is_open.store(false, Ordering::SeqCst);
    CAMERA_STATE.set_status("Camera closed");
    println!("[CAMERA] Closed");
}

/// Take a photo using FFmpeg
pub fn take_photo() -> Result<String, String> {
    if !CAMERA_STATE.is_open() {
        return Err("Camera not open".to_string());
    }
    
    let ffmpeg = get_ffmpeg_path().ok_or("FFmpeg not found")?;
    
    CAMERA_STATE.set_status("Taking photo...");
    
    let photos_dir = get_photos_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let photo_path = photos_dir.join(format!("photo_{}.jpg", timestamp));
    
    // Capture single frame from camera
    let args = get_photo_args(&photo_path);
    
    let output = Command::new(&ffmpeg)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run FFmpeg: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        CAMERA_STATE.set_error("Photo capture failed");
        return Err(format!("FFmpeg error: {}", stderr));
    }
    
    let path_str = photo_path.to_string_lossy().to_string();
    
    if let Ok(mut p) = CAMERA_STATE.last_photo_path.lock() {
        *p = Some(path_str.clone());
    }
    
    CAMERA_STATE.set_status(&format!("Photo saved!"));
    println!("[CAMERA] Photo saved: {}", path_str);
    
    Ok(path_str)
}

/// Start video recording with audio
pub fn start_recording() -> Result<(), String> {
    if !CAMERA_STATE.is_open() {
        return Err("Camera not open".to_string());
    }
    
    if CAMERA_STATE.is_recording() {
        return Err("Already recording".to_string());
    }
    
    let ffmpeg = get_ffmpeg_path().ok_or("FFmpeg not found")?;
    
    let videos_dir = get_videos_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let video_path = videos_dir.join(format!("video_{}.mp4", timestamp));
    
    // Store video path
    if let Ok(mut p) = CAMERA_STATE.current_video_path.lock() {
        *p = Some(video_path.to_string_lossy().to_string());
    }
    
    // Build FFmpeg command for video + audio recording
    let args = get_recording_args(&video_path);
    
    let child = Command::new(&ffmpeg)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start recording: {}", e))?;
    
    // Store process
    if let Ok(mut proc) = CAMERA_STATE.recording_process.lock() {
        *proc = Some(child);
    }
    
    // Set recording state
    if let Ok(mut start) = CAMERA_STATE.recording_start.lock() {
        *start = Some(Instant::now());
    }
    
    CAMERA_STATE.is_recording.store(true, Ordering::SeqCst);
    CAMERA_STATE.set_status("Recording...");
    
    println!("[CAMERA] Recording started: {}", video_path.display());
    Ok(())
}

/// Stop video recording
pub fn stop_recording() -> Result<String, String> {
    if !CAMERA_STATE.is_recording() {
        return Err("Not recording".to_string());
    }
    
    CAMERA_STATE.set_status("Saving video...");
    CAMERA_STATE.is_recording.store(false, Ordering::SeqCst);
    
    // Send 'q' to FFmpeg to stop gracefully
    if let Ok(mut proc_guard) = CAMERA_STATE.recording_process.lock() {
        if let Some(mut child) = proc_guard.take() {
            // Send quit signal
            if let Some(ref mut stdin) = child.stdin {
                use std::io::Write;
                let _ = stdin.write_all(b"q");
            }
            
            // Wait for process to finish (max 5 seconds)
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if start.elapsed() > Duration::from_secs(5) {
                            let _ = child.kill();
                            break;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => break,
                }
            }
        }
    }
    
    // Get video path
    let video_path = CAMERA_STATE.current_video_path.lock()
        .ok()
        .and_then(|p| p.clone())
        .ok_or("No video path")?;
    
    // Store as last video
    if let Ok(mut p) = CAMERA_STATE.last_video_path.lock() {
        *p = Some(video_path.clone());
    }
    
    CAMERA_STATE.set_status("Video saved!");
    println!("[CAMERA] Recording stopped: {}", video_path);
    
    Ok(video_path)
}

/// Get FFmpeg arguments for photo capture (platform-specific)
fn get_photo_args(output_path: &PathBuf) -> Vec<String> {
    let output = output_path.to_string_lossy().to_string();
    
    if cfg!(target_os = "windows") {
        // Get first available camera
        let camera = get_default_camera();
        vec![
            "-f".to_string(), "dshow".to_string(),
            "-i".to_string(), format!("video={}", camera),
            "-frames:v".to_string(), "1".to_string(),
            "-q:v".to_string(), "2".to_string(),
            "-y".to_string(),
            output,
        ]
    } else if cfg!(target_os = "macos") {
        // macOS: Use avfoundation with video device only (no audio for photos)
        vec![
            "-f".to_string(), "avfoundation".to_string(),
            "-video_size".to_string(), "1280x720".to_string(),
            "-framerate".to_string(), "30".to_string(),
            "-i".to_string(), "0:none".to_string(), // video device 0, no audio
            "-frames:v".to_string(), "1".to_string(),
            "-q:v".to_string(), "2".to_string(),
            "-y".to_string(),
            output,
        ]
    } else {
        // Linux
        vec![
            "-f".to_string(), "v4l2".to_string(),
            "-i".to_string(), "/dev/video0".to_string(),
            "-frames:v".to_string(), "1".to_string(),
            "-q:v".to_string(), "2".to_string(),
            "-y".to_string(),
            output,
        ]
    }
}

/// Get FFmpeg arguments for video+audio recording (platform-specific)
fn get_recording_args(output_path: &PathBuf) -> Vec<String> {
    let output = output_path.to_string_lossy().to_string();
    
    if cfg!(target_os = "windows") {
        // Windows: dshow for video and audio
        let camera = get_default_camera();
        let mic = get_default_microphone();
        
        vec![
            "-f".to_string(), "dshow".to_string(),
            "-i".to_string(), format!("video={}:audio={}", camera, mic),
            "-c:v".to_string(), "libx264".to_string(),
            "-preset".to_string(), "ultrafast".to_string(),
            "-crf".to_string(), "23".to_string(),
            "-c:a".to_string(), "aac".to_string(),
            "-b:a".to_string(), "128k".to_string(),
            "-y".to_string(),
            output,
        ]
    } else if cfg!(target_os = "macos") {
        // macOS: avfoundation with proper device indices
        // Device 0 = FaceTime HD Camera, Device 0 = MacBook Air Microphone
        vec![
            "-f".to_string(), "avfoundation".to_string(),
            "-video_size".to_string(), "1280x720".to_string(),
            "-framerate".to_string(), "30".to_string(),
            "-i".to_string(), "0:0".to_string(), // video device 0, audio device 0
            "-c:v".to_string(), "libx264".to_string(),
            "-preset".to_string(), "ultrafast".to_string(),
            "-crf".to_string(), "23".to_string(),
            "-c:a".to_string(), "aac".to_string(),
            "-b:a".to_string(), "128k".to_string(),
            "-y".to_string(),
            output,
        ]
    } else {
        // Linux: v4l2 + pulse/alsa
        vec![
            "-f".to_string(), "v4l2".to_string(),
            "-i".to_string(), "/dev/video0".to_string(),
            "-f".to_string(), "pulse".to_string(),
            "-i".to_string(), "default".to_string(),
            "-c:v".to_string(), "libx264".to_string(),
            "-preset".to_string(), "ultrafast".to_string(),
            "-crf".to_string(), "23".to_string(),
            "-c:a".to_string(), "aac".to_string(),
            "-b:a".to_string(), "128k".to_string(),
            "-y".to_string(),
            output,
        ]
    }
}

/// Get photos directory
fn get_photos_dir() -> PathBuf {
    let dir = dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("IGRIS");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Get videos directory
fn get_videos_dir() -> PathBuf {
    let dir = dirs::video_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("IGRIS");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Get default camera name (Windows dshow)
fn get_default_camera() -> String {
    let cameras = list_cameras();
    cameras.first().cloned().unwrap_or_else(|| "0".to_string())
}

/// Get default microphone name (Windows dshow)
fn get_default_microphone() -> String {
    let ffmpeg = match get_ffmpeg_path() {
        Some(p) => p,
        None => return "Default".to_string(),
    };
    
    if cfg!(target_os = "windows") {
        let output = Command::new(&ffmpeg)
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .output();
        
        if let Ok(out) = output {
            let stderr = String::from_utf8_lossy(&out.stderr);
            
            for line in stderr.lines() {
                if line.contains("(audio)") {
                    // Extract device name
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start+1..].find('"') {
                            return line[start+1..start+1+end].to_string();
                        }
                    }
                }
            }
        }
    }
    
    "Default".to_string()
}

/// List available cameras using FFmpeg
pub fn list_cameras() -> Vec<String> {
    let ffmpeg = match get_ffmpeg_path() {
        Some(p) => p,
        None => return vec!["Default Camera".to_string()],
    };
    
    if cfg!(target_os = "windows") {
        // List dshow devices
        let output = Command::new(&ffmpeg)
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .stderr(Stdio::piped())
            .output();
        
        if let Ok(out) = output {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let mut cameras = Vec::new();
            
            for line in stderr.lines() {
                if line.contains("(video)") {
                    // Extract device name between quotes
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start+1..].find('"') {
                            let name = line[start+1..start+1+end].to_string();
                            cameras.push(name);
                        }
                    }
                }
            }
            
            if !cameras.is_empty() {
                println!("[CAMERA] Found {} camera(s): {:?}", cameras.len(), cameras);
                return cameras;
            }
        }
    }
    
    vec!["Default Camera".to_string()]
}
