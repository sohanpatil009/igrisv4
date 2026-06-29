// File Picker UI Component - Dioxus 0.7
// LocalSend-style file selection interface

use dioxus::prelude::*;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub struct SelectedFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
}

#[component]
pub fn FilePicker(
    on_files_selected: EventHandler<Vec<String>>,
    on_close: EventHandler<()>,
) -> Element {
    let mut selected_files = use_signal(|| Vec::<SelectedFile>::new());
    let mut is_dragging = use_signal(|| false);
    let mut show_error = use_signal(|| None::<String>);

    // Handle file selection via native dialog
    let handle_file_select = move |_| {
        spawn(async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use rfd::AsyncFileDialog;
                
                let files = AsyncFileDialog::new()
                    .set_title("Select Files to Share")
                    .pick_files()
                    .await;

                if let Some(files) = files {
                    let mut selected = Vec::new();
                    
                    for file in files {
                        let path = file.path().to_string_lossy().to_string();
                        let name = file.file_name();
                        
                        // Get file size
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            let size = metadata.len();
                            let file_type = mime_guess::from_path(&path)
                                .first_or_octet_stream()
                                .to_string();
                            
                            selected.push(SelectedFile {
                                path: path.clone(),
                                name,
                                size,
                                file_type,
                            });
                        }
                    }
                    
                    *selected_files.write() = selected;
                }
            }
        });
    };

    // Handle folder selection
    let handle_folder_select = move |_| {
        spawn(async move {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use rfd::AsyncFileDialog;
                
                let folder = AsyncFileDialog::new()
                    .set_title("Select Folder to Share")
                    .pick_folder()
                    .await;

                if let Some(folder) = folder {
                    let path = folder.path();
                    
                    // Recursively get all files in folder
                    if let Ok(files) = get_files_in_folder(path) {
                        *selected_files.write() = files;
                    } else {
                        *show_error.write() = Some("Failed to read folder".to_string());
                    }
                }
            }
        });
    };

    // Remove file from selection
    let mut remove_file = move |index: usize| {
        selected_files.with_mut(|files| {
            files.remove(index);
        });
    };

    // Send selected files
    let send_files = move |_| {
        let paths: Vec<String> = selected_files.read().iter().map(|f| f.path.clone()).collect();
        if !paths.is_empty() {
            on_files_selected.call(paths);
        }
    };

    // Calculate total size
    let total_size = use_memo(move || {
        selected_files.read().iter().map(|f| f.size).sum::<u64>()
    });

    rsx! {
        div {
            class: "file-picker-overlay",
            style: "
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0,0,0,0.85);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 2000;
                backdrop-filter: blur(8px);
            ",
            onclick: move |_| on_close.call(()),
            
            div {
                class: "file-picker-dialog",
                style: "
                    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
                    border: 2px solid #a855f7;
                    border-radius: 20px;
                    padding: 30px;
                    max-width: 700px;
                    width: 90%;
                    max-height: 80vh;
                    display: flex;
                    flex-direction: column;
                    box-shadow: 0 25px 80px rgba(168,85,247,0.5);
                ",
                onclick: move |e| e.stop_propagation(),
                
                // Header
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 25px;",
                    
                    div {
                        h2 {
                            style: "color: #e2e8f0; margin: 0 0 5px 0; font-size: 26px; font-weight: 700;",
                            "📁 Select Files"
                        }
                        p {
                            style: "color: #94a3b8; margin: 0; font-size: 14px;",
                            "Choose files or folders to share"
                        }
                    }
                    
                    button {
                        style: "
                            background: transparent;
                            border: none;
                            color: #94a3b8;
                            font-size: 28px;
                            cursor: pointer;
                            padding: 5px;
                            line-height: 1;
                            transition: color 0.2s;
                        ",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_close.call(());
                        },
                        "×"
                    }
                }
                
                // Error message
                if let Some(error) = show_error() {
                    div {
                        style: "
                            padding: 12px;
                            background: rgba(239, 68, 68, 0.1);
                            border: 1px solid #ef4444;
                            border-radius: 8px;
                            margin-bottom: 20px;
                        ",
                        p {
                            style: "color: #ef4444; margin: 0; font-size: 13px;",
                            "⚠️ {error}"
                        }
                    }
                }
                
                // Selection buttons
                div {
                    style: "display: flex; gap: 12px; margin-bottom: 25px;",
                    
                    button {
                        style: "
                            flex: 1;
                            padding: 16px 24px;
                            background: rgba(168,85,247,0.1);
                            border: 2px dashed #a855f7;
                            color: #a855f7;
                            border-radius: 12px;
                            cursor: pointer;
                            font-size: 15px;
                            font-weight: 600;
                            transition: all 0.2s;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            gap: 10px;
                        ",
                        onclick: handle_file_select,
                        span { style: "font-size: 24px;", "📄" }
                        span { "Select Files" }
                    }
                    
                    button {
                        style: "
                            flex: 1;
                            padding: 16px 24px;
                            background: rgba(168,85,247,0.1);
                            border: 2px dashed #a855f7;
                            color: #a855f7;
                            border-radius: 12px;
                            cursor: pointer;
                            font-size: 15px;
                            font-weight: 600;
                            transition: all 0.2s;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            gap: 10px;
                        ",
                        onclick: handle_folder_select,
                        span { style: "font-size: 24px;", "📁" }
                        span { "Select Folder" }
                    }
                }
                
                // Drag & Drop area
                div {
                    style: format!("
                        padding: 40px;
                        background: {};
                        border: 2px dashed {};
                        border-radius: 12px;
                        text-align: center;
                        margin-bottom: 20px;
                        transition: all 0.3s;
                    ",
                        if is_dragging() { "rgba(168,85,247,0.2)" } else { "rgba(255,255,255,0.03)" },
                        if is_dragging() { "#a855f7" } else { "rgba(168,85,247,0.3)" }
                    ),
                    ondragover: move |e| {
                        e.prevent_default();
                        *is_dragging.write() = true;
                    },
                    ondragleave: move |_| {
                        *is_dragging.write() = false;
                    },
                    ondrop: move |e| {
                        e.prevent_default();
                        *is_dragging.write() = false;
                        // Handle dropped files
                        // Note: File drop handling requires desktop-specific implementation
                    },
                    
                    div {
                        style: "font-size: 48px; margin-bottom: 15px;",
                        if is_dragging() { "📥" } else { "🎯" }
                    }
                    p {
                        style: "color: #94a3b8; margin: 0; font-size: 15px;",
                        if is_dragging() {
                            "Drop files here"
                        } else {
                            "Or drag and drop files here"
                        }
                    }
                }
                
                // Selected files list
                if !selected_files.read().is_empty() {
                    div {
                        style: "
                            flex: 1;
                            overflow-y: auto;
                            margin-bottom: 20px;
                            max-height: 300px;
                        ",
                        
                        div {
                            style: "margin-bottom: 12px; display: flex; justify-content: space-between; align-items: center;",
                            h3 {
                                style: "color: #e2e8f0; margin: 0; font-size: 16px;",
                                "Selected Files ({selected_files.read().len()})"
                            }
                            p {
                                style: "color: #a855f7; margin: 0; font-size: 14px; font-weight: 600;",
                                "{format_bytes(total_size())}"
                            }
                        }
                        
                        div {
                            style: "display: flex; flex-direction: column; gap: 8px;",
                            
                            for (index , file) in selected_files.read().iter().enumerate() {
                                FileItem {
                                    file: file.clone(),
                                    index: index,
                                    on_remove: move |idx: usize| remove_file(idx),
                                }
                            }
                        }
                    }
                }
                
                // Footer buttons
                div {
                    style: "display: flex; gap: 12px; margin-top: auto;",
                    
                    button {
                        style: "
                            flex: 1;
                            padding: 14px 24px;
                            background: rgba(255,255,255,0.1);
                            color: #e2e8f0;
                            border: 1px solid rgba(255,255,255,0.2);
                            border-radius: 10px;
                            cursor: pointer;
                            font-size: 15px;
                            font-weight: 600;
                            transition: all 0.2s;
                        ",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_close.call(());
                        },
                        "Cancel"
                    }
                    
                    button {
                        style: format!("
                            flex: 1;
                            padding: 14px 24px;
                            background: {};
                            color: white;
                            border: none;
                            border-radius: 10px;
                            cursor: {};
                            font-size: 15px;
                            font-weight: 600;
                            transition: all 0.2s;
                            box-shadow: 0 4px 12px rgba(168,85,247,0.4);
                            opacity: {};
                        ",
                            if selected_files.read().is_empty() { "rgba(168,85,247,0.3)" } else { "linear-gradient(135deg, #a855f7, #7c3aed)" },
                            if selected_files.read().is_empty() { "not-allowed" } else { "pointer" },
                            if selected_files.read().is_empty() { "0.5" } else { "1" }
                        ),
                        disabled: selected_files.read().is_empty(),
                        onclick: send_files,
                        "✓ Send {selected_files.read().len()} File(s)"
                    }
                }
            }
        }
    }
}

