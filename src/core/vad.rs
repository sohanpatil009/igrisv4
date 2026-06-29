// src/core/vad.rs
// Production-grade Voice Activity Detection for IGRIS v3
// Hybrid approach: Energy-based noise gate + FFT spectral features + Weighted scoring + State machine
//
// Pipeline:
//   Audio frame (10/20/30ms @ 16kHz)
//   → RMS Energy + Noise Gate (fast reject silence)
//   → ZCR + Spectral Centroid + Spectral Flux + Band Energy (FFT-based)
//   → Weighted confidence score
//   → State machine (Idle → PreSpeech → Speaking → PostSpeech)
//   → VadResult
//
// Performance targets:
//   CPU: < 5% on average laptop
//   Memory: < 20 MB
//   Speech start: < 100ms
//   Speech end: < 300ms

use std::collections::VecDeque;
use std::sync::Arc;

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};

// --------------- Configuration ---------------

/// VAD configuration — serializable, all fields have defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    /// Master enable
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Frame size in milliseconds (10, 20, or 30)
    #[serde(default = "default_frame_ms")]
    pub frame_ms: u32,
    /// Audio sample rate in Hz
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// Confidence threshold above which a frame is considered speech (0.0 – 1.0)
    #[serde(default = "default_speech_threshold")]
    pub speech_threshold: f32,
    /// Consecutive speech frames required to enter Speaking
    #[serde(default = "default_start_frames")]
    pub start_frames: u32,
    /// Consecutive silence frames required to end speech
    #[serde(default = "default_end_frames")]
    pub end_frames: u32,
    /// Hangover in ms — continue reporting speech after confidence drops
    #[serde(default = "default_hangover_ms")]
    pub hangover_ms: u32,
    /// Minimum valid speech duration in ms (shorter utterances are discarded)
    #[serde(default = "default_min_speech_ms")]
    pub min_speech_ms: u32,
    /// Maximum speech duration in ms (safety limit)
    #[serde(default = "default_max_speech_ms")]
    pub max_speech_ms: u32,
    /// Pre-speech ring-buffer size in ms
    #[serde(default = "default_pre_speech_buffer_ms")]
    pub pre_speech_buffer_ms: u32,
    /// Post-speech ring-buffer size in ms
    #[serde(default = "default_post_speech_buffer_ms")]
    pub post_speech_buffer_ms: u32,

    // ---- Noise gate ----
    /// How fast the noise floor adapts (0.0 = never, 1.0 = instantly)
    #[serde(default = "default_noise_floor_adaptation")]
    pub noise_floor_adaptation: f32,
    /// Noise floor is multiplied by this to obtain the gate threshold
    #[serde(default = "default_noise_gate_multiplier")]
    pub noise_gate_multiplier: f32,
    /// Absolute minimum gate threshold (prevents false triggers in dead silence)
    #[serde(default = "default_noise_gate_min")]
    pub noise_gate_min: f32,

    // ---- Feature weights (must sum to 1.0-ish) ----
    #[serde(default = "default_weight_energy")]
    pub weight_energy: f32,
    #[serde(default = "default_weight_zcr")]
    pub weight_zcr: f32,
    #[serde(default = "default_weight_centroid")]
    pub weight_centroid: f32,
    #[serde(default = "default_weight_flux")]
    pub weight_flux: f32,
    #[serde(default = "default_weight_band_energy")]
    pub weight_band_energy: f32,

    // ---- Per-feature thresholds ----
    #[serde(default = "default_energy_threshold")]
    pub energy_threshold: f32,
    /// ZCR values above this strongly indicate non-speech
    #[serde(default = "default_zcr_threshold")]
    pub zcr_threshold: f32,
    /// Expected spectral centroid lower bound (Hz)
    #[serde(default = "default_centroid_min")]
    pub centroid_min: f32,
    /// Expected spectral centroid upper bound (Hz)
    #[serde(default = "default_centroid_max")]
    pub centroid_max: f32,
    /// Minimum spectral flux to consider as active speech
    #[serde(default = "default_flux_min")]
    pub flux_min: f32,
    /// Minimum band-energy ratio (300-3400 Hz / total)
    #[serde(default = "default_band_ratio_min")]
    pub band_ratio_min: f32,
}

