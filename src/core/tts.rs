
// Optimized Text-to-Speech with audio caching for low latency
// Audio caching eliminates repeated synthesis for common phrases
// This module replaces tts.rs with caching support for faster responses

use once_cell::sync::Lazy;
use rodio::{Decoder, OutputStream, Sink};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Type alias for our error type (compatible with Box<dyn Error>)
pub type TtsResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Maximum cache entries
const MAX_CACHE_ENTRIES: usize = 100;

/// Cache entry expiry time (1 hour)
const CACHE_EXPIRY_SECS: u64 = 3600;

/// Audio output directory
const AUDIO_DIR: &str = "./pkg/audio";

/// Single TTS output file (gets overwritten on each synthesis)
const TTS_OUTPUT_FILE: &str = "tts_output.wav";

/// Get speaker ID from config (personality-based)
fn get_speaker_id() -> String {
    // Try to get from config, fallback to default
    if let Ok(config) = std::panic::catch_unwind(|| {
        crate::config::CONFIG.speaker_id()
    }) {
        config
    } else {
        "051".to_string()
    }
}

/// Global TTS cache instance (only caches file paths, not audio streams)
static TTS_CACHE: Lazy<Arc<TtsCache>> = Lazy::new(|| Arc::new(TtsCache::new()));

/// Cache entry with metadata
#[derive(Clone)]
struct CacheEntry {
    audio_path: PathBuf,
    created_at: Instant,
    access_count: u32,
}

/// TTS configuration
#[derive(Clone, Debug)]
pub struct TtsConfig {
    pub speaker_id: String,
    pub model_path: String,
    pub use_cache: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            speaker_id: get_speaker_id(),
            model_path: get_model_path().unwrap_or_default(),
            use_cache: true,
        }
    }
}

/// TTS cache for audio files (thread-safe, no audio streams stored)
pub struct TtsCache {
    /// Audio cache (text hash -> cache entry)
    cache: RwLock<HashMap<u64, CacheEntry>>,
    /// Configuration
    config: RwLock<TtsConfig>,
    /// Statistics
    stats: RwLock<TtsStats>,
    /// Whether engine is initialized
    initialized: RwLock<bool>,
}

/// TTS statistics for monitoring
#[derive(Default, Clone)]
pub struct TtsStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_synthesis_time_ms: u64,
    pub total_requests: u64,
    pub avg_latency_ms: f64,
}

impl TtsCache {
    pub fn new() -> Self {
        // Ensure audio directory exists
        let _ = fs::create_dir_all(AUDIO_DIR);

        Self {
            cache: RwLock::new(HashMap::new()),
            config: RwLock::new(TtsConfig::default()),
            stats: RwLock::new(TtsStats::default()),
            initialized: RwLock::new(false),
        }
    }

    /// Initialize the TTS cache (call once at startup)
    pub fn initialize(&self) -> TtsResult<()> {
        {
            let initialized = self.initialized.read().unwrap();
            if *initialized {
                return Ok(());
            }
        }

        // Verify Piper is available
        let _ = get_piper_path()?;
        let _ = get_model_path()?;

        {
            let mut initialized = self.initialized.write().unwrap();
            *initialized = true;
        }

        Ok(())
    }

    /// Hash text for cache key
    fn hash_text(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.to_lowercase().trim().hash(&mut hasher);
        hasher.finish()
    }

    /// Get audio from cache
    fn get_from_cache(&self, hash: u64) -> Option<PathBuf> {
        let cache = self.cache.read().unwrap();

        if let Some(entry) = cache.get(&hash) {
            // Check if file still exists and not expired
            if entry.audio_path.exists() && entry.created_at.elapsed().as_secs() < CACHE_EXPIRY_SECS
            {
                return Some(entry.audio_path.clone());
            }
        }

        None
    }

    /// Add audio to cache
    fn add_to_cache(&self, hash: u64, path: PathBuf) {
        let mut cache = self.cache.write().unwrap();

        // Evict old entries if at capacity
        if cache.len() >= MAX_CACHE_ENTRIES {
            self.evict_cache_entries(&mut cache);
        }

        cache.insert(
            hash,
            CacheEntry {
                audio_path: path,
                created_at: Instant::now(),
                access_count: 1,
            },
        );
    }