#[component]
fn FileItem(
    file: SelectedFile,
    index: usize,
    on_remove: EventHandler<usize>,
) -> Element {
    let file_icon = get_file_icon(&file.file_type);
    
    rsx! {
        div {
            style: "
                padding: 12px 16px;
                background: rgba(255,255,255,0.05);
                border: 1px solid rgba(168,85,247,0.2);
                border-radius: 10px;
                display: flex;
                align-items: center;
                gap: 12px;
                transition: all 0.2s;
            ",
            
            span {
                style: "font-size: 28px;",
                "{file_icon}"
            }
            
            div {
                style: "flex: 1; min-width: 0;",
                p {
                    style: "
                        color: #e2e8f0;
                        margin: 0 0 4px 0;
                        font-size: 14px;
                        font-weight: 500;
                        white-space: nowrap;
                        overflow: hidden;
                        text-overflow: ellipsis;
                    ",
                    "{file.name}"
                }
                p {
                    style: "color: #64748b; margin: 0; font-size: 12px;",
                    "{format_bytes(file.size)} • {get_file_type_label(&file.file_type)}"
                }
            }
            
            button {
                style: "
                    background: rgba(239, 68, 68, 0.1);
                    border: 1px solid #ef4444;
                    color: #ef4444;
                    padding: 6px 12px;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 12px;
                    font-weight: 600;
                    transition: all 0.2s;
                ",
                onclick: move |e| {
                    e.stop_propagation();
                    on_remove.call(index);
                },
                "Remove"
            }
        }
    }
}

