use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Chat,
    FastSwap,
    Devices,
    Notifications,
    Alarms,
    Reminders,
    SystemInfo,
}

#[component]
pub fn Sidebar(
    active_tab: Signal<Tab>,
    is_awake: bool,
    primary_color: String,
    accent_rgb: String,
) -> Element {
    let tabs = [
        (Tab::Dashboard, "CORE", "Dashboard"),
        (Tab::Chat, "CHAT", "Chat"),
        (Tab::FastSwap, "XFER", "File Share"),
        (Tab::Devices, "NODE", "Devices"),
        (Tab::Notifications, "NOTIF", "Notifications"),
        (Tab::Alarms, "TIME", "Alarms"),
        (Tab::Reminders, "MEMO", "Reminders"),
        (Tab::SystemInfo, "SYS", "System"),
    ];

    rsx! {
        style { r#"
            .sib-item {{
                display: flex; align-items: center; gap: 12px; padding: 14px 16px;
                cursor: pointer; transition: all 0.3s ease; position: relative;
                border-left: 2px solid transparent; color: rgba(255,255,255,0.4);
                font-family: 'JetBrains Mono', 'Courier New', monospace;
            }}
            .sib-item:hover {{
                background: rgba(255,255,255,0.03); color: rgba(255,255,255,0.7);
            }}
            .sib-badge {{
                font-size: 9px; letter-spacing: 2px; font-weight: 700; min-width: 42px;
            }}
            .sib-label {{
                font-size: 13px; font-weight: 400; letter-spacing: 0.5px;
            }}
        "# }
        div {
            style: format!(
                "width: 220px; height: 100vh; background: linear-gradient(180deg, rgba(8,12,28,0.98), rgba(4,8,18,0.99)); \
                 border-right: 1px solid rgba({}, 0.15); display: flex; flex-direction: column; \
                 flex-shrink: 0; z-index: 30; transition: border-color 0.5s ease-in-out; \
                 box-shadow: 4px 0 30px rgba(0,0,0,0.5);",
                accent_rgb,
            ),
            div { style: format!("padding: 28px 20px 18px; border-bottom: 1px solid rgba({}, 0.1); margin-bottom: 12px;", accent_rgb),
                div { style: format!("font-size: 10px; font-weight: 700; color: {}; letter-spacing: 3px; transition: color 0.5s; font-family: 'JetBrains Mono', monospace;", primary_color),
                    "// SYSTEM NAV"
                }
                div { style: "font-size: 9px; color: rgba(255,255,255,0.15); letter-spacing: 1px; margin-top: 4px; font-family: monospace;",
                    ">> v1.0.0_eco"
                }
            }
            div { style: "display: flex; flex-direction: column; gap: 2px; padding: 4px 8px; flex: 1; overflow-y: auto;",
                for (tab, badge, label) in tabs {
                    div {
                        class: "sib-item",
                        style: format!(
                            "background: {}; {}",
                            if active_tab() == tab { format!("rgba({}, 0.08)", accent_rgb) } else { "transparent".to_string() },
                            if active_tab() == tab {
                                format!("border-left: 2px solid {}; color: {};", primary_color, primary_color)
                            } else {
                                "".to_string()
                            }
                        ),
                        onclick: move |_| active_tab.set(tab),
                        span { class: "sib-badge", "{badge}" }
                        span { class: "sib-label", "{label}" }
                        if active_tab() == tab {
                            div { style: format!("position: absolute; right: 12px; width: 4px; height: 4px; border-radius: 50%; background: {}; box-shadow: 0 0 8px {}; animation: pulse 1.5s ease-in-out infinite;", primary_color, primary_color) }
                        }
                    }
                }
            }
            div { style: format!("padding: 16px 20px; border-top: 1px solid rgba({}, 0.1);", accent_rgb),
                div { style: "display: flex; align-items: center; gap: 10px; font-size: 10px; color: rgba(255,255,255,0.3); font-family: monospace;",
                    div {
                        style: format!(
                            "width: 6px; height: 6px; border-radius: 50%; background: {}; \
                             box-shadow: 0 0 10px {}; animation: pulse 2s ease-in-out infinite; transition: all 0.5s;",
                            if is_awake { &primary_color } else { "#22c55e" },
                            if is_awake { &primary_color } else { "#22c55e" },
                        ),
                    }
                    "STATUS: "
                    if is_awake { "LISTENING" } else { "STANDBY" }
                }
            }
        }
    }
}
