// src/utils/shared_memory.rs - High-performance shared memory management
// Optimized for fast response times with pre-warmed pools, caching, and priority queues

use crate::platform::process_builder::ProcessBuilderExt;
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Semaphore};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "windows")]
pub const FFMPEG_PATH: &str = "./pkg/ffmpeg/bin/ffmpeg.exe";
#[cfg(target_os = "windows")]
pub const FFPLAY_PATH: &str = "./pkg/ffmpeg/bin/ffplay.exe";
#[cfg(target_os = "windows")]
pub const PIPER_PATH: &str = "./pkg/piper/piper.exe";

#[cfg(not(target_os = "windows"))]
pub const FFMPEG_PATH: &str = "./pkg/ffmpeg/bin/ffmpeg";
#[cfg(not(target_os = "windows"))]
pub const FFPLAY_PATH: &str = "./pkg/ffmpeg/bin/ffplay";
#[cfg(not(target_os = "windows"))]
pub const PIPER_PATH: &str = "./pkg/piper/piper";

// Pool configuration - Optimized for fast startup
const MAX_CONCURRENT_COMMANDS: usize = 4;
const MAX_CONCURRENT_TTS: usize = 2;
const MAX_CONCURRENT_FFMPEG: usize = 2;
const CACHE_TTL_SECS: u64 = 300; // 5 minutes
const MAX_CACHE_ENTRIES: usize = 50; // Reduced for faster lookups

/// Single TTS output file (gets overwritten each synthesis)
const TTS_OUTPUT_FILE: &str = "./pkg/audio/tts_output.wav";

// Global instance
pub static SHARED_MEMORY: Lazy<Arc<SharedMemory>> = Lazy::new(|| Arc::new(SharedMemory::new()));

// ═══════════════════════════════════════════════════════════════════════════════
// CACHE SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct CacheEntry<T: Clone> {
    value: T,
    created_at: Instant,
    hits: u32,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            hits: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(CACHE_TTL_SECS)
    }
}

struct ResponseCache<T: Clone> {
    entries: HashMap<String, CacheEntry<T>>,
    max_entries: usize,
}

