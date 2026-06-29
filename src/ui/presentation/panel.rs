// src/ui/presentation/panel.rs
// Animated presentation panel with TTS narration

use dioxus::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::ui::presentation::slides::{SLIDES, SlideContent};

/// Global presentation state
pub static PRESENTATION_OPEN: AtomicBool = AtomicBool::new(false);
pub static CURRENT_SLIDE: AtomicUsize = AtomicUsize::new(0);
pub static IS_PLAYING: AtomicBool = AtomicBool::new(false);

/// Open presentation and start narration
pub fn start_presentation() {
    PRESENTATION_OPEN.store(true, Ordering::SeqCst);
    CURRENT_SLIDE.store(0, Ordering::SeqCst);
    IS_PLAYING.store(true, Ordering::SeqCst);
    
    // Start narration in background
    std::thread::spawn(|| {
        narrate_presentation();
    });
}

/// Close presentation
pub fn stop_presentation() {
    PRESENTATION_OPEN.store(false, Ordering::SeqCst);
    IS_PLAYING.store(false, Ordering::SeqCst);
}

/// Check if presentation is currently active (open and playing)
/// Used to pause voice listening during presentation
pub fn is_presentation_active() -> bool {
    PRESENTATION_OPEN.load(Ordering::SeqCst) && IS_PLAYING.load(Ordering::SeqCst)
}

/// Check if presentation is open (even if paused)
pub fn is_presentation_open() -> bool {
    PRESENTATION_OPEN.load(Ordering::SeqCst)
}

