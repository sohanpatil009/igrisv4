// src/core/audio_capture.rs
// Optimized audio capture with VAD integration for low-latency recording
// Eliminates fixed recording durations by detecting speech boundaries in real-time

use crate::core::vad::{VoiceActivityDetector, VadConfig, VadEvent};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Target sample rate for Whisper
const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Maximum wait time for speech to start (ms)
const MAX_WAIT_FOR_SPEECH_MS: u64 = 10000;

/// Audio capture mode
#[derive(Clone, Debug)]
pub enum CaptureMode {
    /// Optimized for wake word detection (fast, short)
    WakeWord,
    /// Optimized for voice commands (balanced)
    Command,
    /// Optimized for longer dictation
    Dictation,
    /// Fixed duration recording (fallback)
    FixedDuration { seconds: u64 },
    /// Custom VAD configuration
    Custom(VadConfig),
}

impl CaptureMode {
    fn to_vad_config(&self) -> Option<VadConfig> {
        match self {
            CaptureMode::WakeWord => Some(VadConfig::for_wake_word()),
            CaptureMode::Command => Some(VadConfig::for_commands()),
            CaptureMode::Dictation => Some(VadConfig::for_dictation()),
            CaptureMode::FixedDuration { .. } => None,
            CaptureMode::Custom(config) => Some(config.clone()),
        }
    }
}

/// Result of audio capture
#[derive(Debug)]
pub struct CaptureResult {
    /// Captured audio samples at 16kHz mono
    pub samples: Vec<f32>,
    /// Duration of captured audio in milliseconds
    pub duration_ms: u32,
    /// Whether speech was detected
    pub speech_detected: bool,
    /// Time to first speech detection (if applicable)
    pub time_to_speech_ms: Option<u32>,
}

/// Audio capture configuration
#[derive(Clone)]
pub struct CaptureConfig {
    /// Capture mode
    pub mode: CaptureMode,
    /// Maximum wait time for speech to start
    pub max_wait_ms: u64,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Command,
            max_wait_ms: MAX_WAIT_FOR_SPEECH_MS,
            debug: false,
        }
    }
}

impl CaptureConfig {
    pub fn wake_word() -> Self {
        Self {
            mode: CaptureMode::WakeWord,
            max_wait_ms: 5000,
            debug: false,
        }
    }

    pub fn command() -> Self {
        Self {
            mode: CaptureMode::Command,
            max_wait_ms: 10000,
            debug: false,
        }
    }

    pub fn fixed(seconds: u64) -> Self {
        Self {
            mode: CaptureMode::FixedDuration { seconds },
            max_wait_ms: seconds * 1000 + 1000,
            debug: false,
        }
    }
}

/// Shared state for audio capture
struct CaptureState {
    /// Raw audio buffer (device sample rate)
    raw_buffer: Vec<f32>,
    /// Device sample rate
    device_sample_rate: u32,
    /// Number of channels
    channels: usize,
    /// VAD instance (if using VAD mode)
    vad: Option<VoiceActivityDetector>,
    /// Frame buffer for VAD processing
    frame_buffer: Vec<f32>,
    /// Whether capture is complete
    complete: bool,
    /// Whether speech was detected
    speech_detected: bool,
    /// Time when speech started
    speech_start_time: Option<Instant>,
    /// Capture start time
    start_time: Instant,
}

impl CaptureState {
    fn new(device_sample_rate: u32, channels: usize, vad_config: Option<VadConfig>) -> Self {
        Self {
            raw_buffer: Vec::with_capacity(device_sample_rate as usize * 15), // 15 sec capacity
            device_sample_rate,
            channels,
            vad: vad_config.map(VoiceActivityDetector::new),
            frame_buffer: Vec::with_capacity(512),
            complete: false,
            speech_detected: false,
            speech_start_time: None,
            start_time: Instant::now(),
        }
    }