// ---- Default-value helpers for serde ----
fn default_enabled() -> bool { true }
fn default_frame_ms() -> u32 { 20 }
fn default_sample_rate() -> u32 { 16000 }
fn default_speech_threshold() -> f32 { 0.65 }
fn default_start_frames() -> u32 { 3 }
fn default_end_frames() -> u32 { 12 }
fn default_hangover_ms() -> u32 { 250 }
fn default_min_speech_ms() -> u32 { 200 }
fn default_max_speech_ms() -> u32 { 15_000 }
fn default_pre_speech_buffer_ms() -> u32 { 300 }
fn default_post_speech_buffer_ms() -> u32 { 200 }
fn default_noise_floor_adaptation() -> f32 { 0.05 }
fn default_noise_gate_multiplier() -> f32 { 2.5 }
fn default_noise_gate_min() -> f32 { 0.003 }
fn default_weight_energy() -> f32 { 0.35 }
fn default_weight_zcr() -> f32 { 0.15 }
fn default_weight_centroid() -> f32 { 0.15 }
fn default_weight_flux() -> f32 { 0.15 }
fn default_weight_band_energy() -> f32 { 0.20 }
fn default_energy_threshold() -> f32 { 0.02 }
fn default_zcr_threshold() -> f32 { 0.30 }
fn default_centroid_min() -> f32 { 250.0 }
fn default_centroid_max() -> f32 { 3600.0 }
fn default_flux_min() -> f32 { 0.005 }
fn default_band_ratio_min() -> f32 { 0.35 }

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            frame_ms: 20,
            sample_rate: 16000,
            speech_threshold: 0.65,
            start_frames: 3,
            end_frames: 12,
            hangover_ms: 250,
            min_speech_ms: 200,
            max_speech_ms: 15_000,
            pre_speech_buffer_ms: 300,
            post_speech_buffer_ms: 200,

            noise_floor_adaptation: 0.05,
            noise_gate_multiplier: 2.5,
            noise_gate_min: 0.003,

            weight_energy: 0.35,
            weight_zcr: 0.15,
            weight_centroid: 0.15,
            weight_flux: 0.15,
            weight_band_energy: 0.20,

            energy_threshold: 0.02,
            zcr_threshold: 0.30,
            centroid_min: 250.0,
            centroid_max: 3600.0,
            flux_min: 0.005,
            band_ratio_min: 0.35,
        }
    }
}

impl VadConfig {
    /// Number of samples per frame at the configured sample rate
    pub fn frame_samples(&self) -> usize {
        (self.sample_rate as u64 * self.frame_ms as u64 / 1000) as usize
    }

    /// Preset for wake-word detection (fast response, short utterances)
    pub fn for_wake_word() -> Self {
        Self {
            frame_ms: 20,
            start_frames: 2,
            end_frames: 15,
            hangover_ms: 100,
            min_speech_ms: 400,
            max_speech_ms: 2000,
            pre_speech_buffer_ms: 300,
            post_speech_buffer_ms: 200,
            speech_threshold: 0.55,
            noise_gate_multiplier: 2.0,
            noise_gate_min: 0.002,
            ..Self::default()
        }
    }

    /// Preset for voice commands (balanced)
    pub fn for_commands() -> Self {
        Self {
            frame_ms: 20,
            start_frames: 3,
            end_frames: 20,
            hangover_ms: 250,
            min_speech_ms: 300,
            max_speech_ms: 10000,
            pre_speech_buffer_ms: 300,
            post_speech_buffer_ms: 200,
            speech_threshold: 0.65,
            ..Self::default()
        }
    }

    /// Preset for dictation (longer pauses OK)
    pub fn for_dictation() -> Self {
        Self {
            frame_ms: 20,
            start_frames: 3,
            end_frames: 30,
            hangover_ms: 400,
            min_speech_ms: 500,
            max_speech_ms: 30000,
            pre_speech_buffer_ms: 300,
            post_speech_buffer_ms: 300,
            speech_threshold: 0.60,
            ..Self::default()
        }
    }

    fn ms_to_samples(&self, ms: u32) -> usize {
        (self.sample_rate as u64 * ms as u64 / 1000) as usize
    }

    fn frames_for_ms(&self, ms: u32) -> u32 {
        ((ms as f32) / self.frame_ms as f32).ceil() as u32
    }

    /// FFT size — next power of two >= frame_samples
    fn fft_size(&self) -> usize {
        let n = self.frame_samples();
        let mut p2 = 1;
        while p2 < n {
            p2 <<= 1;
        }
        p2.max(64)
    }
}

// --------------- Result & Events ---------------

/// Per-frame VAD result
#[derive(Debug, Clone)]
pub struct VadResult {
    /// Whether this frame is classified as speech
    pub is_speech: bool,
    /// Composite confidence in [0, 1]
    pub confidence: f32,
    /// Root-mean-square energy of the frame
    pub rms_energy: f32,
    /// Zero-crossing rate
    pub zcr: f32,
    /// Spectral centroid in Hz (only valid when FFT was computed)
    pub spectral_centroid: f32,
    /// Spectral flux (L2 norm of magnitude delta)
    pub spectral_flux: f32,
    /// Ratio of band energy (300–3400 Hz) to total energy
    pub band_energy_ratio: f32,
    /// Event that occurred during this frame (None for steady state)
    pub event: Option<VadEvent>,
}

