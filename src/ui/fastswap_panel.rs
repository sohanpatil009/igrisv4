use dioxus::prelude::*;
use rfd::FileDialog;
use crate::fastswap::{Device, FileProgress, ProgressStatus, TransferProgress, PendingTransfer};
use std::path::PathBuf;

#[component]
pub fn FastSwapPanel() -> Element {
    let devices = use_signal(|| Vec::<Device>::new());
    let is_scanning = use_signal(|| false);
    let mut selected_files = use_signal(|| Vec::<PathBuf>::new());
    let mut selected_device = use_signal(|| None::<Device>);
    let mut active_transfers = use_signal(|| Vec::<TransferProgress>::new());
    let mut pending_transfers = use_signal(|| Vec::<PendingTransfer>::new());
    let mut status_message = use_signal(|| String::from("FastSwap Ready"));
    let mut current_session = use_signal(|| None::<String>);

    // Auto-scan on mount (also ensures server starts on demand)
    use_effect(move || {
        spawn(async move {
            crate::fastswap::start_on_demand().await;
            scan_for_devices(devices, is_scanning, status_message).await;
        });
    });

    // Periodic device refresh (every 5 seconds)
    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
                if !is_scanning() {
                    scan_for_devices(devices, is_scanning, status_message).await;
                }
            }
        });
    });

    // Progress update loop (every 200ms)
    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(200)).await;
                
                // Get all active transfers from global progress tracker
                let tracker = crate::fastswap::get_progress_tracker();
                let guard = tracker.read().await;
                let transfers: Vec<TransferProgress> = guard.values().cloned().collect();
                drop(guard);
                
                // Update UI
                active_transfers.set(transfers.clone());
                
                // Get pending transfers (incoming)
                let pending = crate::fastswap::get_pending_transfers().await;
                pending_transfers.set(pending);
                
                // Check if current session is complete
                if let Some(session_id) = current_session() {
                    if let Some(progress) = transfers.iter().find(|t| t.session_id == session_id) {
                        if progress.is_complete() {
                            if progress.is_cancelled {
                                status_message.set("❌ Transfer cancelled".to_string());
                            } else {
                                status_message.set("✅ Transfer complete!".to_string());
                            }
                            current_session.set(None);
                        }
                    }
                }
            }
        });
    });

    rsx! {
        div {
            class: "fastswap-panel",
            style: "padding: 24px; background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); border-radius: 16px; color: white; max-height: 80vh; overflow-y: auto;",

            // Header
            div {
                style: "margin-bottom: 24px; border-bottom: 2px solid rgba(168, 85, 247, 0.3); padding-bottom: 16px;",
                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    h2 {
                        style: "margin: 0; color: #a855f7; font-size: 28px; font-weight: bold;",
                        "⚡ FastSwap"
                    }
                    button {
                        style: "padding: 8px 16px; background: rgba(168, 85, 247, 0.2); border: 1px solid #a855f7; border-radius: 8px; color: #a855f7; cursor: pointer; font-size: 14px; transition: all 0.3s;",
                        onclick: move |_| {
                            spawn(async move {
                                scan_for_devices(devices, is_scanning, status_message).await;
                            });
                        },
                        disabled: is_scanning(),
                        if is_scanning() {
                            "🔄 Scanning..."
                        } else {
                            "🔍 Scan Network"
                        }
                    }
                }
                p {
                    style: "margin: 8px 0 0 0; color: #888; font-size: 14px;",
                    "{status_message}"
                }
            }

            // File Selection Section
            div {
                style: "margin-bottom: 24px; padding: 16px; background: rgba(0, 0, 0, 0.3); border-radius: 12px;",
                h3 {
                    style: "margin: 0 0 12px 0; color: #e9d5ff; font-size: 18px;",
                    "📁 Select Files to Send"
                }
                
                div {
                    style: "display: flex; gap: 12px; margin-bottom: 12px;",
                    button {
                        style: "flex: 1; padding: 12px; background: rgba(168, 85, 247, 0.2); border: 1px solid #a855f7; border-radius: 8px; color: #e9d5ff; cursor: pointer; font-size: 14px; transition: all 0.3s;",
                        onclick: move |_| {
                            spawn(async move {
                                select_files(selected_files, status_message).await;
                            });
                        },
                        "📄 Select Files"
                    }
                    button {
                        style: "flex: 1; padding: 12px; background: rgba(168, 85, 247, 0.2); border: 1px solid #a855f7; border-radius: 8px; color: #e9d5ff; cursor: pointer; font-size: 14px; transition: all 0.3s;",
                        onclick: move |_| {
                            spawn(async move {
                                select_folder(selected_files, status_message).await;
                            });
                        },
                        "📁 Select Folder"
                    }
                }
                
                // Selected files list
                if !selected_files().is_empty() {
                    div {
                        style: "margin-top: 12px; padding: 12px; background: rgba(0, 0, 0, 0.3); border-radius: 8px; max-height: 150px; overflow-y: auto;",
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                            div {
                                style: "font-size: 14px; color: #22c55e;",
                                "✅ {selected_files().len()} file(s) selected ({format_total_size(&selected_files())})"
                            }
                            button {
                                style: "padding: 4px 12px; background: rgba(239, 68, 68, 0.2); border: 1px solid #ef4444; border-radius: 6px; color: #ef4444; cursor: pointer; font-size: 12px;",
                                onclick: move |_| {
                                    selected_files.set(Vec::new());
                                    status_message.set("Selection cleared".to_string());
                                },
                                "Clear"
                            }
                        }
                        for file in selected_files().iter().take(5) {
                            div {
                                style: "font-size: 12px; color: #888; padding: 4px 0; border-bottom: 1px solid rgba(255,255,255,0.1);",
                                "{file.file_name().and_then(|n| n.to_str()).unwrap_or(\"unknown\")}"
                            }
                        }
                        if selected_files().len() > 5 {
                            div {
                                style: "font-size: 12px; color: #888; padding: 4px 0; font-style: italic;",
                                "... and {selected_files().len() - 5} more"
                            }
                        }
                    }
                }
            }

            // Device List
            div {
                style: "margin-bottom: 24px;",
                h3 {
                    style: "margin: 0 0 12px 0; color: #e9d5ff; font-size: 18px;",
                    "📱 Nearby Devices ({devices().len()})"
                }
                
                if devices().is_empty() {
                    div {
                        style: "padding: 32px; text-align: center; background: rgba(0, 0, 0, 0.3); border-radius: 12px; border: 2px dashed rgba(168, 85, 247, 0.3);",
                        div {
                            style: "font-size: 48px; margin-bottom: 16px;",
                            "🔍"
                        }
                        p {
                            style: "margin: 0; color: #888; font-size: 16px;",
                            if is_scanning() {
                                "Scanning network for devices..."
                            } else {
                                "No devices found. Click 'Scan Network' to search."
                            }
                        }
                    }
                } else {
                    div {
                        style: "display: grid; gap: 12px;",
                        for device in devices().iter() {
                            div {
                                key: "{device.id}",
                                style: "padding: 16px; background: rgba(168, 85, 247, 0.1); border: 1px solid rgba(168, 85, 247, 0.3); border-radius: 12px; cursor: pointer; transition: all 0.3s;",
                                onclick: {
                                    let dev = device.clone();
                                    let files = selected_files();
                                    move |_| {
                                        if files.is_empty() {
                                            status_message.set("⚠️ Please select files first".to_string());
                                        } else {
                                            selected_device.set(Some(dev.clone()));
                                            let dev_clone = dev.clone();
                                            let files_clone = files.clone();
                                            spawn(async move {
                                                send_files_to_device(dev_clone, files_clone, status_message, current_session).await;
                                            });
                                        }
                                    }
                                },
                                
                                div {
                                    style: "display: flex; align-items: center; gap: 12px;",
                                    div {
                                        style: "font-size: 32px;",
                                        match device.device_type {
                                            crate::fastswap::DeviceType::Mobile => "📱",
                                            crate::fastswap::DeviceType::Desktop => "💻",
                                            crate::fastswap::DeviceType::Web => "🌐",
                                            crate::fastswap::DeviceType::Headless => "🖥️",
                                        }
                                    }
                                    div {
                                        style: "flex: 1;",
                                        div {
                                            style: "font-size: 16px; font-weight: bold; color: #e9d5ff; margin-bottom: 4px;",
                                            "{device.alias}"
                                        }
                                        div {
                                            style: "font-size: 12px; color: #888;",
                                            "{device.device_model} • {device.ip}:{device.port}"
                                        }
                                    }
                                    if !selected_files().is_empty() {
                                        div {
                                            style: "padding: 6px 12px; background: rgba(34, 197, 94, 0.2); border: 1px solid #22c55e; border-radius: 6px; font-size: 12px; color: #22c55e;",
                                            "Send Files"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Active Transfers
            if !active_transfers().is_empty() {
                div {
                    style: "margin-top: 24px; padding-top: 24px; border-top: 2px solid rgba(168, 85, 247, 0.3);",
                    h3 {
                        style: "margin: 0 0 12px 0; color: #e9d5ff; font-size: 18px;",
                        "📤 Active Transfers"
                    }
                    
                    div {
                        style: "display: grid; gap: 12px;",
                        for transfer in active_transfers().iter() {
                            div {
                                key: "{transfer.session_id}",
                                style: "padding: 16px; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(168, 85, 247, 0.3); border-radius: 12px;",
                                
                                // Overall progress
                                div {
                                    style: "margin-bottom: 12px;",
                                    div {
                                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                                        div {
                                            style: "font-size: 14px; font-weight: bold; color: #e9d5ff;",
                                            "📦 {transfer.files.len()} file(s) • {transfer.overall_progress():.1}%"
                                        }
                                        div {
                                            style: "font-size: 12px; color: #888;",
                                            "{format_bytes(transfer.transferred_bytes)} / {format_bytes(transfer.total_bytes)} • {format_speed(transfer.overall_speed())}"
                                        }
                                    }
                                    
                                    // Overall progress bar
                                    div {
                                        style: "width: 100%; height: 8px; background: rgba(0, 0, 0, 0.5); border-radius: 4px; overflow: hidden;",
                                        div {
                                            style: format!(
                                                "height: 100%; background: linear-gradient(90deg, #a855f7, #7c3aed); width: {}%; transition: width 0.3s;",
                                                transfer.overall_progress()
                                            ),
                                        }
                                    }
                                }
                                
                                // Individual files
                                div {
                                    style: "display: grid; gap: 8px;",
                                    for file in transfer.files.iter() {
                                        div {
                                            key: "{file.file_id}",
                                            style: "padding: 8px; background: rgba(0, 0, 0, 0.3); border-radius: 6px;",
                                            
                                            div {
                                                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;",
                                                div {
                                                    style: "font-size: 12px; color: #e9d5ff;",
                                                    "📄 {file.file_name}"
                                                }
                                                div {
                                                    style: format!(
                                                        "font-size: 11px; color: {};",
                                                        match file.status {
                                                            ProgressStatus::Completed => "#22c55e",
                                                            ProgressStatus::Failed(_) => "#ef4444",
                                                            ProgressStatus::Cancelled => "#f59e0b",
                                                            _ => "#a855f7",
                                                        }
                                                    ),
                                                    {
                                                        match &file.status {
                                                            ProgressStatus::Pending => "⏳ Pending".to_string(),
                                                            ProgressStatus::Transferring => "🔄 Transferring".to_string(),
                                                            ProgressStatus::Completed => "✅ Completed".to_string(),
                                                            ProgressStatus::Failed(e) => format!("❌ Failed: {}", e),
                                                            ProgressStatus::Cancelled => "🚫 Cancelled".to_string(),
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // File progress bar
                                            div {
                                                style: "width: 100%; height: 4px; background: rgba(0, 0, 0, 0.5); border-radius: 2px; overflow: hidden; margin-bottom: 4px;",
                                                div {
                                                    style: format!(
                                                        "height: 100%; background: linear-gradient(90deg, #a855f7, #7c3aed); width: {}%; transition: width 0.3s;",
                                                        file.progress_percent()
                                                    ),
                                                }
                                            }
                                            
                                            // File stats
                                            div {
                                                style: "display: flex; justify-content: space-between; font-size: 10px; color: #888;",
                                                div {
                                                    "{format_bytes(file.bytes_sent)} / {format_bytes(file.total_bytes)} ({file.progress_percent():.1}%)"
                                                }
                                                div {
                                                    "{file.format_speed()} • ETA: {file.format_eta()}"
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Cancel button
                                if !transfer.is_complete() {
                                    button {
                                        style: "margin-top: 12px; width: 100%; padding: 8px; background: rgba(239, 68, 68, 0.2); border: 1px solid #ef4444; border-radius: 6px; color: #ef4444; cursor: pointer; font-size: 12px;",
                                        onclick: {
                                            let session_id = transfer.session_id.clone();
                                            move |_| {
                                                let session_id_clone = session_id.clone();
                                                spawn(async move {
                                                    cancel_transfer(&session_id_clone).await;
                                                });
                                            }
                                        },
                                        "🚫 Cancel Transfer"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Helper function to scan for devices
async fn scan_for_devices(
    mut devices: Signal<Vec<Device>>,
    mut is_scanning: Signal<bool>,
    mut status_message: Signal<String>,
) {
    is_scanning.set(true);
    status_message.set("Scanning network for devices...".to_string());
    
    // Get local IP
    let local_ip = local_ip_address::local_ip()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)))
        .to_string();
    
    // Create discovery service and scan
    let discovery = crate::fastswap::network::DiscoveryService::new();
    match discovery.scan_network(&local_ip).await {
        Ok(found_devices) => {
            devices.set(found_devices.clone());
            status_message.set(format!("Found {} device(s)", found_devices.len()));
        }
        Err(e) => {
            status_message.set(format!("Scan failed: {}", e));
        }
    }
    
    is_scanning.set(false);
}

// Helper function to select files
async fn select_files(
    mut selected_files: Signal<Vec<PathBuf>>,
    mut status_message: Signal<String>,
) {
    match FileDialog::new().pick_files() {
        Some(paths) => {
            let mut all_files = Vec::new();
            
            for path in paths {
                if path.is_file() {
                    all_files.push(path);
                } else if path.is_dir() {
                    // If user selects a folder via file picker, add all files
                    collect_files_from_dir(&path, &mut all_files);
                }
            }
            
            if all_files.is_empty() {
                status_message.set("❌ No files found".to_string());
                return;
            }
            
            let total_size = calculate_total_size(&all_files);
            selected_files.set(all_files.clone());
            status_message.set(format!(
                "📁 Selected {} file(s) ({:.2} MB)",
                all_files.len(),
                total_size as f64 / 1_048_576.0
            ));
        }
        None => {
            status_message.set("File selection cancelled".to_string());
        }
    }
}

// Helper function to select folder
async fn select_folder(
    mut selected_files: Signal<Vec<PathBuf>>,
    mut status_message: Signal<String>,
) {
    match FileDialog::new().pick_folder() {
        Some(folder_path) => {
            let mut all_files = Vec::new();
            collect_files_from_dir(&folder_path, &mut all_files);
            
            if all_files.is_empty() {
                status_message.set("❌ No files found in folder".to_string());
                return;
            }
            
            let total_size = calculate_total_size(&all_files);
            let folder_name = folder_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("folder");
            
            selected_files.set(all_files.clone());
            status_message.set(format!(
                "📁 Selected folder '{}' with {} file(s) ({:.2} MB)",
                folder_name,
                all_files.len(),
                total_size as f64 / 1_048_576.0
            ));
        }
        None => {
            status_message.set("Folder selection cancelled".to_string());
        }
    }
}

// Recursive function to collect files from directory
fn collect_files_from_dir(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                collect_files_from_dir(&path, files); // Recursive
            }
        }
    }
}

// Helper function to send files to a device
async fn send_files_to_device(
    device: Device,
    files: Vec<PathBuf>,
    mut status_message: Signal<String>,
    mut current_session: Signal<Option<String>>,
) {
    if files.is_empty() {
        status_message.set("No files selected".to_string());
        return;
    }
    
    status_message.set(format!("Preparing to send {} file(s) to {}", files.len(), device.alias));
    
    // Get local device info
    let local_ip = local_ip_address::local_ip()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)))
        .to_string();
    
    let local_device = crate::fastswap::Device::new_local(
        format!("IGRIS-{}", whoami::username()),
        53317,
        local_ip
    );
    
    // Get global progress tracker
    let progress_tracker = crate::fastswap::get_progress_tracker();
    
    // Create transfer client
    let client = match crate::fastswap::network::TransferClient::new(progress_tracker.clone()) {
        Ok(c) => c,
        Err(e) => {
            status_message.set(format!("Failed to create transfer client: {}", e));
            return;
        }
    };
    
    status_message.set(format!("Sending files to {}...", device.alias));
    
    // Send files
    match client.send_files(&device, files.clone(), &local_device).await {
        Ok(session_id) => {
            current_session.set(Some(session_id.clone()));
            status_message.set(format!("Transferring {} file(s) to {}...", files.len(), device.alias));
            tracing::info!("Transfer started: {}", session_id);
        }
        Err(e) => {
            let error_msg = format!("Failed to send files: {}", e);
            status_message.set(error_msg.clone());
            tracing::error!("Transfer failed: {}", e);
        }
    }
}

// Helper function to cancel transfer
async fn cancel_transfer(session_id: &str) {
    let tracker = crate::fastswap::get_progress_tracker();
    let mut guard = tracker.write().await;
    if let Some(progress) = guard.get_mut(session_id) {
        progress.cancel();
        tracing::info!("Transfer cancelled: {}", session_id);
    }
}

// Helper function to calculate total size
fn calculate_total_size(files: &[PathBuf]) -> u64 {
    files.iter()
        .filter_map(|f| std::fs::metadata(f).ok())
        .map(|m| m.len())
        .sum()
}

// Helper function to format total size
fn format_total_size(files: &[PathBuf]) -> String {
    let total = calculate_total_size(files);
    format_bytes(total)
}

// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    crate::fastswap::models::progress::format_bytes(bytes)
}

// Helper function to format speed
fn format_speed(bytes_per_sec: f64) -> String {
    crate::fastswap::models::progress::format_bytes_per_second(bytes_per_sec)
}
