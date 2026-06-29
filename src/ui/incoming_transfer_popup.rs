use dioxus::prelude::*;
use crate::fastswap::PendingTransfer;

#[component]
pub fn IncomingTransferPopup(pending_transfers: Signal<Vec<PendingTransfer>>) -> Element {
    let mut status_message = use_signal(|| String::new());
    
    // Only show if there are pending transfers
    if pending_transfers().is_empty() {
        return rsx! {};
    }
    
    // Show the first pending transfer
    let transfer = pending_transfers()[0].clone();
    
    rsx! {
        // Full screen overlay
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 9999; background: rgba(0, 0, 0, 0.8); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center;",
            
            // Dialog box
            div {
                style: "width: 90%; max-width: 500px; background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); border: 3px solid #a855f7; border-radius: 20px; padding: 32px; box-shadow: 0 20px 60px rgba(168, 85, 247, 0.5), 0 0 100px rgba(168, 85, 247, 0.3);",
                onclick: move |e| e.stop_propagation(),
                
                // Icon and title
                div {
                    style: "text-align: center; margin-bottom: 24px;",
                    div {
                        style: "font-size: 64px; margin-bottom: 16px;",
                        "📨"
                    }
                    h2 {
                        style: "margin: 0 0 8px 0; color: #e9d5ff; font-size: 24px; font-weight: bold;",
                        "Incoming File Transfer"
                    }
                    div {
                        style: "font-size: 16px; color: #a855f7; font-weight: bold;",
                        "{transfer.sender_name} wants to send you files"
                    }
                }
                
                // Sender info
                div {
                    style: "margin-bottom: 24px; padding: 16px; background: rgba(0, 0, 0, 0.3); border-radius: 12px; border-left: 4px solid #a855f7;",
                    div {
                        style: "font-size: 14px; color: #888; margin-bottom: 8px;",
                        "From Device:"
                    }
                    div {
                        style: "font-size: 16px; color: #e9d5ff; font-weight: bold;",
                        "{transfer.sender_device}"
                    }
                }
                
                // File details
                div {
                    style: "margin-bottom: 24px; padding: 16px; background: rgba(168, 85, 247, 0.1); border-radius: 12px; border: 1px solid rgba(168, 85, 247, 0.3);",
                    div {
                        style: "font-size: 18px; color: #e9d5ff; margin-bottom: 12px; font-weight: bold;",
                        "📦 {transfer.file_count} file(s) • {format_bytes(transfer.total_size)}"
                    }
                    div {
                        style: "max-height: 150px; overflow-y: auto; padding: 8px; background: rgba(0, 0, 0, 0.3); border-radius: 8px;",
                        for (i, file) in transfer.files.iter().enumerate().take(10) {
                            div {
                                key: "{i}",
                                style: "padding: 6px 0; border-bottom: 1px solid rgba(255,255,255,0.1); font-size: 13px; color: #888;",
                                "📄 {file}"
                            }
                        }
                        if transfer.files.len() > 10 {
                            div {
                                style: "padding: 6px 0; font-style: italic; font-size: 13px; color: #888;",
                                "... and {transfer.files.len() - 10} more files"
                            }
                        }
                    }
                }
                
                // Status message
                if !status_message().is_empty() {
                    div {
                        style: "margin-bottom: 16px; padding: 12px; background: rgba(168, 85, 247, 0.2); border-radius: 8px; text-align: center; font-size: 14px; color: #e9d5ff;",
                        "{status_message}"
                    }
                }
                
                // Action buttons
                div {
                    style: "display: flex; gap: 16px;",
                    button {
                        style: "flex: 1; padding: 16px; background: linear-gradient(135deg, #22c55e, #16a34a); border: none; border-radius: 12px; color: white; cursor: pointer; font-size: 16px; font-weight: bold; transition: all 0.3s; box-shadow: 0 4px 12px rgba(34, 197, 94, 0.4);",
                        onclick: {
                            let session_id = transfer.session_id.clone();
                            let sender = transfer.sender_name.clone();
                            move |_| {
                                let session_id_clone = session_id.clone();
                                let sender_clone = sender.clone();
                                spawn(async move {
                                    crate::fastswap::approve_transfer(&session_id_clone).await;
                                    status_message.set(format!("✅ Accepted! Receiving files from {}...", sender_clone));
                                    
                                    // Wait a moment to show the message
                                    async_std::task::sleep(std::time::Duration::from_millis(1500)).await;
                                });
                            }
                        },
                        "✅ Accept"
                    }
                    button {
                        style: "flex: 1; padding: 16px; background: linear-gradient(135deg, #ef4444, #dc2626); border: none; border-radius: 12px; color: white; cursor: pointer; font-size: 16px; font-weight: bold; transition: all 0.3s; box-shadow: 0 4px 12px rgba(239, 68, 68, 0.4);",
                        onclick: {
                            let session_id = transfer.session_id.clone();
                            let sender = transfer.sender_name.clone();
                            move |_| {
                                let session_id_clone = session_id.clone();
                                let sender_clone = sender.clone();
                                spawn(async move {
                                    crate::fastswap::deny_transfer(&session_id_clone).await;
                                    status_message.set(format!("❌ Denied transfer from {}", sender_clone));
                                    
                                    // Wait a moment to show the message
                                    async_std::task::sleep(std::time::Duration::from_millis(1000)).await;
                                });
                            }
                        },
                        "❌ Deny"
                    }
                }
                
                // Help text
                div {
                    style: "margin-top: 20px; text-align: center; font-size: 12px; color: #888;",
                    "Files will be saved to your Downloads folder"
                }
            }
        }
    }
}

// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    crate::fastswap::models::progress::format_bytes(bytes)
}