    /// Evict old/unused cache entries
    fn evict_cache_entries(&self, cache: &mut HashMap<u64, CacheEntry>) {
        // Remove expired entries
        let expired_hashes: Vec<u64> = cache
            .iter()
            .filter(|(_, entry)| entry.created_at.elapsed().as_secs() >= CACHE_EXPIRY_SECS)
            .map(|(hash, _)| *hash)
            .collect();

        for hash in expired_hashes {
            if let Some(entry) = cache.remove(&hash) {
                let _ = fs::remove_file(&entry.audio_path);
            }
        }

        // If still too many, remove least accessed
        if cache.len() >= MAX_CACHE_ENTRIES {
            let mut entries: Vec<_> = cache.iter().map(|(h, e)| (*h, e.access_count)).collect();
            entries.sort_by_key(|(_, count)| *count);

            let to_remove = cache.len() - MAX_CACHE_ENTRIES / 2;
            for (hash, _) in entries.into_iter().take(to_remove) {
                if let Some(entry) = cache.remove(&hash) {
                    let _ = fs::remove_file(&entry.audio_path);
                }
            }
        }
    }

    /// Update statistics
    fn update_stats(&self, cache_hit: bool, latency_ms: u64) {
        let mut stats = self.stats.write().unwrap();
        stats.total_requests += 1;

        if cache_hit {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
            stats.total_synthesis_time_ms += latency_ms;
        }

        if stats.total_requests > 0 {
            stats.avg_latency_ms =
                (stats.total_synthesis_time_ms as f64) / (stats.cache_misses.max(1) as f64);
        }
    }

    /// Get current statistics
    pub fn get_stats(&self) -> TtsStats {
        self.stats.read().unwrap().clone()
    }

    /// Clear the audio cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();

        for entry in cache.values() {
            let _ = fs::remove_file(&entry.audio_path);
        }

        cache.clear();
    }
}

/// TTS Engine - the main interface
pub struct TtsEngine;

/// Global TTS engine instance
pub static TTS_ENGINE: Lazy<TtsEngine> = Lazy::new(|| TtsEngine);

impl TtsEngine {
    /// Initialize the TTS engine (call once at startup)
    pub fn initialize(&self) -> TtsResult<()> {
        TTS_CACHE.initialize()
    }

    /// Speak text (main API - blocking)
    pub fn speak(&self, text: &str) -> TtsResult<()> {
        let start = Instant::now();

        // Always synthesize to the single output file (overwrites previous)
        // This ensures we use the latest audio without accumulating files
        let audio_path = synthesize_to_file_internal(text, None)?;

        let elapsed = start.elapsed().as_millis() as u64;
        TTS_CACHE.update_stats(false, elapsed);

        // Play the audio
        play_audio_file(&audio_path)
    }

    /// Get TTS statistics
    pub fn get_stats(&self) -> TtsStats {
        TTS_CACHE.get_stats()
    }

    /// Clear audio cache
    pub fn clear_cache(&self) {
        TTS_CACHE.clear_cache()
    }
}