impl<T: Clone> ResponseCache<T> {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.entries.get_mut(key) {
            if !entry.is_expired() {
                entry.hits += 1;
                return Some(entry.value.clone());
            }
            // Remove expired entry
            self.entries.remove(key);
        }
        None
    }

    fn insert(&mut self, key: String, value: T) {
        // Evict oldest entries if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }
        self.entries.insert(key, CacheEntry::new(value));
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.created_at)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
    }

    fn clear_expired(&mut self) {
        self.entries.retain(|_, v| !v.is_expired());
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// REQUEST/RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
pub enum Priority {
    High,   // Wake word, urgent commands
    Normal, // Regular commands
    Low,    // Background tasks
}

#[derive(Debug)]
pub struct CmdRequest {
    pub command: String,
    pub args: Vec<String>,
    pub priority: Priority,
    pub response_tx: oneshot::Sender<CmdResponse>,
}

#[derive(Clone, Debug)]
pub struct CmdResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub struct TtsRequest {
    pub text: String,
    pub output_path: String,
    pub speaker_id: String,
    pub priority: Priority,
    pub response_tx: oneshot::Sender<TtsResponse>,
}

#[derive(Clone, Debug)]
pub struct TtsResponse {
    pub success: bool,
    pub output_path: String,
    pub duration_ms: u64,
    pub cached: bool,
}

#[derive(Debug)]
pub struct FFmpegRequest {
    pub args: Vec<String>,
    pub priority: Priority,
    pub response_tx: oneshot::Sender<FFmpegResponse>,
}

#[derive(Clone, Debug)]
pub struct FFmpegResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// THREAD POOL STATUS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
pub enum PoolStatus {
    Idle,
    Active(usize), // Number of active tasks
    Warming,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct PoolStats {
    pub status: PoolStatus,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub avg_response_ms: f64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SHARED MEMORY - MAIN STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════════

pub struct SharedMemory {
    // Semaphores for concurrency control
    cmd_semaphore: Arc<Semaphore>,
    tts_semaphore: Arc<Semaphore>,
    ffmpeg_semaphore: Arc<Semaphore>,

    // Request channels
    cmd_tx: Mutex<Option<mpsc::UnboundedSender<CmdRequest>>>,
    tts_tx: Mutex<Option<mpsc::UnboundedSender<TtsRequest>>>,
    ffmpeg_tx: Mutex<Option<mpsc::UnboundedSender<FFmpegRequest>>>,

    // Response caches
    tts_cache: Mutex<ResponseCache<String>>, // text hash -> audio path
    cmd_cache: Mutex<ResponseCache<CmdResponse>>,

    // Cached paths (resolved once)
    ffmpeg_path: RwLock<String>,
    ffplay_path: RwLock<String>,
    piper_path: RwLock<String>,

    // Active processes
    active_processes: Mutex<Vec<u32>>,

    // Statistics
    stats: RwLock<SharedMemoryStats>,

    // Initialization state
    initialized: RwLock<bool>,
    warmed_up: RwLock<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct SharedMemoryStats {
    pub cmd_requests: u64,
    pub cmd_cache_hits: u64,
    pub tts_requests: u64,
    pub tts_cache_hits: u64,
    pub ffmpeg_requests: u64,
    pub total_response_time_ms: u64,
    pub request_count: u64,
}

impl SharedMemoryStats {
    pub fn avg_response_ms(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            self.total_response_time_ms as f64 / self.request_count as f64
        }
    }
}

impl SharedMemory {
    pub fn new() -> Self {
        Self {
            cmd_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_COMMANDS)),
            tts_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_TTS)),
            ffmpeg_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_FFMPEG)),

            cmd_tx: Mutex::new(None),
            tts_tx: Mutex::new(None),
            ffmpeg_tx: Mutex::new(None),

            tts_cache: Mutex::new(ResponseCache::new(MAX_CACHE_ENTRIES)),
            cmd_cache: Mutex::new(ResponseCache::new(MAX_CACHE_ENTRIES)),

            ffmpeg_path: RwLock::new(resolve_ffmpeg_path()),
            ffplay_path: RwLock::new(resolve_ffplay_path()),
            piper_path: RwLock::new(resolve_piper_path()),

            active_processes: Mutex::new(Vec::new()),
            stats: RwLock::new(SharedMemoryStats::default()),
            initialized: RwLock::new(false),
            warmed_up: RwLock::new(false),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // INITIALIZATION
    // ═══════════════════════════════════════════════════════════════════════════

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if *self.initialized.read().unwrap() {
            return Ok(());
        }

        println!("[SHARED_MEMORY] Initializing thread pools...");

        self.start_cmd_pool().await?;
        self.start_tts_pool().await?;
        self.start_ffmpeg_pool().await?;

        *self.initialized.write().unwrap() = true;
        println!("[SHARED_MEMORY] Thread pools initialized");

        Ok(())
    }

    /// Warm up the pools by pre-executing lightweight tasks
    pub async fn warm_up(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if *self.warmed_up.read().unwrap() {
            return Ok(());
        }

        println!("[SHARED_MEMORY] Warming up pools...");

        // Warm up CMD pool with a simple command
        #[cfg(target_os = "windows")]
        let _ = self.execute_cmd_internal("cmd", vec!["/c".into(), "echo".into(), "warmup".into()], Priority::Low).await;
        
        #[cfg(not(target_os = "windows"))]
        let _ = self.execute_cmd_internal("echo", vec!["warmup".into()], Priority::Low).await;

        *self.warmed_up.write().unwrap() = true;
        println!("[SHARED_MEMORY] Pools warmed up");

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // POOL STARTERS
    // ═══════════════════════════════════════════════════════════════════════════

    async fn start_cmd_pool(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::unbounded_channel::<CmdRequest>();
        let semaphore = self.cmd_semaphore.clone();

        *self.cmd_tx.lock().unwrap() = Some(tx);

        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                let sem = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let start = Instant::now();
                    let response = execute_cmd_sync(&request.command, &request.args);
                    let duration = start.elapsed().as_millis() as u64;
                    let _ = request.response_tx.send(CmdResponse {
                        success: response.success,
                        stdout: response.stdout,
                        stderr: response.stderr,
                        exit_code: response.exit_code,
                        duration_ms: duration,
                    });
                });
            }
        });

        Ok(())
    }

    async fn start_tts_pool(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::unbounded_channel::<TtsRequest>();
        let semaphore = self.tts_semaphore.clone();
        let piper_path = self.piper_path.read().unwrap().clone();

        *self.tts_tx.lock().unwrap() = Some(tx);

        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                let sem = semaphore.clone();
                let piper = piper_path.clone();
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let start = Instant::now();
                    let success = execute_piper_sync(&piper, &request.text, &request.output_path, &request.speaker_id);
                    let duration = start.elapsed().as_millis() as u64;
                    let _ = request.response_tx.send(TtsResponse {
                        success,
                        output_path: request.output_path,
                        duration_ms: duration,
                        cached: false,
                    });
                });
            }
        });

        Ok(())
    }

    async fn start_ffmpeg_pool(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::unbounded_channel::<FFmpegRequest>();
        let semaphore = self.ffmpeg_semaphore.clone();
        let ffmpeg_path = self.ffmpeg_path.read().unwrap().clone();

        *self.ffmpeg_tx.lock().unwrap() = Some(tx);

        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                let sem = semaphore.clone();
                let ffmpeg = ffmpeg_path.clone();
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let start = Instant::now();
                    let response = execute_ffmpeg_sync(&ffmpeg, &request.args);
                    let duration = start.elapsed().as_millis() as u64;
                    let _ = request.response_tx.send(FFmpegResponse {
                        success: response.0,
                        stdout: response.1,
                        stderr: response.2,
                        duration_ms: duration,
                    });
                });
            }
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PUBLIC API - COMMAND EXECUTION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Execute a command with normal priority
    pub async fn execute_cmd(&self, command: &str, args: Vec<String>) -> Result<CmdResponse, String> {
        self.execute_cmd_internal(command, args, Priority::Normal).await
    }

    /// Execute a command with high priority (for urgent tasks)
    pub async fn execute_cmd_urgent(&self, command: &str, args: Vec<String>) -> Result<CmdResponse, String> {
        self.execute_cmd_internal(command, args, Priority::High).await
    }

    async fn execute_cmd_internal(&self, command: &str, args: Vec<String>, priority: Priority) -> Result<CmdResponse, String> {
        // Check cache first
        let cache_key = format!("{}:{}", command, args.join(":"));
        {
            let mut cache = self.cmd_cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                self.record_cache_hit(true);
                return Ok(cached);
            }
        }

        let tx = self.cmd_tx.lock().unwrap().clone().ok_or("CMD pool not initialized")?;
        let (response_tx, response_rx) = oneshot::channel();

        tx.send(CmdRequest {
            command: command.to_string(),
            args,
            priority,
            response_tx,
        }).map_err(|e| format!("Failed to send: {}", e))?;

        let response = response_rx.await.map_err(|e| format!("Failed to receive: {}", e))?;
        
        // Cache successful responses
        if response.success {
            let mut cache = self.cmd_cache.lock().unwrap();
            cache.insert(cache_key, response.clone());
        }

        self.record_request(response.duration_ms);
        Ok(response)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PUBLIC API - TTS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Synthesize speech with caching
    pub async fn speak(&self, text: &str, output_path: &str, speaker_id: &str) -> Result<TtsResponse, String> {
        self.speak_internal(text, speaker_id, Priority::Normal).await
    }

    /// Synthesize speech with high priority (for immediate responses)
    pub async fn speak_urgent(&self, text: &str, output_path: &str, speaker_id: &str) -> Result<TtsResponse, String> {
        self.speak_internal(text, speaker_id, Priority::High).await
    }

    async fn speak_internal(&self, text: &str, speaker_id: &str, priority: Priority) -> Result<TtsResponse, String> {
        // Use single output file that gets overwritten
        let output_path = TTS_OUTPUT_FILE;
        
        // Check cache
        let cache_key = format!("{}:{}", text, speaker_id);
        {
            let mut cache = self.tts_cache.lock().unwrap();
            if let Some(_) = cache.get(&cache_key) {
                // Cache hit - file already synthesized
                self.record_cache_hit(false);
                return Ok(TtsResponse {
                    success: true,
                    output_path: output_path.to_string(),
                    duration_ms: 0,
                    cached: true,
                });
            }
        }

        let tx = self.tts_tx.lock().unwrap().clone().ok_or("TTS pool not initialized")?;
        let (response_tx, response_rx) = oneshot::channel();

        tx.send(TtsRequest {
            text: text.to_string(),
            output_path: output_path.to_string(),
            speaker_id: speaker_id.to_string(),
            priority,
            response_tx,
        }).map_err(|e| format!("Failed to send: {}", e))?;

        let response = response_rx.await.map_err(|e| format!("Failed to receive: {}", e))?;

        // Cache successful responses
        if response.success {
            let mut cache = self.tts_cache.lock().unwrap();
            cache.insert(cache_key, output_path.to_string());
        }

        self.record_request(response.duration_ms);
        Ok(response)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PUBLIC API - FFMPEG
    // ═══════════════════════════════════════════════════════════════════════════

    pub async fn execute_ffmpeg(&self, args: Vec<String>) -> Result<FFmpegResponse, String> {
        let tx = self.ffmpeg_tx.lock().unwrap().clone().ok_or("FFmpeg pool not initialized")?;
        let (response_tx, response_rx) = oneshot::channel();

        tx.send(FFmpegRequest {
            args,
            priority: Priority::Normal,
            response_tx,
        }).map_err(|e| format!("Failed to send: {}", e))?;

        let response = response_rx.await.map_err(|e| format!("Failed to receive: {}", e))?;
        self.record_request(response.duration_ms);
        Ok(response)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PUBLIC API - FFPLAY
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn start_ffplay(&self, args: &[&str]) -> Result<Child, String> {
        let ffplay_path = self.ffplay_path.read().unwrap().clone();

        let child = Command::new_hidden(&ffplay_path)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start ffplay: {}", e))?;

        self.active_processes.lock().unwrap().push(child.id());
        Ok(child)
    }

    pub fn stop_all_ffplay(&self) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new_hidden("taskkill")
                .args(&["/IM", "ffplay.exe", "/F"])
                .output();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("killall").arg("ffplay").output();
        }

        self.active_processes.lock().unwrap().clear();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STATISTICS & CACHE MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    fn record_request(&self, duration_ms: u64) {
        let mut stats = self.stats.write().unwrap();
        stats.request_count += 1;
        stats.total_response_time_ms += duration_ms;
    }

    fn record_cache_hit(&self, is_cmd: bool) {
        let mut stats = self.stats.write().unwrap();
        if is_cmd {
            stats.cmd_cache_hits += 1;
        } else {
            stats.tts_cache_hits += 1;
        }
    }

    pub fn get_stats(&self) -> SharedMemoryStats {
        self.stats.read().unwrap().clone()
    }

    pub fn clear_caches(&self) {
        self.cmd_cache.lock().unwrap().entries.clear();
        self.tts_cache.lock().unwrap().entries.clear();
    }

    pub fn clear_expired_caches(&self) {
        self.cmd_cache.lock().unwrap().clear_expired();
        self.tts_cache.lock().unwrap().clear_expired();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PATH GETTERS
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn get_ffmpeg_path(&self) -> String {
        self.ffmpeg_path.read().unwrap().clone()
    }

    pub fn get_ffplay_path(&self) -> String {
        self.ffplay_path.read().unwrap().clone()
    }

    pub fn get_piper_path(&self) -> String {
        self.piper_path.read().unwrap().clone()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLEANUP
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn cleanup(&self) {
        self.stop_all_ffplay();
        self.clear_caches();

        *self.cmd_tx.lock().unwrap() = None;
        *self.tts_tx.lock().unwrap() = None;
        *self.ffmpeg_tx.lock().unwrap() = None;

        *self.initialized.write().unwrap() = false;
        *self.warmed_up.write().unwrap() = false;
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        Self::new()
    }
}


// ═══════════════════════════════════════════════════════════════════════════════
// SYNC EXECUTION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct CmdResult {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

fn execute_cmd_sync(command: &str, args: &[String]) -> CmdResult {
    #[cfg(target_os = "windows")]
    let output = Command::new(command)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(command).args(args).output();

    match output {
        Ok(output) => CmdResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        },
        Err(e) => CmdResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
            exit_code: None,
        },
    }
}

