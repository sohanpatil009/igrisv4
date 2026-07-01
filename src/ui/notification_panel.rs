use dioxus::prelude::*;

#[derive(Clone, Debug)]
struct NotificationItem {
    id: String,
    notification_id: String,
    app_name: String,
    title: String,
    body: String,
    source_device: String,
    source_device_name: String,
    timestamp: i64,
    reply_allowed: bool,
    reply_id: Option<String>,
    replied: bool,
    read: bool,
}

#[component]
pub fn NotificationPanel(show: bool, on_close: EventHandler<()>) -> Element {
    let mut notifications = use_signal(|| Vec::<NotificationItem>::new());
    let mut selected = use_signal(|| None::<usize>);
    let mut reply_text = use_signal(|| String::new());
    let mut sending_reply = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(500)).await;
                let entries = crate::eco::notifications::get_notification_list();
                let items: Vec<NotificationItem> = entries.iter().rev().map(|e| NotificationItem {
                    id: e.id.clone(),
                    notification_id: e.notification_id.clone(),
                    app_name: e.app_name.clone(),
                    title: e.title.clone(),
                    body: e.body.clone(),
                    source_device: e.source_device.clone(),
                    source_device_name: e.source_device_name.clone(),
                    timestamp: e.timestamp,
                    reply_allowed: e.reply_allowed,
                    reply_id: e.reply_id.clone(),
                    replied: e.replied,
                    read: e.read,
                }).collect();
                notifications.set(items);
            }
        });
    });

    if !show {
        return rsx! {};
    }

    let unread_count = notifications().iter().filter(|n| !n.read).count();

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 9000; background: rgba(0,0,0,0.5); backdrop-filter: blur(4px);",
            onclick: move |_| on_close.call(()),

            div {
                style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); width: 90%; max-width: 600px; max-height: 80vh; background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); border: 1px solid rgba(6, 182, 212, 0.3); border-radius: 20px; padding: 24px; box-shadow: 0 20px 60px rgba(0,0,0,0.5); display: flex; flex-direction: column;",
                onclick: move |e| e.stop_propagation(),

                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h2 { style: "color: #fff; font-size: 20px; margin: 0;",
                        "Notifications"
                        if unread_count > 0 {
                            span { style: "margin-left: 8px; font-size: 13px; color: #06b6d4; background: rgba(6,182,212,0.15); padding: 2px 10px; border-radius: 10px;",
                                "{unread_count} new"
                            }
                        }
                    }
                }

                div { style: "flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; min-height: 200px;",
                    if notifications().is_empty() {
                        div { style: "text-align: center; color: #6b7280; padding: 48px 0; font-size: 14px;",
                            "No notifications yet"
                        }
                    }

                    for (idx, item) in notifications().iter().cloned().enumerate() {
                        div {
                            style: format!(
                                "padding: 12px 16px; border-radius: 10px; cursor: pointer; border: 1px solid {}; background: {}; transition: all 0.2s;",
                                if selected() == Some(idx) { "rgba(6, 182, 212, 0.5)" } else { "rgba(255,255,255,0.05)" },
                                if selected() == Some(idx) { "rgba(6, 182, 212, 0.1)" } else { "rgba(255,255,255,0.02)" },
                            ),
                            onclick: {
                                let item = item.clone();
                                move |_| {
                                    if !item.read {
                                        crate::eco::notifications::mark_notification_read(&item.id);
                                    }
                                    selected.set(Some(idx));
                                    reply_text.set(String::new());
                                }
                            },

                            div { style: "display: flex; justify-content: space-between; align-items: start; gap: 8px;",
                                div { style: "flex: 1; min-width: 0;",
                                    div { style: "display: flex; align-items: center; gap: 8px; margin-bottom: 4px;",
                                        if !item.read {
                                            div { style: "width: 8px; height: 8px; border-radius: 50%; background: #06b6d4; flex-shrink: 0;", }
                                        }
                                        span { style: "font-size: 11px; color: #06b6d4; text-transform: uppercase; letter-spacing: 0.5px;",
                                            "{item.app_name}"
                                        }
                                        span { style: "font-size: 11px; color: #6b7280;",
                                            "· {item.source_device_name}"
                                        }
                                    }
                                    div { style: "font-size: 14px; color: #fff; font-weight: 600; margin-bottom: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                        "{item.title}"
                                    }
                                    if selected() != Some(idx) {
                                        div { style: "font-size: 13px; color: #9ca3af; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            "{item.body}"
                                        }
                                    }
                                }
                            }

                            if selected() == Some(idx) {
                                div { style: "margin-top: 12px; padding-top: 12px; border-top: 1px solid rgba(255,255,255,0.08);",
                                    div { style: "font-size: 13px; color: #d1d5db; line-height: 1.5; margin-bottom: 12px; white-space: pre-wrap;",
                                        "{item.body}"
                                    }

                                    if item.reply_allowed && item.replied {
                                        div { style: "font-size: 12px; color: #6b7280; font-style: italic;",
                                            "Reply sent"
                                        }
                                    }

                                    if item.reply_allowed && !item.replied {
                                        div { style: "display: flex; gap: 8px;",
                                            input {
                                                style: "flex: 1; padding: 8px 12px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); background: rgba(0,0,0,0.3); color: #fff; font-size: 13px; outline: none;",
                                                placeholder: "Type your reply...",
                                                value: "{reply_text}",
                                                disabled: sending_reply(),
                                                oninput: move |e| reply_text.set(e.value()),
                                            }
                                            button {
                                                style: format!(
                                                    "padding: 8px 16px; border-radius: 8px; border: none; background: {}; color: #fff; font-size: 13px; font-weight: 600; cursor: {}; transition: all 0.2s;",
                                                    if sending_reply() { "rgba(255,255,255,0.1)" } else { "linear-gradient(135deg, #06b6d4, #3b82f6)" },
                                                    if sending_reply() { "not-allowed" } else { "pointer" },
                                                ),
                                                disabled: sending_reply(),
                                                onclick: {
                                                    let item = item.clone();
                                                    move |_| {
                                                        let text = reply_text();
                                                        if text.trim().is_empty() { return; }
                                                        let notif_id = item.notification_id.clone();
                                                        let rid = item.reply_id.clone().unwrap_or_default();
                                                        let dev = item.source_device.clone();
                                                        let dev_name = item.source_device_name.clone();
                                                        let item_id = item.id.clone();
                                                        sending_reply.set(true);
                                                        spawn(async move {
                                                            crate::eco::notifications::send_notification_reply(
                                                                &notif_id, &rid, &text, &dev, &dev_name
                                                            ).await;
                                                            crate::eco::notifications::mark_notification_replied(&item_id);
                                                            sending_reply.set(false);
                                                        });
                                                    }
                                                },
                                                "Send"
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
    }
}
