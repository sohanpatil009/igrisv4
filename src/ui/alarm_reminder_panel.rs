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
        msg.set("Alarm cancelled".to_string());
        refresh_trigger.set(refresh_trigger() + 1);
    };

    let mut cancel_reminder = move |id: u32| {
        REMINDER_MANAGER.remove_reminder(id);
        msg.set("Reminder cancelled".to_string());
        refresh_trigger.set(refresh_trigger() + 1);
    };

    let alarms_list: Vec<(u32, String, String)> = alarms().iter()
        .map(|a| (a.id, format_time(&a.time), format_date(&a.time)))
        .collect();
    let reminders_list: Vec<(u32, String, String)> = reminders().iter()
        .map(|r| (r.id, r.message.clone(), format!("{} at {}", format_date(&r.trigger_time), format_time(&r.trigger_time))))
        .collect();

    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { r#"
                .ar-header {{ font-size: 22px; font-weight: bold; margin-bottom: 24px; letter-spacing: 0.5px; }}
                .ar-section {{ margin-bottom: 28px; }}
                .ar-section-title {{ font-size: 12px; text-transform: uppercase; letter-spacing: 1.5px; color: #6b7280; margin-bottom: 12px; }}
                .ar-card {{ padding: 16px; border-radius: 12px; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); margin-bottom: 8px; display: flex; align-items: center; justify-content: space-between; transition: all 0.3s; }}
                .ar-card:hover {{ background: rgba(255,255,255,0.07); border-color: rgba(255,255,255,0.12); }}
                .ar-btn {{ padding: 6px 14px; border-radius: 8px; border: none; cursor: pointer; font-size: 12px; font-weight: 600; transition: all 0.2s; color: #fff; }}
                .ar-btn:hover {{ opacity: 0.8; }}
                .ar-empty {{ color: #6b7280; font-size: 14px; padding: 24px 0; text-align: center; }}
                .ar-badge {{ display: inline-block; padding: 2px 8px; border-radius: 6px; font-size: 11px; font-weight: 600; }}
            "# }

            div { class: "ar-header", style: format!("color: {};", primary_color), "⏰ Alarms & Reminders" }

            if !msg().is_empty() {
                div { style: format!("padding: 10px 16px; border-radius: 8px; background: rgba(34,197,94,0.1); border: 1px solid rgba(34,197,94,0.3); color: #22c55e; font-size: 13px; margin-bottom: 16px;"),
                    "{msg}"
                }
            }

            div { class: "ar-section",
                div { class: "ar-section-title", "Alarms" }
                if alarms_list.is_empty() {
                    div { class: "ar-empty", "No active alarms. Say \"Set alarm for 7 am\" to add one." }
                } else {
                    for (a_id, a_time, a_date) in alarms_list.clone().into_iter() {
                        div { class: "ar-card", key: "{a_id}",
                            div { style: "display: flex; align-items: center; gap: 12px;",
                                span { style: "font-size: 24px;", "🔔" }
                                div {
                                    div { style: format!("font-size: 16px; font-weight: 600; color: {};", primary_color),
                                        "{a_time}"
                                    }
                                    div { style: "font-size: 12px; color: #6b7280; margin-top: 2px;",
                                        "{a_date}"
                                    }
                                }
                            }
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                div { class: "ar-badge", style: "background: rgba(34,197,94,0.15); color: #22c55e;", "Active" }
                                button {
                                    class: "ar-btn",
                                    style: "background: rgba(239,68,68,0.8);",
                                    onclick: move |_| cancel_alarm(a_id),
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "ar-section",
                div { class: "ar-section-title", "Reminders" }
                if reminders_list.is_empty() {
                    div { class: "ar-empty", "No active reminders. Say \"Remind me in 30 minutes\" to add one." }
                } else {
                    for (r_id, r_msg, r_time) in reminders_list.clone().into_iter() {
                        div { class: "ar-card", key: "{r_id}",
                            div { style: "display: flex; align-items: center; gap: 12px;",
                                span { style: "font-size: 24px;", "📌" }
                                div {
                                    div { style: "font-size: 14px; font-weight: 500; color: #e5e7eb;",
                                        "{r_msg}"
                                    }
                                    div { style: "font-size: 12px; color: #6b7280; margin-top: 2px;",
                                        "{r_time}"
                                    }
                                }
                            }
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                div { class: "ar-badge", style: "background: rgba(59,130,246,0.15); color: #3b82f6;", "Pending" }
                                button {
                                    class: "ar-btn",
                                    style: "background: rgba(239,68,68,0.8);",
                                    onclick: move |_| cancel_reminder(r_id),
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