/// Play audio file (creates new output stream each time - thread safe)
fn play_audio_file(path: &Path) -> TtsResult<()> {
    if !path.exists() {
        return Err(format!("Audio file not found: {}", path.display()).into());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Create new output stream for each playback (thread-safe)
    let (_stream, handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&handle)?;
    let source = Decoder::new(reader)?;
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

/// Synthesize text to audio file
fn synthesize_to_file_internal(text: &str, speaker_id: Option<&str>) -> TtsResult<PathBuf> {
    // Use single output file (gets overwritten each time)
    let output_path = PathBuf::from(AUDIO_DIR).join(TTS_OUTPUT_FILE);

    // Use Piper for all platforms
    let speaker = speaker_id.unwrap_or_else(|| {
        // Get speaker from config - use leak to get static str (safe for short-lived strings)
        Box::leak(get_speaker_id().into_boxed_str())
    });
    run_piper_synthesis(text, &output_path, speaker)?;

    Ok(output_path)
}

/// Run Piper synthesis
fn run_piper_synthesis(text: &str, output_path: &Path, speaker_id: &str) -> TtsResult<()> {
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let piper_path = get_piper_path()?;
    let model_path = get_model_path()?;
    
    // Get espeak-ng-data directory path
    let data_dir = get_espeak_data_dir()?;
    
    // Run Piper with file output
    #[cfg(target_os = "windows")]
    let mut child = Command::new(&piper_path)
        .args(&[
            "--model",
            &model_path,
            "--data-dir",
            &data_dir,
            "--output_file",
            output_path.to_str().unwrap(),
            "--speaker",
            speaker_id,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(&piper_path)
        .args(&[
            "--model",
            &model_path,
            "--output_file",
            output_path.to_str().unwrap(),
            "--speaker",
            speaker_id,
        ])
        .env("ESPEAK_DATA_PATH", &data_dir)  // Set environment variable
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write text to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        drop(stdin);
    }

    // Wait for completion
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Piper failed: {}", stderr).into());
    }

    // Wait for file to be written
    std::thread::sleep(Duration::from_millis(50));

    if !output_path.exists() {
        return Err("Piper did not create output file".into());
    }

    Ok(())
}

/// Get Piper executable path
fn get_piper_path() -> TtsResult<String> {
    let paths = vec![
        "./pkg/piper/piper.exe",
        "pkg/piper/piper.exe",
        "./piper/piper.exe",
        "piper/piper.exe",
        "./pkg/piper/piper",
        "pkg/piper/piper",
    ];

    for path in paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err("Piper executable not found".into())
}

/// Get voice model path
fn get_model_path() -> TtsResult<String> {
    let paths = vec![
        "./pkg/models/bold_voice/en_US-libritts_r-medium.onnx",
        "pkg/models/bold_voice/en_US-libritts_r-medium.onnx",
    ];

    for path in paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err("Voice model not found".into())
}

/// Get espeak-ng-data directory path
fn get_espeak_data_dir() -> TtsResult<String> {
    let paths = vec![
        // macOS Homebrew paths (priority for macOS)
        "/opt/homebrew/Cellar/espeak-ng/1.52.0/share/espeak-ng-data",  // Apple Silicon (versioned)
        "/opt/homebrew/share/espeak-ng-data",  // Apple Silicon (symlink)
        "/usr/local/Cellar/espeak-ng/1.52.0/share/espeak-ng-data",     // Intel (versioned)
        "/usr/local/share/espeak-ng-data",     // Intel (symlink)
        // Windows paths
        "./pkg/piper/espeak-ng-data",
        "pkg/piper/espeak-ng-data",
        "./pkg/piper/lib/espeak-ng-data",
        "pkg/piper/lib/espeak-ng-data",
        // Linux paths
        "/usr/share/espeak-ng-data",
        "/usr/local/share/espeak-ng-data",
        // Fallback paths
        "./pkg/models/bold_voice/espeak-ng-data",
        "pkg/models/bold_voice/espeak-ng-data",
    ];

    for path in paths {
        let p = Path::new(path);
        if p.exists() && p.join("phontab").exists() {
            println!("[TTS] Using espeak-ng-data at: {}", path);
            return Ok(path.to_string());
        }
    }

    Err("espeak-ng-data directory not found. Install espeak-ng via: brew install espeak-ng".into())
}

// ============================================
// Public convenience functions
// ============================================

/// Initialize TTS engine (call once at startup)
pub fn init_tts() -> TtsResult<()> {
    TTS_ENGINE.initialize()
}

/// Speak text (blocking) - returns TtsResult for internal use
pub fn speak(text: &str) -> TtsResult<()> {
    TTS_ENGINE.speak(text)
}

/// Speak text (blocking) - compatible with Box<dyn Error>
/// Use this when you need compatibility with functions expecting Box<dyn Error>
pub fn speak_compat(text: &str) -> Result<(), Box<dyn Error>> {
    TTS_ENGINE.speak(text).map_err(|e| -> Box<dyn Error> { e })
}

/// Synthesize to file without playing
pub fn synthesize(text: &str) -> TtsResult<PathBuf> {
    synthesize_to_file_internal(text, None)
}

/// Clear audio cache
pub fn clear_cache() {
    TTS_ENGINE.clear_cache()
}

/// Get TTS statistics
pub fn get_stats() -> TtsStats {
    TTS_ENGINE.get_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_hashing() {
        let hash1 = TTS_CACHE.hash_text("Hello");
        let hash2 = TTS_CACHE.hash_text("hello");
        let hash3 = TTS_CACHE.hash_text("  HELLO  ");

        // Should normalize case and whitespace
        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }
}