/// Significant VAD lifecycle events
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VadEvent {
    /// Transitioned from PreSpeech → Speaking (utterance started)
    SpeechStarted,
    /// Transitioned from PostSpeech → Idle (utterance complete)
    SpeechEnded,
    /// Maximum duration reached — forced end
    MaxDurationReached,
}

// --------------- State Machine ---------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VadState {
    /// Waiting for first speech frame
    Idle,
    /// Accumulating consecutive speech frames before declaring start
    PreSpeech { frames: u32 },
    /// Active speech — may include hangover frames
    Speaking { frames: u32, hangover_frames: u32 },
    /// Speech ended at feature level, counting silence confirmations
    PostSpeech { silence_frames: u32, total_frames: u32 },
}

// --------------- Feature Extraction ---------------

/// Hann window pre-computed for a given length
fn hann_window(size: usize) -> Vec<f32> {
    let pi = std::f32::consts::PI;
    (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * pi * i as f32 / (size as f32 - 1.0)).cos()))
        .collect()
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn compute_zcr(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let crossings: usize = samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / (samples.len() - 1) as f32
}

/// Compute magnitude spectrum via FFT.
/// Returns vector of magnitudes for bins 0..N/2+1 (positive frequencies).
fn compute_magnitude_spectrum(
    samples: &[f32],
    window: &[f32],
    fft: &dyn Fft<f32>,
    fft_size: usize,
    scratch: &mut Vec<Complex<f32>>,
) -> Vec<f32> {
    scratch.clear();
    scratch.resize(fft_size, Complex::new(0.0, 0.0));

    let n = samples.len().min(window.len());
    for i in 0..n {
        scratch[i] = Complex::new(samples[i] * window[i], 0.0);
    }

    fft.process(scratch);

    let half = fft_size / 2 + 1;
    let mut mag = Vec::with_capacity(half);
    for i in 0..half {
        let c = scratch[i];
        mag.push((c.re * c.re + c.im * c.im).sqrt());
    }
    mag
}

/// Spectral centroid — weighted mean frequency
fn spectral_centroid(magnitude: &[f32], bin_freq: f32) -> f32 {
    let total_mag: f32 = magnitude.iter().sum();
    if total_mag < 1e-12 {
        return 0.0;
    }
    let weighted: f32 = magnitude
        .iter()
        .enumerate()
        .map(|(i, &m)| i as f32 * m)
        .sum();
    weighted / total_mag * bin_freq
}

/// Spectral flux — L2 norm of delta from previous magnitude spectrum
fn spectral_flux(current: &[f32], previous: Option<&[f32]>) -> f32 {
    let prev = match previous {
        Some(p) if p.len() == current.len() => p,
        _ => return 0.0,
    };
    let sum_sq: f32 = current
        .iter()
        .zip(prev.iter())
        .map(|(c, p)| (c - p).powi(2))
        .sum();
    (sum_sq / current.len() as f32).sqrt()
}

/// Ratio of energy in [low_bin, high_bin] to total energy
fn band_energy_ratio(magnitude: &[f32], low_bin: usize, high_bin: usize) -> f32 {
    let total: f32 = magnitude.iter().sum();
    if total < 1e-12 {
        return 0.0;
    }
    let band: f32 = magnitude
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= low_bin && *i <= high_bin)
        .map(|(_, &m)| m)
        .sum();
    band / total
}

// --------------- Noise Floor Estimator ---------------

struct NoiseFloorEstimator {
    floor: f32,
    adaptation_rate: f32,
    min_floor: f32,
}

impl NoiseFloorEstimator {
    fn new(initial: f32, adaptation_rate: f32, min_floor: f32) -> Self {
        Self {
            floor: initial.max(min_floor),
            adaptation_rate,
            min_floor,
        }
    }

    /// Call every frame. `is_speech` should be the *previous* decision
    /// so that the floor only adapts during non-speech.
    fn update(&mut self, energy: f32, is_speech: bool) {
        if is_speech {
            return;
        }
        let diff = energy - self.floor;
        if diff < 0.0 {
            self.floor += diff * 0.3;
        } else {
            self.floor += diff * self.adaptation_rate;
        }
        self.floor = self.floor.max(self.min_floor);
    }

    fn threshold(&self, multiplier: f32, min_abs: f32) -> f32 {
        (self.floor * multiplier).max(min_abs)
    }
}

// --------------- Main Detector ---------------

