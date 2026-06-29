// src/setup_manager/gui.rs
// Dioxus GUI for setup progress with download progress bars

use crate::setup_manager::SetupEvent;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Setup state for GUI rendering
#[derive(Clone, Debug, Default)]
pub struct SetupState {
    pub is_complete: bool,
    pub has_error: bool,
    pub error_message: String,
    pub current_phase: SetupPhase,
    pub downloads: HashMap<String, DownloadProgress>,
    pub extractions: HashMap<String, u32>,
    pub validations: Vec<String>,
    pub logs: Vec<SetupLog>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum SetupPhase {
    #[default]
    Initializing,
    Downloading,
    Extracting,
    Validating,
    Complete,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadProgress {
    pub name: String,
    pub progress: u32,
    pub is_complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetupLog {
    pub message: String,
    pub log_type: LogType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogType {
    Info,
    Success,
    Warning,
    Error,
}

/// Global setup state for UI updates
pub static SETUP_STATE: once_cell::sync::Lazy<Arc<Mutex<SetupState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(SetupState::default())));

/// Legacy SetupUI for compatibility - now updates the global state
pub struct SetupUI {
    receiver: mpsc::UnboundedReceiver<SetupEvent>,
}

impl SetupUI {
    pub fn new(receiver: mpsc::UnboundedReceiver<SetupEvent>) -> Self {
        Self { receiver }
    }

    pub async fn run(&mut self) {
        // Initialize state
        {
            let mut state = SETUP_STATE.lock().unwrap();
            state.current_phase = SetupPhase::Initializing;
            state.logs.push(SetupLog {
                message: "Starting IGRIS setup...".to_string(),
                log_type: LogType::Info,
            });
        }

        while let Some(event) = self.receiver.recv().await {
            let mut state = SETUP_STATE.lock().unwrap();

            match event {
                SetupEvent::Started => {
                    state.current_phase = SetupPhase::Downloading;
                    state.logs.push(SetupLog {
                        message: "Setup started - downloading components...".to_string(),
                        log_type: LogType::Info,
                    });
                }
                SetupEvent::Downloading { name, progress } => {
                    state.current_phase = SetupPhase::Downloading;
                    state.downloads.insert(
                        name.clone(),
                        DownloadProgress {
                            name: name.clone(),
                            progress,
                            is_complete: progress >= 100,
                        },
                    );

                    if progress >= 100 {
                        state.logs.push(SetupLog {
                            message: format!("Downloaded: {}", name),
                            log_type: LogType::Success,
                        });
                    }
                }
                SetupEvent::Extracting { name, progress } => {
                    state.current_phase = SetupPhase::Extracting;
                    state.extractions.insert(name.clone(), progress);

                    if progress >= 100 {
                        state.logs.push(SetupLog {
                            message: format!("Extracted: {}", name),
                            log_type: LogType::Success,
                        });
                    }
                }
                SetupEvent::Validating { name } => {
                    state.current_phase = SetupPhase::Validating;
                    state.logs.push(SetupLog {
                        message: format!("Validating: {}", name),
                        log_type: LogType::Info,
                    });
                }
                SetupEvent::Completed { name } => {
                    state.validations.push(name.clone());
                    state.logs.push(SetupLog {
                        message: format!("Validated: {}", name),
                        log_type: LogType::Success,
                    });
                }
                SetupEvent::Error { task, message } => {
                    state.current_phase = SetupPhase::Error;
                    state.has_error = true;
                    state.error_message = format!("{}: {}", task, message);
                    state.logs.push(SetupLog {
                        message: format!("Error in {}: {}", task, message),
                        log_type: LogType::Error,
                    });
                }
                SetupEvent::AllComplete => {
                    state.current_phase = SetupPhase::Complete;
                    state.is_complete = true;
                    state.logs.push(SetupLog {
                        message: "Setup completed successfully!".to_string(),
                        log_type: LogType::Success,
                    });
                    break;
                }
            }
        }
    }
}

/// Get the current setup state for rendering
pub fn get_setup_state() -> SetupState {
    SETUP_STATE.lock().unwrap().clone()
}

/// Check if setup is complete
pub fn is_setup_complete() -> bool {
    SETUP_STATE.lock().unwrap().is_complete
}

/// Dioxus Setup GUI Component with Permissions
#[component]
pub fn SetupGui() -> Element {
    let mut update_trigger = use_signal(|| 0);
    let mut setup_state = use_signal(SetupState::default);
    let mut show_permissions = use_signal(|| true);  // Show permissions first
    let mut permissions_accepted = use_signal(|| false);

    // Periodic UI refresh to update from SETUP_STATE
    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(100)).await;
                update_trigger.set(update_trigger() + 1);

                let state = get_setup_state();
                setup_state.set(state);
            }
        });
    });

    let _ = update_trigger();
    let state = setup_state();

    // If permissions not accepted yet, show permissions panel
    if show_permissions() && !permissions_accepted() {
        return rsx! {
            crate::setup_manager::permissions_ui::PermissionsPanel {
                on_start_download: move |selections| {
                    // Save permissions selections (can be used later)
                    println!("[SETUP] Permissions accepted: {:?}", selections);
                    permissions_accepted.set(true);
                    show_permissions.set(false);
                }
            }
        };
    }

    // Pre-compute data for rendering
    let phase = state.current_phase.clone();
    let has_downloads = !state.downloads.is_empty();
    let has_extractions = !state.extractions.is_empty();
    let has_validations = !state.validations.is_empty();
    let has_error = state.has_error;
    let is_complete = state.is_complete;
    let error_msg = state.error_message.clone();

    // Convert to vectors for iteration
    let downloads: Vec<_> = state.downloads.values().cloned().collect();
    let extractions: Vec<_> = state
        .extractions
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let validations = state.validations.clone();
    let logs = state.logs.clone();

    rsx! {
        style {
            r#"
            /* Hide all scrollbars in setup GUI - Webkit */
            ::-webkit-scrollbar {{
                width: 0px;
                height: 0px;
                display: none;
            }}
            
            ::-webkit-scrollbar-track {{
                display: none;
            }}
            
            ::-webkit-scrollbar-thumb {{
                display: none;
            }}
            
            /* Hide scrollbars - Firefox */
            * {{
                scrollbar-width: none !important;
                scrollbar-color: transparent transparent !important;
            }}
            
            /* Hide scrollbars - IE and Edge */
            * {{
                -ms-overflow-style: none !important;
            }}
            "#
        }
        
        div {
            style: "width: 100%; height: 100%; min-height: 100vh; background: linear-gradient(135deg, #0a0a0a 0%, #1a1a2e 50%, #0a0a0a 100%); color: #fff; font-family: 'Segoe UI', 'Inter', -apple-system, BlinkMacSystemFont, sans-serif; overflow: hidden;",

            // Inner container with padding
            div {
                style: "padding: 40px; max-width: 900px; margin: 0 auto; height: 100%; overflow-y: auto; overflow-x: hidden;",

                // Header
                div {
                    style: "text-align: center; margin-bottom: 40px;",
                    h1 {
                        style: "font-size: 42px; font-weight: 700; letter-spacing: 4px; margin: 0 0 8px 0; background: linear-gradient(90deg, #06b6d4, #3b82f6, #8b5cf6); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;",
                        "IGRIS"
                    }
                    p {
                        style: "font-size: 16px; color: #9ca3af; letter-spacing: 2px; margin: 0;",
                        "Voice Assistant Setup"
                    }
                }

                // Phase indicator
                div {
                    style: "display: flex; justify-content: center; align-items: center; gap: 12px; margin-bottom: 40px; flex-wrap: wrap;",

                    // Phase 1: Initializing
                    {render_phase_step("1", "Init", phase == SetupPhase::Initializing, is_phase_done(&phase, &SetupPhase::Initializing))}

                    {render_phase_connector(is_phase_done(&phase, &SetupPhase::Initializing))}

                    // Phase 2: Downloading
                    {render_phase_step("2", "Download", phase == SetupPhase::Downloading, is_phase_done(&phase, &SetupPhase::Downloading))}

                    {render_phase_connector(is_phase_done(&phase, &SetupPhase::Downloading))}

                    // Phase 3: Extracting
                    {render_phase_step("3", "Extract", phase == SetupPhase::Extracting, is_phase_done(&phase, &SetupPhase::Extracting))}

                    {render_phase_connector(is_phase_done(&phase, &SetupPhase::Extracting))}

                    // Phase 4: Validating
                    {render_phase_step("4", "Validate", phase == SetupPhase::Validating, is_phase_done(&phase, &SetupPhase::Validating))}

                    {render_phase_connector(is_phase_done(&phase, &SetupPhase::Validating))}

                    // Phase 5: Complete
                    {render_phase_step("5", "Done", phase == SetupPhase::Complete, is_complete)}
                }

                // Downloads section
                if has_downloads {
                    div {
                        style: "background: rgba(6, 182, 212, 0.1); border: 1px solid rgba(6, 182, 212, 0.3); border-radius: 12px; padding: 20px; margin-bottom: 20px;",

                        div {
                            style: "display: flex; align-items: center; gap: 10px; margin-bottom: 16px;",
                            span { style: "font-size: 20px;", "📥" }
                            span { style: "font-size: 16px; font-weight: 600; color: #06b6d4;", "Downloading Components" }
                        }

                        div {
                            style: "display: flex; flex-direction: column; gap: 12px;",

                            for download in downloads.iter() {
                                {render_download_item(download)}
                            }
                        }
                    }
                }

                // Extractions section
                if has_extractions {
                    div {
                        style: "background: rgba(139, 92, 246, 0.1); border: 1px solid rgba(139, 92, 246, 0.3); border-radius: 12px; padding: 20px; margin-bottom: 20px;",

                        div {
                            style: "display: flex; align-items: center; gap: 10px; margin-bottom: 16px;",
                            span { style: "font-size: 20px;", "📦" }
                            span { style: "font-size: 16px; font-weight: 600; color: #8b5cf6;", "Extracting Files" }
                        }

                        div {
                            style: "display: flex; flex-direction: column; gap: 8px;",

                            for (name, progress) in extractions.iter() {
                                {render_extraction_item(name, *progress)}
                            }
                        }
                    }
                }

                // Validations section
                if has_validations {
                    div {
                        style: "background: rgba(34, 197, 94, 0.1); border: 1px solid rgba(34, 197, 94, 0.3); border-radius: 12px; padding: 20px; margin-bottom: 20px;",

                        div {
                            style: "display: flex; align-items: center; gap: 10px; margin-bottom: 16px;",
                            span { style: "font-size: 20px;", "✅" }
                            span { style: "font-size: 16px; font-weight: 600; color: #22c55e;", "Validated Components" }
                        }

                        div {
                            style: "display: flex; flex-wrap: wrap; gap: 8px;",

                            for name in validations.iter() {
                                span {
                                    style: "background: rgba(34, 197, 94, 0.2); color: #22c55e; padding: 6px 12px; border-radius: 16px; font-size: 12px; font-weight: 500;",
                                    "✓ {name}"
                                }
                            }
                        }
                    }
                }

                // Error display
                if has_error {
                    div {
                        style: "background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.5); border-radius: 12px; padding: 20px; margin-bottom: 20px;",

                        div {
                            style: "display: flex; align-items: flex-start; gap: 16px;",

                            span { style: "font-size: 28px;", "❌" }

                            div {
                                h3 {
                                    style: "font-size: 16px; font-weight: 600; color: #ef4444; margin: 0 0 8px 0;",
                                    "Setup Error"
                                }
                                p {
                                    style: "font-size: 14px; color: #fca5a5; margin: 0 0 12px 0; line-height: 1.5;",
                                    "{error_msg}"
                                }
                                p {
                                    style: "font-size: 12px; color: #6b7280; margin: 0;",
                                    "Please check your internet connection and try again."
                                }
                            }
                        }
                    }
                }

                // Completion message
                if is_complete {
                    div {
                        style: "background: linear-gradient(135deg, rgba(34, 197, 94, 0.15), rgba(6, 182, 212, 0.15)); border: 1px solid rgba(34, 197, 94, 0.5); border-radius: 12px; padding: 32px; margin-bottom: 20px; text-align: center;",

                        div { style: "font-size: 48px; margin-bottom: 12px;", "🎉" }

                        h2 {
                            style: "font-size: 22px; font-weight: 700; color: #22c55e; margin: 0 0 8px 0;",
                            "Setup Complete!"
                        }

                        p {
                            style: "font-size: 14px; color: #9ca3af; margin: 0 0 16px 0;",
                            "All components have been installed successfully."
                        }

                        p {
                            style: "font-size: 14px; color: #06b6d4; margin: 0;",
                            "Launching IGRIS Voice Assistant..."
                        }
                    }
                }

                // Logs section
                div {
                    style: "background: rgba(0, 0, 0, 0.4); border: 1px solid #374151; border-radius: 12px; padding: 16px; max-height: 200px; overflow-y: auto; overflow-x: hidden;",

                    div {
                        style: "font-size: 11px; font-weight: 600; color: #6b7280; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px;",
                        "Setup Logs"
                    }

                    div {
                        style: "display: flex; flex-direction: column; gap: 4px; font-family: 'Consolas', 'Monaco', monospace; font-size: 12px;",

                        for log in logs.iter().rev().take(15) {
                            {render_log_item(log)}
                        }
                    }
                }
            }

            // CSS animations
            style {
                r#"
                @keyframes spin {{
                    from {{ transform: rotate(0deg); }}
                    to {{ transform: rotate(360deg); }}
                }}

                @keyframes pulse-glow {{
                    0%, 100% {{ box-shadow: 0 0 8px rgba(6, 182, 212, 0.4); }}
                    50% {{ box-shadow: 0 0 16px rgba(6, 182, 212, 0.8); }}
                }}
                "#
            }
        }
    }
}

