use dioxus::prelude::*;
use crate::commands::reminders::REMINDER_MANAGER;

fn format_time(dt: &chrono::DateTime<chrono::Local>) -> String {
    dt.format("%I:%M %p").to_string()
}

fn format_date(dt: &chrono::DateTime<chrono::Local>) -> String {
    let today = chrono::Local::now();
    if dt.date_naive() == today.date_naive() {
        "Today".to_string()
    } else if dt.date_naive() == (today + chrono::Duration::days(1)).date_naive() {
        "Tomorrow".to_string()
    } else {
        dt.format("%b %d").to_string()
    }
}

#[component]
pub fn AlarmReminderPanel(primary_color: String, accent_rgb: String) -> Element {
    let mut alarms = use_signal(|| REMINDER_MANAGER.list_alarms());
    let mut reminders = use_signal(|| REMINDER_MANAGER.list_reminders());
    let mut refresh_trigger = use_signal(|| 0);
    let mut msg = use_signal(|| String::new());

    use_effect(move || {
        let _ = refresh_trigger();
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_secs(2)).await;
                alarms.set(REMINDER_MANAGER.list_alarms());
                reminders.set(REMINDER_MANAGER.list_reminders());
            }
        });
    });

    let mut cancel_alarm = move |id: u32| {
        REMINDER_MANAGER.remove_alarm(id);
        msg.set("Alarm terminated.".to_string());
        refresh_trigger.set(refresh_trigger() + 1);
    };

    let mut cancel_reminder = move |id: u32| {
        REMINDER_MANAGER.remove_reminder(id);
        msg.set("Reminder terminated.".to_string());
        refresh_trigger.set(refresh_trigger() + 1);
    };

    let alarms_list: Vec<(u32, String, String)> = alarms().iter()
        .map(|a| (a.id, format_time(&a.time), format_date(&a.time)))
        .collect();
    let reminders_list: Vec<(u32, String, String)> = reminders().iter()
        .map(|r| (r.id, r.message.clone(), format!("{} at {}", format_date(&r.trigger_time), format_time(&r.trigger_time))))
        .collect();

    let css = format!(r#"
        .ar-section {{ margin-bottom: 28px; }}
        .ar-section-title {{ font-size: 10px; text-transform: uppercase; letter-spacing: 2px; color: rgba(255,255,255,0.3); margin-bottom: 12px; font-family: 'JetBrains Mono', monospace; }}
        .ar-card {{
            position: relative; padding: 16px; border-radius: 4px;
            background: linear-gradient(135deg, rgba(8,12,28,0.85), rgba(4,8,18,0.9));
            border: 1px solid rgba(255,255,255,0.06); margin-bottom: 8px;
            display: flex; align-items: center; justify-content: space-between;
            transition: all 0.3s ease; overflow: hidden;
        }}
        .ar-card::before {{
            content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        }}
        .ar-card:hover {{ border-color: rgba({accent_rgb},0.25); background: linear-gradient(135deg, rgba(12,18,34,0.9), rgba(6,12,24,0.95)); }}
        .ar-btn {{
            padding: 5px 12px; border-radius: 4px; border: none; cursor: pointer;
            font-size: 10px; font-weight: 600; letter-spacing: 1px;
            transition: all 0.2s; font-family: monospace; color: #fff;
        }}
        .ar-btn:hover {{ opacity: 0.8; }}
        .ar-empty {{ color: rgba(255,255,255,0.25); font-size: 12px; padding: 32px 0; text-align: center; font-family: monospace; }}
    "#, accent_rgb = accent_rgb);

    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { "{css}" }

            div { style: format!("font-size: 14px; font-weight: 700; color: {}; letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; margin-bottom: 24px;", primary_color),
                "// SCHEDULED TASKS"
            }

            if !msg().is_empty() {
                div { style: format!("padding: 8px 14px; border-radius: 4px; background: rgba({}, 0.08); border: 1px solid rgba({}, 0.25); color: {}; font-size: 11px; margin-bottom: 16px; font-family: monospace;", accent_rgb, accent_rgb, primary_color),
                    ">> {msg}"
                }
            }

            div { class: "ar-section",
                div { class: "ar-section-title", "// ALARM_SEQUENCE" }
                if alarms_list.is_empty() {
                    div { class: "ar-empty", "<EMPTY> No alarms scheduled. Voice command: \"Set alarm for 7 AM\"" }
                } else {
                    for (a_id, a_time, a_date) in alarms_list.clone().into_iter() {
                        div { class: "ar-card", key: "{a_id}",
                            div { style: "display: flex; align-items: center; gap: 12px;",
                                span { style: "font-size: 18px;", "⏰" }
                                div {
                                    div { style: format!("font-size: 15px; font-weight: 600; color: {}; font-family: 'JetBrains Mono', monospace;", primary_color),
                                        "{a_time}"
                                    }
                                    div { style: "font-size: 11px; color: rgba(255,255,255,0.3); margin-top: 2px; font-family: monospace;",
                                        "{a_date}"
                                    }
                                }
                            }
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "padding: 2px 8px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba(34,197,94,0.12); color: #22c55e; font-family: monospace; border: 1px solid rgba(34,197,94,0.2);", "ACTIVE" }
                                button {
                                    class: "ar-btn",
                                    style: "background: rgba(239,68,68,0.2); border: 1px solid rgba(239,68,68,0.3); color: #ef4444;",
                                    onclick: move |_| cancel_alarm(a_id),
                                    "CANCEL"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "ar-section",
                div { class: "ar-section-title", "// REMINDER_QUEUE" }
                if reminders_list.is_empty() {
                    div { class: "ar-empty", "<EMPTY> No reminders pending. Voice command: \"Remind me in 30 minutes\"" }
                } else {
                    for (r_id, r_msg, r_time) in reminders_list.clone().into_iter() {
                        div { class: "ar-card", key: "{r_id}",
                            div { style: "display: flex; align-items: center; gap: 12px;",
                                span { style: "font-size: 18px;", "📋" }
                                div {
                                    div { style: "font-size: 13px; font-weight: 500; color: #d1d5db; font-family: 'JetBrains Mono', monospace;",
                                        "{r_msg}"
                                    }
                                    div { style: "font-size: 11px; color: rgba(255,255,255,0.3); margin-top: 2px; font-family: monospace;",
                                        "{r_time}"
                                    }
                                }
                            }
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                span { style: "padding: 2px 8px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba(59,130,246,0.12); color: #3b82f6; font-family: monospace; border: 1px solid rgba(59,130,246,0.2);", "PENDING" }
                                button {
                                    class: "ar-btn",
                                    style: "background: rgba(239,68,68,0.2); border: 1px solid rgba(239,68,68,0.3); color: #ef4444;",
                                    onclick: move |_| cancel_reminder(r_id),
                                    "CANCEL"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