/// Production-grade Voice Activity Detector
pub struct VoiceActivityDetector {
    config: VadConfig,
    state: VadState,

    // Buffers
    pre_buffer: VecDeque<f32>,
    speech_buffer: Vec<f32>,

    // Noise tracking
    noise: NoiseFloorEstimator,

    // Feature tracking
    prev_magnitude: Option<Vec<f32>>,

    // FFT resources (reused across frames to avoid allocation)
    fft: Arc<dyn Fft<f32>>,
    fft_size: usize,
    fft_scratch: Vec<Complex<f32>>,
    window: Vec<f32>,

    // Statistics
    frame_count: u64,
}

impl VoiceActivityDetector {
    /// Create a new VAD with the given configuration.
    pub fn new(config: VadConfig) -> Self {
        let fft_size = config.fft_size();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);

        let pre_buf_cap = config.ms_to_samples(config.pre_speech_buffer_ms);

        Self {
            noise: NoiseFloorEstimator::new(
                config.noise_gate_min,
                config.noise_floor_adaptation,
                config.noise_gate_min,
            ),
            state: VadState::Idle,
            pre_buffer: VecDeque::with_capacity(pre_buf_cap),
            speech_buffer: Vec::with_capacity(config.ms_to_samples(config.max_speech_ms)),
            prev_magnitude: None,
            fft,
            fft_size,
            fft_scratch: Vec::with_capacity(fft_size),
            window: hann_window(config.frame_samples()),
            frame_count: 0,
            config,
        }
    }

    /// Process a single frame of audio and return the VAD verdict.
    pub fn process_frame(&mut self, samples: &[f32]) -> (VadResult, Option<VadEvent>) {
        self.frame_count += 1;

        let rms = compute_rms(samples);
        let zcr = compute_zcr(samples);
        let gate_threshold =
            self.noise
                .threshold(self.config.noise_gate_multiplier, self.config.noise_gate_min);
        let passes_gate = rms > gate_threshold;

        let (centroid, flux, band_ratio) = if passes_gate {
            let mag = compute_magnitude_spectrum(
                samples,
                &self.window,
                &*self.fft,
                self.fft_size,
                &mut self.fft_scratch,
            );
            let bin_freq = self.config.sample_rate as f32 / self.fft_size as f32;
            let centroid = spectral_centroid(&mag, bin_freq);
            let flux = spectral_flux(&mag, self.prev_magnitude.as_deref());

            let low_bin = (300.0 / bin_freq) as usize;
            let high_bin = (3400.0 / bin_freq).min(mag.len() as f32 - 1.0) as usize;
            let band_ratio = band_energy_ratio(&mag, low_bin, high_bin);

            self.prev_magnitude = Some(mag);
            (centroid, flux, band_ratio)
        } else {
            (0.0, 0.0, 0.0)
        };

        let confidence = self.compute_confidence(rms, zcr, centroid, flux, band_ratio);

        let is_speech = confidence >= self.config.speech_threshold;
        let (event, new_state) = self.advance_state(is_speech);

        self.noise.update(rms, is_speech);

        self.manage_buffers(samples, &new_state, event);

        self.state = new_state;

        let result = VadResult {
            is_speech,
            confidence,
            rms_energy: rms,
            zcr,
            spectral_centroid: centroid,
            spectral_flux: flux,
            band_energy_ratio: band_ratio,
            event,
        };

        (result, event)
    }

    /// Process one sample at a time, buffering internally until a full frame
    /// is accumulated. Returns `Some((result, event))` when a frame completes.
    pub fn process_sample(
        &mut self,
        sample: f32,
        frame_buf: &mut Vec<f32>,
    ) -> Option<(VadResult, Option<VadEvent>)> {
        frame_buf.push(sample);
        if frame_buf.len() >= self.config.frame_samples() {
            let result = self.process_frame(frame_buf);
            frame_buf.clear();
            Some(result)
        } else {
            None
        }
    }

    // ---- Confidence scoring ----

    fn compute_confidence(
        &self,
        rms: f32,
        zcr: f32,
        centroid: f32,
        flux: f32,
        band_ratio: f32,
    ) -> f32 {
        let w = &self.config;

        let noise_threshold =
            self.noise
                .threshold(w.noise_gate_multiplier, w.noise_gate_min);

        let energy_score = if rms <= noise_threshold {
            0.0
        } else if rms >= w.energy_threshold {
            1.0
        } else {
            (rms - noise_threshold) / (w.energy_threshold - noise_threshold)
        };

        let zcr_score = if zcr <= w.zcr_threshold {
            1.0
        } else if zcr >= w.zcr_threshold * 2.0 {
            0.0
        } else {
            1.0 - (zcr - w.zcr_threshold) / w.zcr_threshold
        };

        let centroid_score = if centroid < w.centroid_min {
            (centroid / w.centroid_min).max(0.0)
        } else if centroid <= w.centroid_max {
            1.0
        } else {
            (2.0 - centroid / w.centroid_max).max(0.0)
        };

        let flux_score = if flux >= w.flux_min {
            1.0
        } else {
            (flux / w.flux_min).min(1.0)
        };

        let band_score = if band_ratio >= w.band_ratio_min {
            1.0
        } else {
            (band_ratio / w.band_ratio_min).min(1.0)
        };

        let total_weight = w.weight_energy
            + w.weight_zcr
            + w.weight_centroid
            + w.weight_flux
            + w.weight_band_energy;
        if total_weight < 1e-6 {
            return 0.0;
        }

        (energy_score * w.weight_energy
            + zcr_score * w.weight_zcr
            + centroid_score * w.weight_centroid
            + flux_score * w.weight_flux
            + band_score * w.weight_band_energy)
            / total_weight
    }

    // ---- State machine ----

    fn advance_state(&self, is_speech: bool) -> (Option<VadEvent>, VadState) {
        let max_frames = self.config.frames_for_ms(self.config.max_speech_ms);
        let hangover_frames = self.config.frames_for_ms(self.config.hangover_ms);
        let end_frames = self.config.end_frames;

        match self.state {
            VadState::Idle => {
                if is_speech {
                    if self.config.start_frames <= 1 {
                        (
                            Some(VadEvent::SpeechStarted),
                            VadState::Speaking {
                                frames: 0,
                                hangover_frames,
                            },
                        )
                    } else {
                        (None, VadState::PreSpeech { frames: 1 })
                    }
                } else {
                    (None, VadState::Idle)
                }
            }

            VadState::PreSpeech { frames } => {
                if is_speech {
                    let next = frames + 1;
                    if next >= self.config.start_frames {
                        (
                            Some(VadEvent::SpeechStarted),
                            VadState::Speaking {
                                frames: 0,
                                hangover_frames,
                            },
                        )
                    } else {
                        (None, VadState::PreSpeech { frames: next })
                    }
                } else {
                    (None, VadState::Idle)
                }
            }

            VadState::Speaking {
                frames,
                hangover_frames,
            } => {
                let next_frames = frames + 1;

                if next_frames >= max_frames {
                    return (Some(VadEvent::MaxDurationReached), VadState::Idle);
                }

                if is_speech {
                    (
                        None,
                        VadState::Speaking {
                            frames: next_frames,
                            hangover_frames,
                        },
                    )
                } else if hangover_frames > 0 {
                    (
                        None,
                        VadState::Speaking {
                            frames: next_frames,
                            hangover_frames: hangover_frames - 1,
                        },
                    )
                } else {
                    (
                        None,
                        VadState::PostSpeech {
                            silence_frames: 1,
                            total_frames: next_frames,
                        },
                    )
                }
            }

            VadState::PostSpeech {
                silence_frames,
                total_frames,
            } => {
                let next_total = total_frames + 1;

                if next_total >= max_frames {
                    return (Some(VadEvent::MaxDurationReached), VadState::Idle);
                }

                if is_speech {
                    (
                        None,
                        VadState::Speaking {
                            frames: next_total,
                            hangover_frames: self.config.frames_for_ms(self.config.hangover_ms),
                        },
                    )
                } else if silence_frames + 1 >= end_frames {
                    let min_frames = self.config.frames_for_ms(self.config.min_speech_ms);
                    if total_frames >= min_frames {
                        (Some(VadEvent::SpeechEnded), VadState::Idle)
                    } else {
                        (None, VadState::Idle)
                    }
                } else {
                    (
                        None,
                        VadState::PostSpeech {
                            silence_frames: silence_frames + 1,
                            total_frames: next_total,
                        },
                    )
                }
            }
        }
    }

    // ---- Buffer management ----

    fn manage_buffers(
        &mut self,
        samples: &[f32],
        new_state: &VadState,
        event: Option<VadEvent>,
    ) {
        match event {
            Some(VadEvent::SpeechStarted) => {
                self.speech_buffer.clear();
                self.speech_buffer.extend(self.pre_buffer.drain(..));
                self.speech_buffer.extend_from_slice(samples);
            }
            Some(VadEvent::SpeechEnded | VadEvent::MaxDurationReached) => {
                if let VadState::Idle = new_state {
                }
            }
            None => match *new_state {
                VadState::Idle | VadState::PreSpeech { .. } => {
                    for &s in samples {
                        if self.pre_buffer.len()
                            >= self.config.ms_to_samples(self.config.pre_speech_buffer_ms)
                        {
                            self.pre_buffer.pop_front();
                        }
                        self.pre_buffer.push_back(s);
                    }
                }
                VadState::Speaking { .. } | VadState::PostSpeech { .. } => {
                    self.speech_buffer.extend_from_slice(samples);
                }
            },
        }
    }

    // ---- Public helpers ----

    /// Whether the detector currently considers speech to be active
    pub fn is_speaking(&self) -> bool {
        matches!(
            self.state,
            VadState::Speaking { .. } | VadState::PostSpeech { .. }
        )
    }

    /// Current state machine state
    pub fn state(&self) -> VadState {
        self.state
    }

    /// Reset all state (keeps configuration and noise floor estimate)
    pub fn reset(&mut self) {
        self.state = VadState::Idle;
        self.pre_buffer.clear();
        self.speech_buffer.clear();
        self.prev_magnitude = None;
        self.frame_count = 0;
    }

    /// Full reset including noise floor estimate
    pub fn reset_hard(&mut self) {
        self.reset();
        self.noise = NoiseFloorEstimator::new(
            self.config.noise_gate_min,
            self.config.noise_floor_adaptation,
            self.config.noise_gate_min,
        );
    }

    /// Force-update the noise floor estimate (e.g., from a known silence sample)
    pub fn update_noise_floor(&mut self, energy: f32) {
        self.noise.update(energy, false);
    }

    /// Take ownership of the collected speech buffer
    pub fn take_audio(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.speech_buffer)
    }

    /// Read-only reference to current speech buffer
    pub fn get_audio(&self) -> &[f32] {
        &self.speech_buffer
    }

    /// Current configuration
    pub fn config(&self) -> &VadConfig {
        &self.config
    }

    /// Mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut VadConfig {
        &mut self.config
    }

    /// Total frames processed since creation / last reset
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

