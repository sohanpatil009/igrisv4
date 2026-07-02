use dioxus::prelude::*;
use crate::eco::notification::{NotificationData, get_notifications};

#[component]
pub fn NotificationPanel(primary_color: String, accent_rgb: String) -> Element {
    let mut notifications = use_signal(|| Vec::<NotificationData>::new());
    let mut reply_texts = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut sending = use_signal(|| String::new());

    let notif_count = notifications().len();
    let notif_header = format!("{} notifications  ||  Syncing from linked devices", notif_count);

    // Poll notifications
    use_effect(move || {
        spawn(async move {
            loop {
                let notifs = get_notifications();
                notifications.set(notifs);
                async_std::task::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    });

    let css = format!(r#"
        .np-card {{
            position: relative; padding: 20px; border-radius: 4px; margin-bottom: 12px;
            background: linear-gradient(135deg, rgba(8,12,28,0.85), rgba(4,8,18,0.9));
            border: 1px solid rgba(255,255,255,0.06); transition: all 0.3s ease; overflow: hidden;
        }}
        .np-card::before {{
            content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        }}
        .np-card:hover {{ border-color: rgba({accent_rgb},0.25); }}
        .np-notif-item {{
            animation: fadeIn 0.4s ease-out;
            position: relative; padding: 16px 20px; border-radius: 4px; margin-bottom: 8px;
            background: linear-gradient(135deg, rgba(12,16,32,0.9), rgba(6,10,20,0.95));
            border: 1px solid rgba(255,255,255,0.06);
            transition: all 0.3s ease;
        }}
        .np-notif-item:hover {{ border-color: rgba({accent_rgb},0.2); }}
        .np-notif-item.unread {{ border-left: 2px solid {primary_color}; }}
        .np-reply-input {{
            width: 100%; padding: 8px 12px; font-size: 12px; font-family: 'JetBrains Mono', monospace;
            background: rgba(0,0,0,0.3); border: 1px solid rgba({accent_rgb},0.2);
            border-radius: 4px; color: #e5e7eb; outline: none; resize: none;
        }}
        .np-reply-input:focus {{ border-color: {primary_color}; }}
        .np-reply-btn {{
            padding: 6px 16px; border-radius: 4px; font-size: 11px; font-weight: 600;
            letter-spacing: 1px; cursor: pointer; transition: all 0.3s ease;
            font-family: 'JetBrains Mono', monospace; border: 1px solid transparent;
            background: rgba({accent_rgb},0.15); color: {primary_color};
            border-color: rgba({accent_rgb},0.3);
        }}
        .np-reply-btn:hover {{ background: rgba({accent_rgb},0.25); }}
        .np-reply-btn:disabled {{ opacity: 0.5; cursor: not-allowed; }}
        .np-empty {{ color: rgba(255,255,255,0.25); font-size: 12px; padding: 48px 0; text-align: center; font-family: monospace; }}
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(10px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
    "#, accent_rgb = accent_rgb, primary_color = primary_color);

    let mut do_reply = move |notif_id: String| {
        let text = reply_texts.with(|m| m.get(&notif_id).cloned().unwrap_or_default());
        if text.trim().is_empty() {
            return;
        }
        let notif_id_clone = notif_id.clone();
        sending.set(notif_id.clone());
        spawn(async move {
            // Send reply via eco manager
            if let Ok(mut guard) = crate::eco::manager::ECO_MANAGER.lock() {
                if let Some(ref mut manager) = *guard {
                    let _ = manager.reply_to_notification(&notif_id_clone, &text).await;
                }
            }
            // Clear the input
            reply_texts.write().remove(&notif_id);
            sending.set(String::new());
        });
    };

    rsx! {
        div { style: "padding: 24px 32px; height: 100%; overflow-y: auto;",
            style { "{css}" }

            div { style: format!("font-size: 14px; font-weight: 700; color: {}; letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; margin-bottom: 24px;", primary_color),
                "// NOTIFICATION SYNC"
            }

            // Header info
            div { class: "np-card",
                div { style: "display: flex; align-items: center; gap: 16px;",
                    div { style: format!("width: 40px; height: 40px; border-radius: 50%; border: 1px solid rgba({}, 0.3); display: flex; align-items: center; justify-content: center; flex-shrink: 0;", accent_rgb),
                        div { style: format!("width: 8px; height: 8px; border-radius: 50%; background: {}; box-shadow: 0 0 12px {}; animation: pulse-dot 1.5s ease-in-out infinite;", primary_color, primary_color) }
                    }
                    div { style: "flex: 1;",
                        div { style: "font-size: 16px; font-weight: 600; color: #e5e7eb; font-family: 'JetBrains Mono', monospace;",
                            "NOTIFICATION CENTER"
                        }
                        div { style: "font-size: 11px; color: rgba(255,255,255,0.3); margin-top: 4px; font-family: monospace;",
                            "{notif_header}"
                        }
                    }
                }
            }

            // Notifications list
            div { style: "margin-top: 16px;",
                div { style: format!("font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.4); letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; margin-bottom: 12px;"),
                    {
                        let unread = notifications().iter().filter(|n| !n.read).count();
                        format!("// NOTIFICATIONS  ( {} total, {} unread )", notifications().len(), unread)
                    }
                }

                if notifications().is_empty() {
                    div { class: "np-empty",
                        div { style: "font-size: 12px; margin-bottom: 8px; color: rgba(255,255,255,0.3);",
                            "<NO_NOTIFICATIONS>"
                        }
                        div { style: "font-size: 11px; color: rgba(255,255,255,0.15);",
                            "Notifications from linked devices will appear here"
                        }
                    }
                }

                for notif in notifications().iter() {
                    div {
                        class: format!("np-notif-item {}", if notif.read { "" } else { "unread" }),
                        key: "{notif.id}",

                        // Header: device name + app name + time
                        div { style: "display: flex; align-items: center; gap: 8px; margin-bottom: 8px;",
                            span { style: format!("padding: 2px 8px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba({}, 0.12); color: {}; font-family: monospace; border: 1px solid rgba({}, 0.2);",
                                accent_rgb, primary_color, accent_rgb),
                                "{notif.device_name}"
                            }
                            span { style: "padding: 2px 8px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba(255,255,255,0.06); color: rgba(255,255,255,0.5); font-family: monospace; border: 1px solid rgba(255,255,255,0.08);",
                                "{notif.app_name}"
                            }
                            span { style: "font-size: 10px; color: rgba(255,255,255,0.2); font-family: monospace; margin-left: auto;",
                                {
                                    let now = chrono::Utc::now().timestamp_millis();
                                    let diff_ms = now - notif.timestamp;
                                    let diff_secs = diff_ms / 1000;
                                    if diff_secs < 60 {
                                        format!("{}s ago", diff_secs)
                                    } else if diff_secs < 3600 {
                                        format!("{}m ago", diff_secs / 60)
                                    } else if diff_secs < 86400 {
                                        format!("{}h ago", diff_secs / 3600)
                                    } else {
                                        format!("{}d ago", diff_secs / 86400)
                                    }
                                }
                            }
                        }

                        // Title
                        div { style: "font-size: 14px; font-weight: 600; color: #e5e7eb; font-family: 'JetBrains Mono', monospace; margin-bottom: 4px;",
                            "{notif.title}"
                        }

                        // Body
                        div { style: "font-size: 12px; color: rgba(255,255,255,0.5); font-family: monospace; margin-bottom: 12px; line-height: 1.5;",
                            "{notif.body}"
                        }

                        // Reply input
                        div { style: "display: flex; gap: 8px; align-items: flex-end;",
                            input {
                                class: "np-reply-input",
                                value: reply_texts.with(|m| m.get(&notif.id).cloned().unwrap_or_default()),
                                oninput: {
                                    let notif_id = notif.id.clone();
                                    move |e: Event<FormData>| {
                                        reply_texts.write().insert(notif_id.clone(), e.value());
                                    }
                                },
                                placeholder: "Type reply...",
                                disabled: sending() == notif.id,
                            }
                            button {
                                class: "np-reply-btn",
                                disabled: sending() == notif.id,
                                onclick: {
                                    let notif_id = notif.id.clone();
                                    move |_| do_reply(notif_id.clone())
                                },
                                if sending() == notif.id { "..." } else { ">>" }
                            }
                        }
                    }
                }
            }
        }
    }
}
