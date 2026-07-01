use dioxus::prelude::*;
use crate::commands::system::get_system_info;

#[component]
pub fn SystemInfoPanel(primary_color: String, accent_rgb: String) -> Element {
    let mut os_info = use_signal(|| "Loading...".to_string());
    let mut mem_info = use_signal(|| "Loading...".to_string());
    let mut cpu_info = use_signal(|| "Loading...".to_string());
    let mut uptime_info = use_signal(|| "Loading...".to_string());
    let mut ip_info = use_signal(|| "Loading...".to_string());
    let hostname = use_signal(|| {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    });

    use_effect(move || {
        spawn(async move {
            os_info.set(get_system_info("os"));
            mem_info.set(get_system_info("memory"));
            cpu_info.set(get_system_info("cpu"));
            uptime_info.set(get_system_info("uptime"));
            ip_info.set(get_system_info("ip"));
        });
    });

    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { r#"
                .si-header {{ font-size: 22px; font-weight: bold; margin-bottom: 24px; letter-spacing: 0.5px; }}
                .si-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }}
                .si-card {{ padding: 20px; border-radius: 12px; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); transition: all 0.3s; }}
                .si-card:hover {{ background: rgba(255,255,255,0.07); border-color: rgba(255,255,255,0.12); }}
                .si-card-label {{ font-size: 11px; text-transform: uppercase; letter-spacing: 1.5px; color: #6b7280; margin-bottom: 8px; }}
                .si-card-value {{ font-size: 14px; color: #e5e7eb; font-family: 'JetBrains Mono', monospace; word-break: break-word; }}
                .si-card-icon {{ font-size: 28px; margin-bottom: 12px; }}
                .si-btn {{ padding: 8px 20px; border-radius: 8px; border: none; cursor: pointer; font-size: 13px; font-weight: 600; color: #fff; transition: all 0.2s; }}
                .si-btn:hover {{ opacity: 0.85; }}
                @media (max-width: 700px) {{ .si-grid {{ grid-template-columns: 1fr; }} }}
            "# }

            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px;",
                div { class: "si-header", style: format!("color: {};", primary_color), "ℹ️ System Information" }
                button {
                    class: "si-btn",
                    style: format!("background: rgba({}, 0.8);", accent_rgb),
                    onclick: move |_| {
                        spawn(async move {
                            os_info.set(get_system_info("os"));
                            mem_info.set(get_system_info("memory"));
                            cpu_info.set(get_system_info("cpu"));
                            uptime_info.set(get_system_info("uptime"));
                            ip_info.set(get_system_info("ip"));
                        });
                    },
                    "🔄 Refresh"
                }
            }

            div { class: "si-grid",
                div { class: "si-card",
                    div { class: "si-card-icon", "💻" }
                    div { class: "si-card-label", "Operating System" }
                    div { class: "si-card-value", "{os_info}" }
                }
                div { class: "si-card",
                    div { class: "si-card-icon", "🧠" }
                    div { class: "si-card-label", "Memory" }
                    div { class: "si-card-value", "{mem_info}" }
                }
                div { class: "si-card",
                    div { class: "si-card-icon", "⚡" }
                    div { class: "si-card-label", "Processor" }
                    div { class: "si-card-value", "{cpu_info}" }
                }
                div { class: "si-card",
                    div { class: "si-card-icon", "⏱️" }
                    div { class: "si-card-label", "Uptime" }
                    div { class: "si-card-value", "{uptime_info}" }
                }
                div { class: "si-card",
                    div { class: "si-card-icon", "🌐" }
                    div { class: "si-card-label", "Public IP" }
                    div { class: "si-card-value", "{ip_info}" }
                }
                div { class: "si-card",
                    div { class: "si-card-icon", "🖥️" }
                    div { class: "si-card-label", "Hostname" }
                    div { class: "si-card-value", "{hostname}" }
                }
            }
        }
    }
}