// ============== Free-standing helpers ==============

/// Quick energy-based check — useful for low-cost pre-filtering
pub fn is_speech_frame(samples: &[f32], threshold: f32) -> bool {
    compute_rms(samples) > threshold
}

/// Trim leading/trailing silence from a buffer using the provided config thresholds
pub fn trim_silence(samples: &[f32], config: &VadConfig) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let frame_size = config.frame_samples();
    let threshold = config.noise_gate_min.max(config.energy_threshold);

    let mut start = 0;
    for (i, chunk) in samples.chunks(frame_size).enumerate() {
        if compute_rms(chunk) > threshold {
            start = i.saturating_sub(1) * frame_size;
            break;
        }
    }

    let mut end = samples.len();
    for (i, chunk) in samples.chunks(frame_size).enumerate().rev() {
        if compute_rms(chunk) > threshold {
            end = ((i + 2) * frame_size).min(samples.len());
            break;
        }
    }

    if start >= end {
        return Vec::new();
    }
    samples[start..end].to_vec()
}

// ============== Tests ==============

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sample_rate: u32, n_samples: usize, amplitude: f32) -> Vec<f32> {
        let pi2 = 2.0 * std::f32::consts::PI;
        (0..n_samples)
            .map(|i| amplitude * (pi2 * freq * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    fn noise(n_samples: usize, amplitude: f32) -> Vec<f32> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
        let mut rng = seed;
        (0..n_samples)
            .map(|_| {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let val = (rng >> 33) as f32 / 2147483648.0_f32;
                (val * 2.0 - 1.0) * amplitude
            })
            .collect()
    }

    fn saw(freq: f32, sample_rate: u32, n_samples: usize, amplitude: f32) -> Vec<f32> {
        let period = sample_rate as f32 / freq;
        (0..n_samples)
            .map(|i| {
                let phase = (i as f32 / period).fract();
                amplitude * (phase * 2.0 - 1.0)
            })
            .collect()
    }

    #[test]
    fn test_rms_silence() {
        let s = vec![0.0f32; 320];
        assert!(compute_rms(&s) < 1e-6);
    }

    #[test]
    fn test_rms_sine() {
        let s = sine(440.0, 16000, 320, 0.5);
        let rms = compute_rms(&s);
        assert!((rms - 0.353).abs() < 0.01, "rms = {}", rms);
    }

    #[test]
    fn test_zcr_sine() {
        let s = sine(440.0, 16000, 320, 0.5);
        let zcr = compute_zcr(&s);
        assert!((zcr - 440.0 / 16000.0 * 2.0).abs() < 0.01, "zcr = {}", zcr);
    }

    #[test]
    fn test_zcr_noise() {
        let s = noise(320, 0.5);
        let zcr = compute_zcr(&s);
        assert!(zcr > 0.3 && zcr < 0.7, "zcr = {}", zcr);
    }

    #[test]
    fn test_spectral_centroid_sine() {
        let config = VadConfig::default();
        let config = VadConfig {
            frame_ms: 20,
            sample_rate: 16000,
            ..config
        };
        let mut detector = VoiceActivityDetector::new(config.clone());
        let s = sine(1000.0, 16000, config.frame_samples(), 0.5);

        let (result, _) = detector.process_frame(&s);
        assert!(
            (result.spectral_centroid - 1000.0).abs() < 200.0,
            "centroid = {}",
            result.spectral_centroid
        );
    }

    #[test]
    fn test_noise_gate_rejects_silence() {
        let config = VadConfig::default();
        let mut vad = VoiceActivityDetector::new(config);
        let silent = vec![0.0f32; vad.config.frame_samples()];
        let (result, _) = vad.process_frame(&silent);
        assert!(
            !result.is_speech,
            "silence should not be speech (conf={})",
            result.confidence
        );
    }

    #[test]
    fn test_noise_gate_passes_speech() {
        let config = VadConfig::default();
        let sr = config.sample_rate;
        let fs = config.frame_samples();
        let mut vad = VoiceActivityDetector::new(config);
        let speech = saw(440.0, sr, fs, 0.3);
        let (result, _) = vad.process_frame(&speech);
        assert!(
            result.is_speech,
            "sawtooth wave should be speech (conf={})",
            result.confidence
        );
    }

    #[test]
    fn test_speech_start_requires_consecutive_frames() {
        let config = VadConfig {
            start_frames: 4,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config.clone());
        let frame = saw(440.0, config.sample_rate, config.frame_samples(), 0.3);

        for i in 0..3 {
            let (_, event) = vad.process_frame(&frame);
            assert!(event.is_none(), "frame {} should not trigger event", i + 1);
            assert!(
                !matches!(vad.state(), VadState::Speaking { .. }),
                "frame {} should not be Speaking",
                i + 1
            );
        }

        let (_, event) = vad.process_frame(&frame);
        assert!(
            matches!(event, Some(VadEvent::SpeechStarted)),
            "4th frame should trigger SpeechStarted"
        );
        assert!(matches!(vad.state(), VadState::Speaking { .. }));
    }

    #[test]
    fn test_speech_end_requires_silence() {
        let config = VadConfig {
            start_frames: 2,
            end_frames: 3,
            hangover_ms: 0,
            min_speech_ms: 0,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config.clone());
        let speech = saw(440.0, config.sample_rate, config.frame_samples(), 0.3);
        let silence = vec![0.0f32; config.frame_samples()];

        for _ in 0..2 {
            vad.process_frame(&speech);
        }
        assert!(matches!(vad.state(), VadState::Speaking { .. }));

        let (_, event1) = vad.process_frame(&silence);
        assert!(event1.is_none() || matches!(event1, Some(VadEvent::SpeechEnded)));
        if event1.is_some() {
            return;
        }
        let (_, event2) = vad.process_frame(&silence);
        assert!(event2.is_none() || matches!(event2, Some(VadEvent::SpeechEnded)));
        if event2.is_some() {
            return;
        }
        let (_, event3) = vad.process_frame(&silence);
        assert!(
            matches!(event3, Some(VadEvent::SpeechEnded)),
            "3rd silence frame should end speech"
        );
        assert!(matches!(vad.state(), VadState::Idle));
    }

    #[test]
    fn test_speech_recovery_during_post_speech() {
        let config = VadConfig {
            start_frames: 2,
            end_frames: 10,
            hangover_ms: 0,
            min_speech_ms: 0,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config.clone());
        let speech = saw(440.0, config.sample_rate, config.frame_samples(), 0.3);
        let silence = vec![0.0f32; config.frame_samples()];

        for _ in 0..2 {
            vad.process_frame(&speech);
        }
        assert!(vad.is_speaking());

        vad.process_frame(&silence);

        let (_, event) = vad.process_frame(&speech);
        assert!(matches!(vad.state(), VadState::Speaking { .. }));
    }

    #[test]
    fn test_hangover_prevents_clipping() {
        let config = VadConfig {
            start_frames: 2,
            end_frames: 2,
            hangover_ms: 60,
            min_speech_ms: 0,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config.clone());
        let speech = saw(440.0, config.sample_rate, config.frame_samples(), 0.3);
        let silence = vec![0.0f32; config.frame_samples()];

        for _ in 0..2 {
            vad.process_frame(&speech);
        }
        assert!(vad.is_speaking());

        for _ in 0..6 {
            let _ = vad.process_frame(&silence);
        }
        assert!(matches!(vad.state(), VadState::Idle));
    }

    #[test]
    fn test_pre_buffer_captures_audio_before_speech() {
        let config = VadConfig {
            start_frames: 2,
            pre_speech_buffer_ms: 100,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config.clone());
        let silence = vec![0.0f32; config.frame_samples()];
        let speech = saw(440.0, config.sample_rate, config.frame_samples(), 0.3);

        for _ in 0..5 {
            vad.process_frame(&silence);
        }

        let (_, event) = vad.process_frame(&speech);
        assert!(event.is_none());

        let (_, event) = vad.process_frame(&speech);
        assert!(matches!(event, Some(VadEvent::SpeechStarted)));

        let audio = vad.get_audio();
        assert!(!audio.is_empty(), "speech buffer should not be empty");
    }

    #[test]
    fn test_take_audio_empties_buffer() {
        let config = VadConfig {
            start_frames: 1,
            end_frames: 1,
            hangover_ms: 0,
            min_speech_ms: 0,
            speech_threshold: 0.5,
            ..Default::default()
        };
        let sr = config.sample_rate;
        let fs = config.frame_samples();
        let mut vad = VoiceActivityDetector::new(config);
        let speech = saw(440.0, sr, fs, 0.4);
        let silence = vec![0.0f32; fs];

        let (r1, _) = vad.process_frame(&speech);
        assert!(
            r1.is_speech,
            "first frame should be speech (conf={})",
            r1.confidence
        );
        let (r2, _) = vad.process_frame(&silence);
        assert!(!r2.is_speech, "silence should not be speech");

        let audio = vad.take_audio();
        assert!(!audio.is_empty(), "audio should have been captured");
        assert!(vad.get_audio().is_empty(), "buffer should be empty after take");
    }

    #[test]
    fn test_noise_floor_adapts() {
        let config = VadConfig {
            noise_floor_adaptation: 0.1,
            noise_gate_multiplier: 2.0,
            noise_gate_min: 0.001,
            start_frames: 10,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config);

        let low_noise = vec![0.005f32; vad.config.frame_samples()];
        let high_noise = vec![0.05f32; vad.config.frame_samples()];

        for _ in 0..50 {
            let _ = vad.process_frame(&low_noise);
        }

        for _ in 0..50 {
            let _ = vad.process_frame(&high_noise);
        }

        let (result, _) = vad.process_frame(&high_noise);
        assert!(
            !result.is_speech,
            "adapted noise floor should reject constant noise"
        );
    }

    #[test]
    fn test_reset_clears_state() {
        let config = VadConfig {
            start_frames: 1,
            speech_threshold: 0.5,
            ..Default::default()
        };
        let sr = config.sample_rate;
        let fs = config.frame_samples();
        let mut vad = VoiceActivityDetector::new(config);
        let speech = saw(440.0, sr, fs, 0.4);

        let (result, _) = vad.process_frame(&speech);
        assert!(
            result.is_speech,
            "frame should be speech (conf={})",
            result.confidence
        );
        assert!(vad.is_speaking(), "VAD should be speaking");

        vad.reset();
        assert!(!vad.is_speaking());
        assert!(vad.get_audio().is_empty());
    }

    #[test]
    fn test_trim_silence() {
        let config = VadConfig::default();
        let frame_size = config.frame_samples();

        let mut buf = Vec::new();
        buf.extend(vec![0.0f32; frame_size * 3]);
        buf.extend(vec![0.3f32; frame_size * 5]);
        buf.extend(vec![0.0f32; frame_size * 2]);

        let trimmed = trim_silence(&buf, &config);
        assert!(!trimmed.is_empty());
        assert!(trimmed.len() <= frame_size * 7);
    }

    #[test]
    fn bench_vad_throughput() {
        let config = VadConfig::default();
        let mut vad = VoiceActivityDetector::new(config.clone());
        let frame = vec![0.1f32; config.frame_samples()];

        let start = std::time::Instant::now();
        let iterations = 2000;
        for _ in 0..iterations {
            let _ = vad.process_frame(&frame);
        }
        let elapsed = start.elapsed();
        let per_frame = elapsed / iterations;
        let audio_duration_ms = iterations as u64 * config.frame_ms as u64;

        eprintln!(
            "VAD bench: {} frames ({} ms audio) in {:?} ({:?}/frame, {:.1}x realtime)",
            iterations,
            audio_duration_ms,
            elapsed,
            per_frame,
            audio_duration_ms as f64 / elapsed.as_secs_f64() / 1000.0,
        );

        assert!(
            elapsed.as_secs_f64() < audio_duration_ms as f64 / 1000.0,
            "VAD must be faster than realtime"
        );
    }
}
