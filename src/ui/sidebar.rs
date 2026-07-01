use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    FastSwap,
    Devices,
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
        (Tab::Dashboard, "🎛️", "Dashboard"),
        (Tab::FastSwap, "📁", "File Share"),
        (Tab::Devices, "🔗", "Devices"),
        (Tab::Alarms, "⏰", "Alarms"),
        (Tab::Reminders, "📋", "Reminders"),
        (Tab::SystemInfo, "ℹ️", "System"),
    ];

    rsx! {
        style { r#"
            .sidebar-nav-item {{
                display: flex; align-items: center; gap: 12px; padding: 12px 14px;
                border-radius: 10px; cursor: pointer; transition: all 0.3s ease;
                border-left: 3px solid transparent; color: #9ca3af;
            }}
            .sidebar-nav-item:hover {{
                background: rgba(255,255,255,0.05); color: #d1d5db;
            }}
        "# }
        div {
            style: format!(
                "width: 220px; height: 100vh; background: linear-gradient(180deg, rgba(15,15,35,0.95), rgba(10,10,20,0.98)); \
                 border-right: 1px solid rgba({}, 0.2); display: flex; flex-direction: column; \
                 backdrop-filter: blur(20px); flex-shrink: 0; z-index: 30; transition: border-color 0.5s ease-in-out;",
                accent_rgb,
            ),
            div { style: "padding: 24px 20px 16px; border-bottom: 1px solid rgba(255,255,255,0.05); margin-bottom: 8px;",
                div { style: format!("font-size: 14px; font-weight: bold; color: {}; letter-spacing: 1px; transition: color 0.5s;", primary_color),
                    "NAVIGATION"
                }
            }
            div { style: "display: flex; flex-direction: column; gap: 2px; padding: 4px 8px; flex: 1; overflow-y: auto;",
                for (tab, icon, label) in tabs {
                    div {
                        class: "sidebar-nav-item",
                        style: format!(
                            "background: {}; {}",
                            if active_tab() == tab { format!("rgba({}, 0.15)", accent_rgb) } else { "transparent".to_string() },
                            if active_tab() == tab {
                                format!("border-left: 3px solid {}; color: {};", primary_color, primary_color)
                            } else {
                                "".to_string()
                            }
                        ),
                        onclick: move |_| active_tab.set(tab),
                        span { style: "font-size: 18px;", "{icon}" }
                        span { style: "font-size: 13px; font-weight: 500;", "{label}" }
                    }
                }
            }
            div { style: "padding: 16px 20px; border-top: 1px solid rgba(255,255,255,0.05);",
                div { style: "display: flex; align-items: center; gap: 8px; font-size: 12px; color: #6b7280;",
                    div {
                        style: format!(
                            "width: 8px; height: 8px; border-radius: 50%; background: {}; animation: pulse 2s ease-in-out infinite; transition: background 0.5s;",
                            if is_awake { primary_color.clone() } else { "#22c55e".to_string() }
                        ),
                    }
                    if is_awake { "Listening" } else { "Standby" }
                }
            }
        }
    }
}
