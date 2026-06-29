// src/setup_manager/permissions_ui.rs - Clean permissions UI
use dioxus::prelude::*;

/// Permission selections returned to parent
#[derive(Clone, Debug, PartialEq)]
pub struct PermissionSelections {
    pub camera: bool,
    pub files: bool,
    pub apps: bool,
}

/// Main Permissions Panel - Clean login style
#[component]
pub fn PermissionsPanel(
    on_start_download: EventHandler<PermissionSelections>,
) -> Element {
    let mut camera = use_signal(|| true);
    let mut files = use_signal(|| true);
    let mut apps = use_signal(|| true);
    let mut manual_mode = use_signal(|| false);
    
    let handle_accept_all = move |_| {
        on_start_download.call(PermissionSelections {
            camera: true,
            files: true,
            apps: true,
        });
    };
    
    let handle_done = move |_| {
        on_start_download.call(PermissionSelections {
            camera: camera(),
            files: files(),
            apps: apps(),
        });
    };

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: linear-gradient(135deg, #0f0f1a 0%, #1a1a2e 100%); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            
            div {
                style: "background: rgba(26, 26, 46, 0.95); border: 2px solid #06b6d4; border-radius: 16px; padding: 40px; width: 380px; box-shadow: 0 0 60px rgba(6, 182, 212, 0.2);",
                
                // Icon
                div {
                    style: "width: 56px; height: 56px; margin: 0 auto 20px; background: linear-gradient(135deg, #06b6d4, #0891b2); border-radius: 14px; display: flex; align-items: center; justify-content: center; font-size: 28px;",
                    "🛡️"
                }
                
                // Title
                h1 {
                    style: "color: #cffafe; font-size: 24px; font-weight: 600; text-align: center; margin: 0 0 6px 0;",
                    "Permissions"
                }
                
                p {
                    style: "color: #6b7280; font-size: 13px; text-align: center; margin: 0 0 28px 0;",
                    "Enable features for IGRIS"
                }
                
                // Checkboxes
                div {
                    style: "display: flex; flex-direction: column; gap: 10px; margin-bottom: 28px;",
                    
                    PermissionCheckbox {
                        icon: "📷",
                        label: "Camera",
                        checked: camera(),
                        on_change: move |val| camera.set(val),
                    }
                    
                    PermissionCheckbox {
                        icon: "📁",
                        label: "Files",
                        checked: files(),
                        on_change: move |val| files.set(val),
                    }
                    
                    PermissionCheckbox {
                        icon: "🚀",
                        label: "Apps",
                        checked: apps(),
                        on_change: move |val| apps.set(val),
                    }
                }
                
                // Buttons
                div {
                    style: "display: flex; flex-direction: column; gap: 10px;",
                    
                    if !manual_mode() {
                        button {
                            onclick: handle_accept_all,
                            style: "width: 100%; padding: 12px; background: linear-gradient(135deg, #06b6d4, #0891b2); color: white; border: none; border-radius: 10px; font-size: 15px; font-weight: 600; cursor: pointer;",
                            "Accept All"
                        }
                        
                        button {
                            onclick: move |_| manual_mode.set(true),
                            style: "width: 100%; padding: 12px; background: transparent; color: #6b7280; border: 1px solid #374151; border-radius: 10px; font-size: 15px; cursor: pointer;",
                            "Customize"
                        }
                    }
                    
                    if manual_mode() {
                        button {
                            onclick: handle_done,
                            style: "width: 100%; padding: 12px; background: linear-gradient(135deg, #22c55e, #16a34a); color: white; border: none; border-radius: 10px; font-size: 15px; font-weight: 600; cursor: pointer;",
                            "Done"
                        }
                    }
                }
            }
        }
    }
}

/// Clean checkbox component
#[component]
fn PermissionCheckbox(
    icon: &'static str,
    label: &'static str,
    checked: bool,
    on_change: EventHandler<bool>,
) -> Element {
    let border_color = if checked { "#06b6d4" } else { "#374151" };
    let bg_color = if checked { "rgba(6, 182, 212, 0.08)" } else { "transparent" };
    
    rsx! {
        div {
            onclick: move |_| on_change.call(!checked),
            style: "display: flex; align-items: center; gap: 14px; padding: 12px 14px; background: {bg_color}; border: 1px solid {border_color}; border-radius: 10px; cursor: pointer;",
            
            span { style: "font-size: 20px;", "{icon}" }
            
            span {
                style: "flex: 1; color: #cffafe; font-size: 15px; font-weight: 500;",
                "{label}"
            }
            
            div {
                style: "width: 20px; height: 20px; border: 2px solid {border_color}; border-radius: 5px; display: flex; align-items: center; justify-content: center;",
                if checked {
                    div {
                        style: "width: 10px; height: 10px; background: #06b6d4; border-radius: 2px;",
                    }
                }
            }
        }
    }
}

/// Download Progress Panel
#[component]
pub fn DownloadProgressPanel(
    current_item: String,
    progress: u32,
    total_items: usize,
    completed_items: usize,
) -> Element {
    let progress_percent = if total_items > 0 {
        (completed_items as f32 / total_items as f32 * 100.0) as u32
    } else {
        progress
    };

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: linear-gradient(135deg, #0f0f1a 0%, #1a1a2e 100%); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            
            div {
                style: "background: rgba(26, 26, 46, 0.95); border: 2px solid #06b6d4; border-radius: 16px; padding: 40px; width: 360px; text-align: center;",
                
                div {
                    style: "width: 64px; height: 64px; margin: 0 auto 20px; background: linear-gradient(135deg, #06b6d4, #0891b2); border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 28px;",
                    "⬇️"
                }
                
                h2 {
                    style: "color: #cffafe; font-size: 20px; margin: 0 0 6px 0;",
                    "Downloading"
                }
                
                p {
                    style: "color: #6b7280; font-size: 13px; margin: 0 0 24px 0;",
                    "{current_item}"
                }
                
                div {
                    style: "width: 100%; height: 6px; background: #1f2937; border-radius: 3px; overflow: hidden; margin-bottom: 12px;",
                    div {
                        style: "width: {progress_percent}%; height: 100%; background: linear-gradient(90deg, #06b6d4, #22c55e); border-radius: 3px;",
                    }
                }
                
                p {
                    style: "color: #4b5563; font-size: 12px; margin: 0;",
                    "{completed_items} / {total_items}"
                }
            }
        }
    }
}

/// Settings button
#[component]
pub fn SettingsButton(
    on_click: EventHandler<()>,
    pending_count: usize,
) -> Element {
    rsx! {
        button {
            onclick: move |_| on_click.call(()),
            style: "position: fixed; top: 20px; right: 20px; z-index: 50; background: rgba(6, 182, 212, 0.15); border: 1px solid #06b6d4; border-radius: 50%; width: 44px; height: 44px; display: flex; align-items: center; justify-content: center; cursor: pointer; color: #06b6d4; font-size: 20px;",
            "⚙"
            
            if pending_count > 0 {
                div {
                    style: "position: absolute; top: -6px; right: -6px; background: #ef4444; color: white; border-radius: 50%; width: 20px; height: 20px; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: bold;",
                    "{pending_count}"
                }
            }
        }
    }
}