/// Render a phase step indicator
fn render_phase_step(num: &str, label: &str, is_current: bool, is_done: bool) -> Element {
    let bg_style = if is_done {
        "background: #22c55e;"
    } else if is_current {
        "background: linear-gradient(135deg, #06b6d4, #3b82f6); animation: pulse-glow 2s ease-in-out infinite;"
    } else {
        "background: #374151;"
    };

    let text_color = if is_current {
        "color: #06b6d4;"
    } else if is_done {
        "color: #22c55e;"
    } else {
        "color: #6b7280;"
    };

    let display_text = if is_done { "✓" } else { num };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; gap: 6px;",

            div {
                style: format!("width: 32px; height: 32px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 600; color: white; {}", bg_style),
                "{display_text}"
            }

            span {
                style: format!("font-size: 11px; font-weight: 500; {}", text_color),
                "{label}"
            }
        }
    }
}

/// Render connector line between phases
fn render_phase_connector(is_done: bool) -> Element {
    let bg = if is_done {
        "background: #22c55e;"
    } else {
        "background: #374151;"
    };

    rsx! {
        div {
            style: format!("width: 30px; height: 2px; margin-bottom: 20px; {}", bg),
        }
    }
}

/// Check if a phase is done based on current phase
fn is_phase_done(current: &SetupPhase, check: &SetupPhase) -> bool {
    let order = |p: &SetupPhase| match p {
        SetupPhase::Initializing => 0,
        SetupPhase::Downloading => 1,
        SetupPhase::Extracting => 2,
        SetupPhase::Validating => 3,
        SetupPhase::Complete => 4,
        SetupPhase::Error => 5,
    };
    order(current) > order(check)
}