fn execute_piper_sync(piper_path: &str, text: &str, output_path: &str, speaker_id: &str) -> bool {
    let model_path = "./pkg/models/bold_voice/en_US-libritts_r-medium.onnx";

    let args = vec![
        "--model", model_path,
        "--output_file", output_path,
        "--speaker", speaker_id,
    ];

    #[cfg(target_os = "windows")]
    let mut child = match Command::new(piper_path)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = match Command::new(piper_path)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
        let _ = stdin.write_all(b"\n");
        let _ = stdin.flush();
    }

    child.wait().map(|s| s.success()).unwrap_or(false)
}

fn execute_ffmpeg_sync(ffmpeg_path: &str, args: &[String]) -> (bool, String, String) {
    #[cfg(target_os = "windows")]
    let output = Command::new(ffmpeg_path)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new(ffmpeg_path).args(args).output();

    match output {
        Ok(output) => (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ),
        Err(e) => (false, String::new(), format!("Failed: {}", e)),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PATH RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════════

fn resolve_ffmpeg_path() -> String {
    let paths = [
        "./pkg/ffmpeg/bin/ffmpeg.exe",
        "./pkg/ffmpeg/ffmpeg.exe",
        "ffmpeg.exe",
        "ffmpeg",
    ];
    for path in paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    FFMPEG_PATH.to_string()
}

fn resolve_ffplay_path() -> String {
    let paths = [
        "./pkg/ffmpeg/bin/ffplay.exe",
        "./pkg/ffmpeg/ffplay.exe",
        "ffplay.exe",
        "ffplay",
    ];
    for path in paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    FFPLAY_PATH.to_string()
}

fn resolve_piper_path() -> String {
    let paths = [
        "./pkg/piper/piper.exe",
        "piper/piper.exe",
        "piper.exe",
        "piper",
    ];
    for path in paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    PIPER_PATH.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONVENIENCE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Initialize the shared memory system
pub async fn init_shared_memory() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    SHARED_MEMORY.initialize().await?;
    SHARED_MEMORY.warm_up().await?;
    Ok(())
}

/// Get the global shared memory instance
pub fn get_shared_memory() -> Arc<SharedMemory> {
    SHARED_MEMORY.clone()
}

/// Quick command execution
pub async fn quick_cmd(command: &str, args: Vec<String>) -> Result<CmdResponse, String> {
    SHARED_MEMORY.execute_cmd(command, args).await
}

/// Quick TTS synthesis
pub async fn quick_speak(text: &str, output_path: &str, speaker_id: &str) -> Result<TtsResponse, String> {
    SHARED_MEMORY.speak(text, output_path, speaker_id).await
}

/// Quick FFmpeg execution
pub async fn quick_ffmpeg(args: Vec<String>) -> Result<FFmpegResponse, String> {
    SHARED_MEMORY.execute_ffmpeg(args).await
}

/// Start FFplay
pub fn quick_ffplay(args: &[&str]) -> Result<Child, String> {
    SHARED_MEMORY.start_ffplay(args)
}

/// Cleanup on shutdown
pub fn cleanup_shared_memory() {
    SHARED_MEMORY.cleanup();
}

/// Get statistics
pub fn get_stats() -> SharedMemoryStats {
    SHARED_MEMORY.get_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry::new("test".to_string());
        assert!(!entry.is_expired());
        assert_eq!(entry.hits, 0);
    }

    #[test]
    fn test_response_cache() {
        let mut cache: ResponseCache<String> = ResponseCache::new(3);
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key3"), None);
    }

    #[test]
    fn test_shared_memory_creation() {
        let sm = SharedMemory::new();
        let stats = sm.get_stats();
        assert_eq!(stats.request_count, 0);
    }

    #[test]
    fn test_path_resolution() {
        let _ffmpeg = resolve_ffmpeg_path();
        let _ffplay = resolve_ffplay_path();
        let _piper = resolve_piper_path();
    }
}