    /// Process incoming audio samples
    fn process_samples(&mut self, samples: &[f32]) {
        if self.complete {
            return;
        }

        // Convert to mono if stereo
        let mono_samples: Vec<f32> = if self.channels == 2 {
            samples
                .chunks(2)
                .map(|chunk| {
                    if chunk.len() == 2 {
                        (chunk[0] + chunk[1]) / 2.0
                    } else {
                        chunk[0]
                    }
                })
                .collect()
        } else {
            samples.to_vec()
        };

        // Store raw samples
        self.raw_buffer.extend(&mono_samples);

        // Process through VAD if enabled
        if let Some(ref mut vad) = self.vad {
            // Resample to 16kHz for VAD processing
            let resampled =
                resample_chunk(&mono_samples, self.device_sample_rate, WHISPER_SAMPLE_RATE);

            for sample in resampled {
                if let Some((_, Some(event))) = vad.process_sample(sample, &mut self.frame_buffer) {
                    match event {
                        VadEvent::SpeechStarted => {
                            self.speech_detected = true;
                            self.speech_start_time = Some(Instant::now());
                        }
                        VadEvent::SpeechEnded | VadEvent::MaxDurationReached => {
                            self.complete = true;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Get final audio at 16kHz
    fn get_resampled_audio(&self) -> Vec<f32> {
        if let Some(ref vad) = self.vad {
            // Use VAD's trimmed audio
            vad.get_audio().to_vec()
        } else {
            // Resample entire buffer
            resample_linear(
                &self.raw_buffer,
                self.device_sample_rate,
                WHISPER_SAMPLE_RATE,
            )
        }
    }
}

/// Capture audio with VAD-based endpoint detection
pub fn capture_audio_vad(
    config: CaptureConfig,
) -> Result<CaptureResult, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let stream_config = device.default_input_config()?;

    let sample_rate = stream_config.sample_rate().0;
    let channels = stream_config.channels() as usize;

    if config.debug {
        println!("ðŸŽ¤ Audio device: {} Hz, {} channels", sample_rate, channels);
    }

    let vad_config = config.mode.to_vad_config();
    let state = Arc::new(Mutex::new(CaptureState::new(
        sample_rate,
        channels,
        vad_config,
    )));
    let state_clone = state.clone();

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    let err_fn = move |err| eprintln!("Audio stream error: {}", err);

    // Build the appropriate stream based on sample format
    let stream = match stream_config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config.into(),
            move |data: &[f32], _: &_| {
                if !stop_flag_clone.load(Ordering::Relaxed) {
                    if let Ok(mut state) = state_clone.lock() {
                        state.process_samples(data);
                    }
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => {
            let state_clone = state.clone();
            let stop_flag_clone = stop_flag.clone();
            device.build_input_stream(
                &stream_config.into(),
                move |data: &[i16], _: &_| {
                    if !stop_flag_clone.load(Ordering::Relaxed) {
                        let floats: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        if let Ok(mut state) = state_clone.lock() {
                            state.process_samples(&floats);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let state_clone = state.clone();
            let stop_flag_clone = stop_flag.clone();
            device.build_input_stream(
                &stream_config.into(),
                move |data: &[u16], _: &_| {
                    if !stop_flag_clone.load(Ordering::Relaxed) {
                        let floats: Vec<f32> =
                            data.iter().map(|&s| (s as f32 / 65535.0) - 0.5).collect();
                        if let Ok(mut state) = state_clone.lock() {
                            state.process_samples(&floats);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        other => return Err(format!("Unsupported sample format: {:?}", other).into()),
    };

    stream.play()?;

    let start_time = Instant::now();
    let max_wait = Duration::from_millis(config.max_wait_ms);

    // For fixed duration mode
    let fixed_duration = match config.mode {
        CaptureMode::FixedDuration { seconds } => Some(Duration::from_secs(seconds)),
        _ => None,
    };

    // Wait for capture to complete
    loop {
        std::thread::sleep(Duration::from_millis(10));

        let elapsed = start_time.elapsed();

        // Check fixed duration
        if let Some(duration) = fixed_duration {
            if elapsed >= duration {
                break;
            }
        }

        // Check VAD completion
        {
            let state_guard = state.lock().unwrap();
            if state_guard.complete {
                break;
            }

            // Check timeout (only if no speech detected yet)
            if !state_guard.speech_detected && elapsed >= max_wait {
                if config.debug {
                    println!("â° Timeout waiting for speech");
                }
                break;
            }
        }

        // Safety timeout (30 seconds absolute max)
        if elapsed >= Duration::from_secs(30) {
            break;
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    drop(stream);

    // Extract results
    let state_guard = state.lock().unwrap();
    let samples = state_guard.get_resampled_audio();
    let duration_ms = (samples.len() as u32 * 1000) / WHISPER_SAMPLE_RATE;
    let speech_detected = state_guard.speech_detected;
    let time_to_speech_ms = state_guard
        .speech_start_time
        .map(|t| t.duration_since(state_guard.start_time).as_millis() as u32);

    Ok(CaptureResult {
        samples,
        duration_ms,
        speech_detected,
        time_to_speech_ms,
    })
}

/// Quick capture for wake word detection
pub fn capture_wake_word() -> Result<CaptureResult, Box<dyn std::error::Error>> {
    capture_audio_vad(CaptureConfig::wake_word())
}

/// Quick capture for voice commands
pub fn capture_command() -> Result<CaptureResult, Box<dyn std::error::Error>> {
    capture_audio_vad(CaptureConfig::command())
}

/// Fixed duration capture (fallback to original behavior)
pub fn capture_fixed(seconds: u64) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let result = capture_audio_vad(CaptureConfig::fixed(seconds))?;
    Ok(result.samples)
}

/// Resample a small chunk of audio (for real-time processing)
fn resample_chunk(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_hz as f64 / to_hz as f64;
    let new_len = (input.len() as f64 / ratio) as usize;

    if new_len == 0 {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let original_idx_float = i as f64 * ratio;
        let original_idx = original_idx_float as usize;

        if original_idx >= input.len() - 1 {
            break;
        }

        let fraction = (original_idx_float - original_idx as f64) as f32;
        let a = input[original_idx];
        let b = input[original_idx + 1];

        output.push(a + (b - a) * fraction);
    }

    output
}

/// Full linear resampling for final audio
fn resample_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return input.to_vec();
    }

    let ratio = from_hz as f32 / to_hz as f32;
    let new_len = (input.len() as f32 / ratio) as usize;
    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let original_idx_float = i as f32 * ratio;
        let original_idx = original_idx_float as usize;

        if original_idx >= input.len() - 1 {
            break;
        }

        let fraction = original_idx_float - original_idx as f32;
        let a = input[original_idx];
        let b = input[original_idx + 1];

        output.push(a + (b - a) * fraction);
    }

    output
}

/// Streaming audio capture with callback
pub struct StreamingCapture {
    stop_flag: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
}

impl StreamingCapture {
    /// Start streaming capture with a callback for each audio chunk
    pub fn start<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: FnMut(&[f32]) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;
        let config = device.default_input_config()?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let callback = Arc::new(Mutex::new(callback));

        let err_fn = |err| eprintln!("Stream error: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if stop_flag_clone.load(Ordering::Relaxed) {
                        return;
                    }

                    // Convert to mono at 16kHz
                    let mono: Vec<f32> = if channels == 2 {
                        data.chunks(2)
                            .map(|c| (c[0] + c.get(1).unwrap_or(&0.0)) / 2.0)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    let resampled = resample_chunk(&mono, sample_rate, WHISPER_SAMPLE_RATE);

                    if let Ok(mut cb) = callback.lock() {
                        cb(&resampled);
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err("Only F32 format supported for streaming".into()),
        };

        stream.play()?;

        Ok(Self {
            stop_flag,
            stream: Some(stream),
        })
    }

    /// Stop the streaming capture
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.stream = None;
    }
}

impl Drop for StreamingCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample() {
        let input: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0).sin()).collect();
        let output = resample_linear(&input, 48000, 16000);

        // Should be roughly 1/3 the size
        assert!(output.len() > 15000 && output.len() < 17000);
    }

    #[test]
    fn test_capture_config() {
        let wake = CaptureConfig::wake_word();
        assert!(matches!(wake.mode, CaptureMode::WakeWord));

        let cmd = CaptureConfig::command();
        assert!(matches!(cmd.mode, CaptureMode::Command));

        let fixed = CaptureConfig::fixed(5);
        assert!(matches!(
            fixed.mode,
            CaptureMode::FixedDuration { seconds: 5 }
        ));
    }
}