/// Render a download item with progress bar
fn render_download_item(download: &DownloadProgress) -> Element {
    let name = download.name.clone();
    let progress = download.progress;
    let is_complete = download.is_complete;

    let progress_width = format!("{}%", progress);
    let status_text = if is_complete {
        "Complete".to_string()
    } else {
        format!("{}%", progress)
    };

    let status_color = if is_complete {
        "color: #22c55e;"
    } else {
        "color: #06b6d4;"
    };
    let bar_bg = if is_complete {
        "background: linear-gradient(90deg, #22c55e, #16a34a);"
    } else {
        "background: linear-gradient(90deg, #06b6d4, #3b82f6);"
    };

    rsx! {
        div {
            style: "background: rgba(0, 0, 0, 0.3); border-radius: 8px; padding: 12px;",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",

                span {
                    style: "font-size: 13px; font-weight: 500; color: #e5e7eb;",
                    "{name}"
                }

                span {
                    style: format!("font-size: 12px; font-weight: 600; {}", status_color),
                    if is_complete { "✓ " } else { "" }
                    "{status_text}"
                }
            }

            div {
                style: "width: 100%; height: 6px; background: #1f2937; border-radius: 3px; overflow: hidden;",

                div {
                    style: format!("width: {}; height: 100%; border-radius: 3px; transition: width 0.3s ease; {}", progress_width, bar_bg),
                }
            }
        }
    }
}

