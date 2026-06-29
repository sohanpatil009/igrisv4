// src/core/mod.rs - Core voice processing modules

pub mod stt;
pub mod tts;
pub mod vad;
pub mod wake_word;
pub mod audio_capture;
pub mod about;
#[cfg(feature = "candle")]
pub mod local_llm;

// Re-exports for convenience
pub use stt::{init_whisper_context, transcribe_audio, hybrid_transcribe_audio};
pub use tts::{speak, speak_compat, TTS_ENGINE};
pub use audio_capture::{capture_audio_vad, CaptureConfig, CaptureResult, CaptureMode};
pub use wake_word::listen_for_wake_word;
pub use about::{IgrisAbout, AboutSection, is_about_query, wants_detailed_info};
#[cfg(feature = "candle")]
pub use local_llm::{init_local_llm, is_local_llm_ready, global_reason, default_tool_system_prompt, parse_tool_call};
