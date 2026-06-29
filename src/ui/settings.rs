// src/ui/settings.rs - Settings UI Panel
// Dioxus 0.7 component for user configuration

use dioxus::prelude::*;
use crate::config::{CONFIG, Personality, Theme};

/// Settings panel visibility state
#[derive(Clone, Copy, PartialEq)]
pub struct SettingsState {
    pub is_open: bool,
}

/// Settings Panel Component
#[component]
pub fn SettingsPanel(is_open: Signal<bool>) -> Element {
    // Local state for form values
    let config = CONFIG.get();
    let mut personality = use_signal(|| config.personality.clone());
    let mut sensitivity = use_signal(|| (config.recognition.sensitivity * 100.0) as i32);
    let mut volume = use_signal(|| (config.tts.volume * 100.0) as i32);
    let mut speed = use_signal(|| (config.tts.speed * 100.0) as i32);
    let mut show_logs = use_signal(|| config.ui.show_logs);
    let mut theme = use_signal(|| config.ui.theme.clone());
    let mut save_status = use_signal(|| String::new());

    // Don't render if not open
    if !is_open() {
        return rsx! {};
    }

    let current_personality = personality();
    let current_theme = theme();

    rsx! {
        // Overlay backdrop
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.7); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| is_open.set(false),

            // Settings modal
            div {
                style: "background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); border: 1px solid rgba(6, 182, 212, 0.3); border-radius: 16px; padding: 24px; width: 480px; max-height: 80vh; overflow-y: auto; box-shadow: 0 20px 60px rgba(0,0,0,0.5);",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; padding-bottom: 16px; border-bottom: 1px solid rgba(255,255,255,0.1);",
                    
                    h2 {
                        style: "font-size: 24px; font-weight: 700; color: #06b6d4; margin: 0;",
                        "⚙️ Settings"
                    }
                    
                    button {
                        style: "background: none; border: none; color: #9ca3af; font-size: 24px; cursor: pointer; padding: 4px 8px; border-radius: 4px;",
                        onclick: move |_| is_open.set(false),
                        "✕"
                    }
                }

                // Personality Section
                div {
                    style: "margin-bottom: 24px;",
                    
                    h3 {
                        style: "font-size: 14px; font-weight: 600; color: #9ca3af; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px;",
                        "🎭 Personality"
                    }
                    
                    div {
                        style: "display: flex; gap: 12px;",
                        
                        // IGRIS option
                        div {
                            style: format!(
                                "flex: 1; padding: 16px; border-radius: 12px; cursor: pointer; transition: all 0.2s; border: 2px solid {}; background: {};",
                                if current_personality == Personality::Igris { "#06b6d4" } else { "rgba(255,255,255,0.1)" },
                                if current_personality == Personality::Igris { "rgba(6, 182, 212, 0.1)" } else { "transparent" }
                            ),
                            onclick: move |_| personality.set(Personality::Igris),
                            
                            div { style: "font-size: 24px; margin-bottom: 8px;", "🗡️" }
                            div { style: "font-size: 16px; font-weight: 600; color: #fff;", "IGRIS" }
                            div { style: "font-size: 12px; color: #9ca3af; margin-top: 4px;", "Deep voice • Wake: \"Arise\" or \"Hey IGRIS\"" }
                        }
                        
                        // Alita option
                        div {
                            style: format!(
                                "flex: 1; padding: 16px; border-radius: 12px; cursor: pointer; transition: all 0.2s; border: 2px solid {}; background: {};",
                                if current_personality == Personality::Alita { "#ec4899" } else { "rgba(255,255,255,0.1)" },
                                if current_personality == Personality::Alita { "rgba(236, 72, 153, 0.1)" } else { "transparent" }
                            ),
                            onclick: move |_| personality.set(Personality::Alita),
                            
                            div { style: "font-size: 24px; margin-bottom: 8px;", "🌸" }
                            div { style: "font-size: 16px; font-weight: 600; color: #fff;", "Alita" }
                            div { style: "font-size: 12px; color: #9ca3af; margin-top: 4px;", "Friendly buddy • Wake: \"Hey Alita\"" }
                        }
                    }
                }

                // Voice Recognition Section
                div {
                    style: "margin-bottom: 24px;",
                    
                    h3 {
                        style: "font-size: 14px; font-weight: 600; color: #9ca3af; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px;",
                        "🎤 Voice Recognition"
                    }
                    
                    // Sensitivity slider
                    div {
                        style: "margin-bottom: 16px;",
                        
                        div {
                            style: "display: flex; justify-content: space-between; margin-bottom: 8px;",
                            span { style: "color: #e5e7eb; font-size: 14px;", "Sensitivity" }
                            span { style: "color: #06b6d4; font-size: 14px; font-weight: 600;", "{sensitivity()}%" }
                        }
                        
                        input {
                            r#type: "range",
                            min: "10",
                            max: "90",
                            value: "{sensitivity()}",
                            style: "width: 100%; accent-color: #06b6d4;",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<i32>() {
                                    sensitivity.set(val);
                                }
                            }
                        }
                        
                        div {
                            style: "display: flex; justify-content: space-between; font-size: 11px; color: #6b7280; margin-top: 4px;",
                            span { "Less sensitive" }
                            span { "More sensitive" }
                        }
                    }
                }

                // Text-to-Speech Section
                div {
                    style: "margin-bottom: 24px;",
                    
                    h3 {
                        style: "font-size: 14px; font-weight: 600; color: #9ca3af; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px;",
                        "🔊 Text-to-Speech"
                    }
                    
                    // Volume slider
                    div {
                        style: "margin-bottom: 16px;",
                        
                        div {
                            style: "display: flex; justify-content: space-between; margin-bottom: 8px;",
                            span { style: "color: #e5e7eb; font-size: 14px;", "Volume" }
                            span { style: "color: #06b6d4; font-size: 14px; font-weight: 600;", "{volume()}%" }
                        }
                        
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{volume()}",
                            style: "width: 100%; accent-color: #06b6d4;",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<i32>() {
                                    volume.set(val);
                                }
                            }
                        }
                    }
                    
                    // Speed slider
                    div {
                        style: "margin-bottom: 16px;",
                        
                        div {
                            style: "display: flex; justify-content: space-between; margin-bottom: 8px;",
                            span { style: "color: #e5e7eb; font-size: 14px;", "Speed" }
                            span { style: "color: #06b6d4; font-size: 14px; font-weight: 600;", "{speed() as f32 / 100.0:.1}x" }
                        }
                        
                        input {
                            r#type: "range",
                            min: "50",
                            max: "200",
                            value: "{speed()}",
                            style: "width: 100%; accent-color: #06b6d4;",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<i32>() {
                                    speed.set(val);
                                }
                            }
                        }
                        
                        div {
                            style: "display: flex; justify-content: space-between; font-size: 11px; color: #6b7280; margin-top: 4px;",
                            span { "Slower" }
                            span { "Faster" }
                        }
                    }
                }

                // Appearance Section
                div {
                    style: "margin-bottom: 24px;",
                    
                    h3 {
                        style: "font-size: 14px; font-weight: 600; color: #9ca3af; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px;",
                        "🎨 Appearance"
                    }
                    
                    // Theme selection
                    div {
                        style: "display: flex; gap: 8px; margin-bottom: 16px;",
                        
                        button {
                            style: format!(
                                "flex: 1; padding: 10px; border-radius: 8px; border: 2px solid {}; background: {}; color: #fff; cursor: pointer; font-size: 13px;",
                                if current_theme == Theme::Dark { "#06b6d4" } else { "rgba(255,255,255,0.1)" },
                                if current_theme == Theme::Dark { "rgba(6, 182, 212, 0.1)" } else { "transparent" }
                            ),
                            onclick: move |_| theme.set(Theme::Dark),
                            "🌙 Dark"
                        }
                        
                        button {
                            style: format!(
                                "flex: 1; padding: 10px; border-radius: 8px; border: 2px solid {}; background: {}; color: #fff; cursor: pointer; font-size: 13px;",
                                if current_theme == Theme::Light { "#f59e0b" } else { "rgba(255,255,255,0.1)" },
                                if current_theme == Theme::Light { "rgba(245, 158, 11, 0.1)" } else { "transparent" }
                            ),
                            onclick: move |_| theme.set(Theme::Light),
                            "☀️ Light"
                        }
                        
                        button {
                            style: format!(
                                "flex: 1; padding: 10px; border-radius: 8px; border: 2px solid {}; background: {}; color: #fff; cursor: pointer; font-size: 13px;",
                                if current_theme == Theme::Cyber { "#8b5cf6" } else { "rgba(255,255,255,0.1)" },
                                if current_theme == Theme::Cyber { "rgba(139, 92, 246, 0.1)" } else { "transparent" }
                            ),
                            onclick: move |_| theme.set(Theme::Cyber),
                            "💜 Cyber"
                        }
                    }
                    
                    // Show logs toggle
                    div {
                        style: "display: flex; align-items: center; justify-content: space-between; padding: 12px; background: rgba(255,255,255,0.05); border-radius: 8px;",
                        
                        span { style: "color: #e5e7eb; font-size: 14px;", "Show Logs Panel" }
                        
                        button {
                            style: format!(
                                "width: 48px; height: 26px; border-radius: 13px; border: none; cursor: pointer; position: relative; transition: all 0.2s; background: {};",
                                if show_logs() { "#06b6d4" } else { "#374151" }
                            ),
                            onclick: move |_| show_logs.set(!show_logs()),
                            
                            div {
                                style: format!(
                                    "width: 20px; height: 20px; border-radius: 50%; background: #fff; position: absolute; top: 3px; transition: all 0.2s; left: {};",
                                    if show_logs() { "25px" } else { "3px" }
                                ),
                            }
                        }
                    }
                }

                // Save status message
                if !save_status().is_empty() {
                    div {
                        style: "padding: 12px; background: rgba(34, 197, 94, 0.1); border: 1px solid rgba(34, 197, 94, 0.3); border-radius: 8px; margin-bottom: 16px; color: #22c55e; font-size: 14px; text-align: center;",
                        "{save_status()}"
                    }
                }

                // Action buttons
                div {
                    style: "display: flex; gap: 12px; padding-top: 16px; border-top: 1px solid rgba(255,255,255,0.1);",
                    
                    button {
                        style: "flex: 1; padding: 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.2); background: transparent; color: #9ca3af; cursor: pointer; font-size: 14px; font-weight: 500;",
                        onclick: move |_| {
                            let _ = CONFIG.reset();
                            // Reload values
                            let config = CONFIG.get();
                            personality.set(config.personality);
                            sensitivity.set((config.recognition.sensitivity * 100.0) as i32);
                            volume.set((config.tts.volume * 100.0) as i32);
                            speed.set((config.tts.speed * 100.0) as i32);
                            show_logs.set(config.ui.show_logs);
                            theme.set(config.ui.theme);
                            save_status.set("Reset to defaults!".to_string());
                        },
                        "Reset Defaults"
                    }
                    
                    button {
                        style: "flex: 1; padding: 12px; border-radius: 8px; border: none; background: linear-gradient(135deg, #06b6d4, #3b82f6); color: #fff; cursor: pointer; font-size: 14px; font-weight: 600;",
                        onclick: move |_| {
                            // Save all settings
                            let result = CONFIG.update(|c| {
                                c.personality = personality();
                                c.recognition.sensitivity = sensitivity() as f32 / 100.0;
                                c.tts.volume = volume() as f32 / 100.0;
                                c.tts.speed = speed() as f32 / 100.0;
                                c.ui.show_logs = show_logs();
                                c.ui.theme = theme();
                            });
                            
                            match result {
                                Ok(_) => save_status.set("✓ Settings saved!".to_string()),
                                Err(e) => save_status.set(format!("Error: {}", e)),
                            }
                        },
                        "💾 Save Settings"
                    }
                }
            }
        }
    }
}

/// Settings button component (to add to main UI)
#[component]
pub fn SettingsButton(is_open: Signal<bool>) -> Element {
    rsx! {
        button {
            style: "position: fixed; top: 24px; right: 24px; z-index: 50; background: rgba(6, 182, 212, 0.1); border: 1px solid rgba(6, 182, 212, 0.3); border-radius: 8px; padding: 8px 12px; color: #06b6d4; cursor: pointer; font-size: 18px; display: flex; align-items: center; transition: all 0.2s;",
            onclick: move |_| is_open.set(true),
            
            span { "⚙️" }
        }
    }
}