// Helper functions

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn get_file_icon(mime_type: &str) -> &'static str {
    if mime_type.starts_with("image/") {
        "🖼️"
    } else if mime_type.starts_with("video/") {
        "🎥"
    } else if mime_type.starts_with("audio/") {
        "🎵"
    } else if mime_type.starts_with("text/") {
        "📝"
    } else if mime_type.contains("pdf") {
        "📕"
    } else if mime_type.contains("zip") || mime_type.contains("archive") {
        "📦"
    } else if mime_type.contains("word") || mime_type.contains("document") {
        "📄"
    } else if mime_type.contains("sheet") || mime_type.contains("excel") {
        "📊"
    } else if mime_type.contains("presentation") || mime_type.contains("powerpoint") {
        "📽️"
    } else {
        "📄"
    }
}

fn get_file_type_label(mime_type: &str) -> String {
    if mime_type.starts_with("image/") {
        "Image".to_string()
    } else if mime_type.starts_with("video/") {
        "Video".to_string()
    } else if mime_type.starts_with("audio/") {
        "Audio".to_string()
    } else if mime_type.contains("pdf") {
        "PDF".to_string()
    } else if mime_type.contains("zip") {
        "Archive".to_string()
    } else if mime_type.contains("word") {
        "Document".to_string()
    } else if mime_type.contains("sheet") {
        "Spreadsheet".to_string()
    } else {
        "File".to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_files_in_folder(path: &std::path::Path) -> Result<Vec<SelectedFile>, std::io::Error> {
    use walkdir::WalkDir;
    
    let mut files = Vec::new();
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path_str = entry.path().to_string_lossy().to_string();
            let name = entry.file_name().to_string_lossy().to_string();
            
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.len();
                let file_type = mime_guess::from_path(entry.path())
                    .first_or_octet_stream()
                    .to_string();
                
                files.push(SelectedFile {
                    path: path_str,
                    name,
                    size,
                    file_type,
                });
            }
        }
    }
    
    Ok(files)
}