/// Narrate all slides with TTS
fn narrate_presentation() {
    use crate::core::tts::speak;
    
    for (i, slide) in SLIDES.iter().enumerate() {
        if !IS_PLAYING.load(Ordering::SeqCst) {
            break;
        }
        
        // Update current slide
        CURRENT_SLIDE.store(i, Ordering::SeqCst);
        
        // Speak narration
        if let Err(e) = speak(slide.narration) {
            eprintln!("[PRESENTATION] TTS error: {}", e);
        }
        
        // Pause between slides
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    
    IS_PLAYING.store(false, Ordering::SeqCst);
}

/// Go to next slide
pub fn next_slide() {
    let current = CURRENT_SLIDE.load(Ordering::SeqCst);
    if current < SLIDES.len() - 1 {
        CURRENT_SLIDE.store(current + 1, Ordering::SeqCst);
    }
}

/// Go to previous slide
pub fn prev_slide() {
    let current = CURRENT_SLIDE.load(Ordering::SeqCst);
    if current > 0 {
        CURRENT_SLIDE.store(current - 1, Ordering::SeqCst);
    }
}

/// Presentation Panel Component
#[component]
pub fn PresentationPanel() -> Element {
    let mut is_open = use_signal(|| PRESENTATION_OPEN.load(Ordering::SeqCst));
    let mut current_slide = use_signal(|| CURRENT_SLIDE.load(Ordering::SeqCst));
    let mut is_playing = use_signal(|| IS_PLAYING.load(Ordering::SeqCst));
    
    // Poll for state changes - only when presentation might be active
    use_effect(move || {
        spawn(async move {
            loop {
                // Check less frequently when closed, more when open
                let delay = if PRESENTATION_OPEN.load(Ordering::SeqCst) { 200 } else { 1000 };
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                
                let open = PRESENTATION_OPEN.load(Ordering::SeqCst);
                is_open.set(open);
                
                // Only update other states if open
                if open {
                    current_slide.set(CURRENT_SLIDE.load(Ordering::SeqCst));
                    is_playing.set(IS_PLAYING.load(Ordering::SeqCst));
                }
            }
        });
    });
    
    let open = is_open();
    if !open {
        return rsx! { div { style: "display: none;" } };
    }
    
    let slide_idx = current_slide();
    let slide = &SLIDES[slide_idx];
    let total_slides = SLIDES.len();
    let progress_pct = ((slide_idx + 1) as f32 / total_slides as f32) * 100.0;
    let progress_str = format!("{}%", progress_pct as i32);
    let prev_opacity = if slide_idx == 0 { "0.5" } else { "1.0" };
    let next_opacity = if slide_idx == total_slides - 1 { "0.5" } else { "1.0" };
    let playing = is_playing();
    
    rsx! {
        // Full screen overlay
        div {
            style: "
                position: fixed;
                top: 0;
                left: 0;
                width: 100vw;
                height: 100vh;
                background: linear-gradient(135deg, #0a0a0a 0%, #1a1a2e 100%);
                z-index: 9999;
                display: flex;
                flex-direction: column;
            ",
            
            // Progress bar - IGRIS purple theme
            div {
                style: "
                    position: absolute;
                    top: 0;
                    left: 0;
                    height: 4px;
                    background: linear-gradient(90deg, #a855f7, #7c3aed);
                    transition: width 0.5s ease;
                    width: {progress_str};
                ",
            }
            
            // Close button
            button {
                style: "
                    position: absolute;
                    top: 20px;
                    right: 20px;
                    background: rgba(168, 85, 247, 0.2);
                    border: 1px solid #a855f7;
                    color: white;
                    font-size: 24px;
                    width: 40px;
                    height: 40px;
                    border-radius: 50%;
                    cursor: pointer;
                    z-index: 10001;
                    transition: all 0.3s ease;
                ",
                onclick: move |_| {
                    stop_presentation();
                },
                "X"
            }

            // Main slide content
            div {
                style: "
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    justify-content: center;
                    align-items: center;
                    padding: 60px;
                ",
                
                // Icon with glow effect
                div {
                    style: "
                        font-size: 80px; 
                        margin-bottom: 20px;
                        filter: drop-shadow(0 0 20px rgba(168, 85, 247, 0.5));
                    ",
                    "{slide.icon}"
                }
                
                // Title - IGRIS purple gradient
                h1 {
                    style: "
                        font-size: 3rem;
                        margin-bottom: 30px;
                        background: linear-gradient(90deg, #a855f7, #7c3aed);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        background-clip: text;
                        text-align: center;
                        text-shadow: 0 0 30px rgba(168, 85, 247, 0.3);
                    ",
                    "{slide.title}"
                }
                
                // Bullet points
                div {
                    style: "
                        display: flex;
                        flex-direction: column;
                        gap: 15px;
                        max-width: 800px;
                    ",
                    for point in slide.bullet_points.iter() {
                        div {
                            style: "
                                background: rgba(168, 85, 247, 0.05);
                                border: 1px solid rgba(168, 85, 247, 0.3);
                                border-radius: 12px;
                                padding: 15px 25px;
                                font-size: 1.2rem;
                                color: #e0e0e0;
                                transition: all 0.3s ease;
                            ",
                            "{point}"
                        }
                    }
                }
                
                // Diagram based on slide type
                {render_slide_diagram(slide.content.clone())}
            }

            // Navigation
            div {
                style: "
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    gap: 20px;
                    padding: 30px;
                ",
                
                // Previous button - IGRIS purple
                button {
                    style: "
                        background: linear-gradient(90deg, #a855f7, #7c3aed);
                        border: none;
                        color: white;
                        padding: 12px 30px;
                        border-radius: 25px;
                        font-size: 1rem;
                        cursor: pointer;
                        opacity: {prev_opacity};
                        box-shadow: 0 0 20px rgba(168, 85, 247, 0.3);
                        transition: all 0.3s ease;
                    ",
                    disabled: slide_idx == 0,
                    onclick: move |_| { prev_slide(); },
                    "Previous"
                }
                
                // Slide counter
                span {
                    style: "color: #a855f7; font-size: 1rem; font-weight: bold;",
                    "{slide_idx + 1} / {total_slides}"
                }
                
                // Next button - IGRIS purple
                button {
                    style: "
                        background: linear-gradient(90deg, #a855f7, #7c3aed);
                        border: none;
                        color: white;
                        padding: 12px 30px;
                        border-radius: 25px;
                        font-size: 1rem;
                        cursor: pointer;
                        opacity: {next_opacity};
                        box-shadow: 0 0 20px rgba(168, 85, 247, 0.3);
                        transition: all 0.3s ease;
                    ",
                    disabled: slide_idx == total_slides - 1,
                    onclick: move |_| { next_slide(); },
                    "Next"
                }
                
                // Play/Pause button
                button {
                    style: "
                        background: rgba(168, 85, 247, 0.1);
                        border: 1px solid #a855f7;
                        color: #a855f7;
                        padding: 12px 20px;
                        border-radius: 25px;
                        font-size: 1rem;
                        cursor: pointer;
                        margin-left: 20px;
                        transition: all 0.3s ease;
                    ",
                    onclick: move |_| {
                        let p = IS_PLAYING.load(Ordering::SeqCst);
                        if p {
                            IS_PLAYING.store(false, Ordering::SeqCst);
                        } else {
                            IS_PLAYING.store(true, Ordering::SeqCst);
                            std::thread::spawn(|| narrate_presentation());
                        }
                    },
                    if playing { "Pause" } else { "Play" }
                }
            }
        }
    }
}

/// Render diagram based on slide content - IGRIS Purple Theme
fn render_slide_diagram(content: SlideContent) -> Element {
    match content {
        SlideContent::Architecture => rsx! {
            div {
                style: "display: flex; align-items: center; justify-content: center; gap: 20px; margin-top: 30px; flex-wrap: wrap;",
                
                div { style: "background: #1a1a2e; border: 2px solid #a855f7; border-radius: 12px; padding: 20px; text-align: center; min-width: 100px; box-shadow: 0 0 15px rgba(168, 85, 247, 0.3);",
                    div { style: "font-size: 24px;", "Mic" }
                    div { style: "color: #a855f7; font-size: 12px; margin-top: 8px;", "Input" }
                }
                div { style: "color: #a855f7; font-size: 24px;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #7c3aed; border-radius: 12px; padding: 20px; text-align: center; min-width: 100px; box-shadow: 0 0 15px rgba(124, 58, 237, 0.3);",
                    div { style: "font-size: 24px;", "Gear" }
                    div { style: "color: #7c3aed; font-size: 12px; margin-top: 8px;", "Core" }
                }
                div { style: "color: #a855f7; font-size: 24px;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #22c55e; border-radius: 12px; padding: 20px; text-align: center; min-width: 100px; box-shadow: 0 0 15px rgba(34, 197, 94, 0.3);",
                    div { style: "font-size: 24px;", "Plug" }
                    div { style: "color: #22c55e; font-size: 12px; margin-top: 8px;", "Plugins" }
                }
                div { style: "color: #a855f7; font-size: 24px;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #ef4444; border-radius: 12px; padding: 20px; text-align: center; min-width: 100px; box-shadow: 0 0 15px rgba(239, 68, 68, 0.3);",
                    div { style: "font-size: 24px;", "Out" }
                    div { style: "color: #ef4444; font-size: 12px; margin-top: 8px;", "Output" }
                }
            }
        },
        SlideContent::VoiceFlow => rsx! {
            div {
                style: "display: flex; align-items: center; justify-content: center; gap: 10px; margin-top: 30px; flex-wrap: wrap;",
                
                div { style: "background: #1a1a2e; border: 2px solid #a855f7; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(168, 85, 247, 0.3);",
                    div { style: "font-size: 20px;", "Mic" }
                    div { style: "color: #a855f7; font-size: 10px;", "Audio" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #7c3aed; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(124, 58, 237, 0.3);",
                    div { style: "font-size: 20px;", "Wave" }
                    div { style: "color: #7c3aed; font-size: 10px;", "VAD" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #22c55e; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(34, 197, 94, 0.3);",
                    div { style: "font-size: 20px;", "Ear" }
                    div { style: "color: #22c55e; font-size: 10px;", "Wake" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #f59e0b; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(245, 158, 11, 0.3);",
                    div { style: "font-size: 20px;", "Text" }
                    div { style: "color: #f59e0b; font-size: 10px;", "STT" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #ef4444; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(239, 68, 68, 0.3);",
                    div { style: "font-size: 20px;", "Brain" }
                    div { style: "color: #ef4444; font-size: 10px;", "NLU" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #8b5cf6; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(139, 92, 246, 0.3);",
                    div { style: "font-size: 20px;", "Bolt" }
                    div { style: "color: #8b5cf6; font-size: 10px;", "Action" }
                }
            }
        },
        SlideContent::NluModule => rsx! {
            div {
                style: "display: flex; flex-direction: column; align-items: center; gap: 15px; margin-top: 30px;",
                
                div { style: "background: #1a1a2e; border: 2px solid #a855f7; border-radius: 10px; padding: 12px 25px; box-shadow: 0 0 12px rgba(168, 85, 247, 0.3); color: #e0e0e0;",
                    "\"Open Chrome\""
                }
                div { style: "color: #a855f7; font-size: 20px;", "v" }
                div { style: "display: flex; gap: 15px;",
                    div { style: "background: #1a1a2e; border: 2px solid #22c55e; border-radius: 10px; padding: 12px; text-align: center; box-shadow: 0 0 12px rgba(34, 197, 94, 0.3);",
                        span { style: "color: #22c55e;", "SBERT" }
                    }
                    div { style: "background: #1a1a2e; border: 2px solid #f59e0b; border-radius: 10px; padding: 12px; text-align: center; box-shadow: 0 0 12px rgba(245, 158, 11, 0.3);",
                        span { style: "color: #f59e0b;", "NER" }
                    }
                    div { style: "background: #1a1a2e; border: 2px solid #8b5cf6; border-radius: 10px; padding: 12px; text-align: center; box-shadow: 0 0 12px rgba(139, 92, 246, 0.3);",
                        span { style: "color: #8b5cf6;", "Context" }
                    }
                }
                div { style: "color: #a855f7; font-size: 20px;", "v" }
                div { style: "background: #1a1a2e; border: 2px solid #ef4444; border-radius: 10px; padding: 12px 25px; box-shadow: 0 0 12px rgba(239, 68, 68, 0.3); color: #e0e0e0;",
                    "open_app: chrome"
                }
            }
        },
        SlideContent::FileShare => rsx! {
            div {
                style: "display: flex; align-items: center; justify-content: center; gap: 25px; margin-top: 30px;",
                
                div { style: "background: #1a1a2e; border: 2px solid #a855f7; border-radius: 12px; padding: 20px; text-align: center; box-shadow: 0 0 15px rgba(168, 85, 247, 0.3);",
                    div { style: "font-size: 28px; color: #a855f7;", "PC" }
                    div { style: "color: #a855f7; margin-top: 8px;", "Device A" }
                    div { style: "color: #22c55e; font-size: 10px;", "TLS" }
                }
                div { style: "border: 2px dashed #7c3aed; border-radius: 15px; padding: 15px 30px; text-align: center; box-shadow: 0 0 12px rgba(124, 58, 237, 0.2);",
                    div { style: "color: #7c3aed;", "LAN" }
                    div { style: "color: #a855f7; font-size: 10px;", "Encrypted" }
                }
                div { style: "background: #1a1a2e; border: 2px solid #22c55e; border-radius: 12px; padding: 20px; text-align: center; box-shadow: 0 0 15px rgba(34, 197, 94, 0.3);",
                    div { style: "font-size: 28px; color: #22c55e;", "Linux" }
                    div { style: "color: #22c55e; margin-top: 8px;", "Device B" }
                    div { style: "color: #22c55e; font-size: 10px;", "TLS" }
                }
            }
        },
        SlideContent::SetupManager => rsx! {
            div {
                style: "display: flex; align-items: center; justify-content: center; gap: 15px; margin-top: 30px; flex-wrap: wrap;",
                
                div { style: "background: #1a1a2e; border: 2px solid #a855f7; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(168, 85, 247, 0.3);",
                    div { style: "font-size: 20px; color: #a855f7;", "Launch" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #f59e0b; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(245, 158, 11, 0.3);",
                    div { style: "font-size: 20px; color: #f59e0b;", "Download" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #22c55e; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(34, 197, 94, 0.3);",
                    div { style: "font-size: 20px; color: #22c55e;", "Extract" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #ef4444; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(239, 68, 68, 0.3);",
                    div { style: "font-size: 20px; color: #ef4444;", "Validate" }
                }
                div { style: "color: #a855f7;", "->" }
                div { style: "background: #1a1a2e; border: 2px solid #8b5cf6; border-radius: 10px; padding: 15px; text-align: center; box-shadow: 0 0 12px rgba(139, 92, 246, 0.3);",
                    div { style: "font-size: 20px; color: #8b5cf6;", "Ready" }
                }
            }
        },
        _ => rsx! { div {} }
    }
}