/// Render an extraction item
fn render_extraction_item(name: &str, progress: u32) -> Element {
    let is_done = progress >= 100;
    let status = if is_done { "Done" } else { "Extracting..." };

    let icon_style = if is_done {
        "background: #22c55e; color: white;"
    } else {
        "border: 2px solid #8b5cf6; border-top-color: transparent; animation: spin 1s linear infinite;"
    };

    let status_color = if is_done {
        "color: #22c55e;"
    } else {
        "color: #8b5cf6;"
    };

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 12px; padding: 10px; background: rgba(0, 0, 0, 0.3); border-radius: 8px;",

            div {
                style: format!("width: 20px; height: 20px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 10px; {}", icon_style),
                if is_done { "✓" } else { "" }
            }

            span {
                style: "font-size: 13px; color: #e5e7eb; flex: 1;",
                "{name}"
            }

            span {
                style: format!("font-size: 11px; font-weight: 500; {}", status_color),
                "{status}"
            }
        }
    }
}

/// Render a log item
fn render_log_item(log: &SetupLog) -> Element {
    let log_style = match log.log_type {
        LogType::Success => "color: #22c55e; background: rgba(34, 197, 94, 0.1);",
        LogType::Error => "color: #ef4444; background: rgba(239, 68, 68, 0.1);",
        LogType::Warning => "color: #f59e0b; background: rgba(245, 158, 11, 0.1);",
        LogType::Info => "color: #9ca3af;",
    };

    let msg = log.message.clone();

    rsx! {
        div {
            style: format!("padding: 4px 8px; border-radius: 4px; {}", log_style),
            span { style: "color: #4b5563; margin-right: 6px;", "›" }
            "{msg}"
        }
    }
}

/// Standalone setup window component
#[component]
pub fn SetupWindow() -> Element {
    rsx! {
        div {
            style: "width: 100vw; height: 100vh; overflow: hidden;",
            SetupGui {}
        }
    }
}
