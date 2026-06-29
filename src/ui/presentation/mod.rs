// src/ui/presentation/mod.rs
// IGRIS Self-Presentation UI with animated slides and TTS narration

pub mod slides;
pub mod panel;

pub use panel::{PresentationPanel, start_presentation, stop_presentation, next_slide, prev_slide, is_presentation_active, is_presentation_open};
pub use slides::{SLIDES, Slide, SlideContent};
