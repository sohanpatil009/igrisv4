// src/ui/search_results.rs - Search Results UI Component
use dioxus::prelude::*;
use crate::SEARCH_STATE;

/// Search result item for UI display
#[derive(Clone, PartialEq)]
pub struct SearchResultItem {
    pub path: String,
    pub name: String,
    pub drive: String,
    pub score: u32,
    pub is_folder: bool,
}

/// Helper to close the search panel (updates global state)
fn close_search_panel() {
    if let Ok(mut state) = SEARCH_STATE.lock() {
        state.is_open = false;
    }
}

/// Search Results Panel Component with progress tracking and cancel functionality
#[component]
pub fn SearchResultsPanel(
    is_open: Signal<bool>,
    results: Signal<Vec<SearchResultItem>>,
    search_query: Signal<String>,
    is_searching: Signal<bool>,
) -> Element {
    let open = is_open();
    let items = results();
    let query = search_query();
    let searching = is_searching();

    if !open {
        return rsx! {};
    }

    // Group results by drive
    let mut grouped: std::collections::BTreeMap<String, Vec<SearchResultItem>> = std::collections::BTreeMap::new();
    for item in items.iter() {
        grouped.entry(item.drive.clone()).or_default().push(item.clone());
    }

    rsx! {
        // Backdrop
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0, 0, 0, 0.7); z-index: 90; backdrop-filter: blur(4px);",
            onclick: move |_| close_search_panel(),
        }

        // Search Results Container
        div {
            style: "position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); width: clamp(400px, 60vw, 800px); max-height: 70vh; background: linear-gradient(135deg, #1a1a2e, #16213e); border: 1px solid rgba(6, 182, 212, 0.3); border-radius: 16px; z-index: 100; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);",

            // Header
            div {
                style: "padding: 20px 24px; border-bottom: 1px solid rgba(6, 182, 212, 0.2); display: flex; justify-content: space-between; align-items: center;",
                
                div {
                    style: "display: flex; flex-direction: column; gap: 4px;",
                    h2 {
                        style: "font-size: 18px; font-weight: 600; color: #06b6d4; margin: 0;",
                        "🔍 Search Results"
                    }
                    if !query.is_empty() {
                        span {
                            style: "font-size: 12px; color: #9ca3af;",
                            "Searching for: \"{query}\""
                        }
                    }
                }

                div {
                    style: "display: flex; gap: 8px; align-items: center;",
                    
                    // Close button
                    button {
                        style: "background: rgba(239, 68, 68, 0.2); border: 1px solid rgba(239, 68, 68, 0.3); color: #ef4444; width: 32px; height: 32px; border-radius: 8px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 16px; transition: all 0.2s;",
                        onclick: move |_| close_search_panel(),
                        "✕"
                    }
                }
            }

            // Progress indicator (when searching)
            if searching {
                div {
                    style: "padding: 20px 24px; border-bottom: 1px solid rgba(6, 182, 212, 0.1);",
                    
                    div {
                        style: "display: flex; align-items: center; gap: 12px;",
                        div {
                            style: "width: 20px; height: 20px; border: 2px solid rgba(6, 182, 212, 0.3); border-top-color: #06b6d4; border-radius: 50%; animation: spin 1s linear infinite;",
                        }
                        div {
                            style: "display: flex; flex-direction: column; gap: 2px;",
                            span {
                                style: "font-size: 13px; color: #e5e7eb;",
                                "Searching..."
                            }
                        }
                    }
                }
            }

            // Results list with invisible scrollbar
            if !searching {
                div {
                    style: "flex: 1; overflow-y: auto; overflow-x: hidden; padding: 16px 24px; scrollbar-width: none; -ms-overflow-style: none;",
                    
                    if items.is_empty() {
                        div {
                            style: "text-align: center; padding: 40px 20px;",
                            div {
                                style: "font-size: 48px; margin-bottom: 16px;",
                                "📭"
                            }
                            p {
                                style: "color: #9ca3af; font-size: 14px;",
                                "No results found"
                            }
                        }
                    } else {
                        // Results count
                        div {
                            style: "margin-bottom: 16px; padding: 8px 12px; background: rgba(6, 182, 212, 0.1); border-radius: 8px; display: inline-block;",
                            span {
                                style: "color: #06b6d4; font-size: 13px; font-weight: 500;",
                                "Found {items.len()} results"
                            }
                        }

                        // Grouped by drive
                        for (drive, drive_items) in grouped.iter() {
                            div {
                                style: "margin-bottom: 20px;",
                                
                                // Drive header
                                div {
                                    style: "display: flex; align-items: center; gap: 8px; margin-bottom: 12px; padding-bottom: 8px; border-bottom: 1px solid rgba(6, 182, 212, 0.15);",
                                    span {
                                        style: "font-size: 14px; color: #3b82f6; font-weight: 600;",
                                        "💾 {drive}"
                                    }
                                    span {
                                        style: "font-size: 11px; color: #6b7280; background: rgba(59, 130, 246, 0.1); padding: 2px 8px; border-radius: 10px;",
                                        "{drive_items.len()} items"
                                    }
                                }

                                // Items in this drive
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px;",
                                    for item in drive_items.iter() {
                                        SearchResultRow { item: item.clone() }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Footer
            div {
                style: "padding: 12px 24px; border-top: 1px solid rgba(6, 182, 212, 0.2); display: flex; justify-content: space-between; align-items: center;",
                
                span {
                    style: "font-size: 11px; color: #6b7280;",
                    if searching {
                        "Search in progress... Click Cancel to stop"
                    } else {
                        "Click filename to open • Click path to open folder"
                    }
                }

                if !items.is_empty() && !searching {
                    button {
                        style: "background: rgba(6, 182, 212, 0.2); border: 1px solid rgba(6, 182, 212, 0.3); color: #06b6d4; padding: 6px 12px; border-radius: 6px; cursor: pointer; font-size: 12px; transition: all 0.2s;",
                        onclick: move |_| {
                            // Open first result's folder
                            if let Some(first) = items.first() {
                                let path = std::path::Path::new(&first.path);
                                if let Some(parent) = path.parent() {
                                    let _ = crate::platform_utils::get_file_system()
                                        .open_folder(&parent.to_string_lossy());
                                }
                            }
                        },
                        "📂 Open Location"
                    }
                }
            }
        }

        // Keyframes for spinner
        style {
            r#"
            @keyframes spin {{
                from {{ transform: rotate(0deg); }}
                to {{ transform: rotate(360deg); }}
            }}
            "#
        }
    }
}

/// Individual search result row
#[component]
fn SearchResultRow(item: SearchResultItem) -> Element {
    let icon = if item.is_folder { "📁" } else { get_file_icon(&item.name) };
    let path_for_file = item.path.clone();
    let path_for_folder = item.path.clone();
    let is_folder = item.is_folder;

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 12px; padding: 10px 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.05); border-radius: 8px; transition: all 0.2s;",

            // Icon + Filename (clickable - opens file/folder)
            div {
                style: "display: flex; align-items: center; gap: 12px; cursor: pointer; flex-shrink: 0;",
                onclick: move |e| {
                    e.stop_propagation();
                    if is_folder {
                        let _ = crate::platform_utils::get_file_system().open_folder(&path_for_file);
                    } else {
                        let _ = crate::platform_utils::get_file_system().open_file(&path_for_file);
                    }
                },
                
                // Icon
                span {
                    style: "font-size: 20px; flex-shrink: 0;",
                    "{icon}"
                }
                
                // Filename
                span {
                    style: "font-size: 13px; color: #e5e7eb; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 200px;",
                    title: "{item.name}",
                    "{item.name}"
                }
            }

            // Path (clickable - opens containing folder)
            div {
                style: "flex: 1; min-width: 0; cursor: pointer;",
                onclick: move |e| {
                    e.stop_propagation();
                    // Open the containing folder
                    let path = std::path::Path::new(&path_for_folder);
                    if let Some(parent) = path.parent() {
                        let _ = crate::platform_utils::get_file_system()
                            .open_folder(&parent.to_string_lossy());
                    }
                },
                
                span {
                    style: "font-size: 11px; color: #60a5fa; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; display: block; transition: color 0.2s;",
                    title: "Click to open folder: {item.path}",
                    "📂 {item.path}"
                }
            }

            // Score badge
            div {
                style: format!(
                    "padding: 2px 8px; border-radius: 10px; font-size: 10px; font-weight: 600; flex-shrink: 0; {}",
                    if item.score >= 80 {
                        "background: rgba(34, 197, 94, 0.2); color: #22c55e;"
                    } else if item.score >= 60 {
                        "background: rgba(234, 179, 8, 0.2); color: #eab308;"
                    } else {
                        "background: rgba(107, 114, 128, 0.2); color: #9ca3af;"
                    }
                ),
                "{item.score}%"
            }
        }
    }
}

/// Get file icon based on extension
fn get_file_icon(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    
    match ext.as_str() {
        "pdf" => "📕",
        "doc" | "docx" => "📘",
        "xls" | "xlsx" => "📗",
        "ppt" | "pptx" => "📙",
        "txt" | "md" => "📄",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => "🖼️",
        "mp3" | "wav" | "flac" | "aac" | "ogg" => "🎵",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" => "🎬",
        "zip" | "rar" | "7z" | "tar" | "gz" => "📦",
        "exe" | "msi" => "⚙️",
        "py" => "🐍",
        "rs" => "🦀",
        "js" | "ts" => "📜",
        "html" | "htm" => "🌐",
        "css" => "🎨",
        "json" => "📋",
        "xml" => "📰",
        "sql" => "🗃️",
        "sh" | "bat" | "ps1" => "⌨️",
        _ => "📄",
    }
}
