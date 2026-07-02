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

    let css = format!(r#"
        .si-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }}
        .si-card {{
            position: relative; padding: 20px; border-radius: 4px;
            background: linear-gradient(135deg, rgba(8,12,28,0.85), rgba(4,8,18,0.9));
            border: 1px solid rgba(255,255,255,0.06);
            transition: all 0.3s ease;
            overflow: hidden;
        }}
        .si-card::before {{
            content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        }}
        .si-card::after {{
            content: ''; position: absolute; bottom: 8px; right: 8px; width: 10px; height: 10px;
            border-right: 1px solid rgba(255,255,255,0.08);
            border-bottom: 1px solid rgba(255,255,255,0.08);
        }}
        .si-card:nth-child(1)::before {{ background: linear-gradient(90deg, transparent, {accent}, transparent); }}
        .si-card:nth-child(6)::before {{ background: linear-gradient(90deg, transparent, {accent}, transparent); }}
        .si-card:hover {{ border-color: rgba({accent_rgb},0.3); background: linear-gradient(135deg, rgba(12,18,34,0.9), rgba(6,12,24,0.95)); }}
        .si-card-label {{ font-size: 10px; text-transform: uppercase; letter-spacing: 2px; color: rgba(255,255,255,0.35); margin-bottom: 8px; font-family: 'JetBrains Mono', monospace; }}
        .si-card-value {{ font-size: 14px; color: #e5e7eb; font-family: 'JetBrains Mono', monospace; word-break: break-word; }}
        .si-card-tag {{ font-size: 9px; color: rgba(255,255,255,0.1); font-family: monospace; margin-top: 8px; letter-spacing: 0.5px; }}
        @media (max-width: 700px) {{ .si-grid {{ grid-template-columns: 1fr; }} }}
    "#, accent = primary_color, accent_rgb = accent_rgb);

    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { "{css}" }

            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px;",
                div { style: format!("font-size: 14px; font-weight: 700; color: {}; letter-spacing: 2px; font-family: 'JetBrains Mono', monospace;", primary_color),
                    "// SYSTEM DIAGNOSTICS"
                }
                button {
                    style: format!(
                        "padding: 6px 14px; border-radius: 4px; border: 1px solid rgba({}, 0.3); \
                         background: rgba({}, 0.08); cursor: pointer; font-size: 10px; font-weight: 600; \
                         color: {}; letter-spacing: 1px; transition: all 0.2s; font-family: monospace;",
                        accent_rgb, accent_rgb, primary_color
                    ),
                    onclick: move |_| {
                        spawn(async move {
                            os_info.set(get_system_info("os"));
                            mem_info.set(get_system_info("memory"));
                            cpu_info.set(get_system_info("cpu"));
                            uptime_info.set(get_system_info("uptime"));
                            ip_info.set(get_system_info("ip"));
                        });
                    },
                    ">> REFRESH"
                }
            }

            div { class: "si-grid",
                div { class: "si-card",
                    div { class: "si-card-label", "Operating System" }
                    div { class: "si-card-value", "{os_info}" }
                    div { class: "si-card-tag", "SYS_01" }
                }
                div { class: "si-card",
                    div { class: "si-card-label", "Memory" }
                    div { class: "si-card-value", "{mem_info}" }
                    div { class: "si-card-tag", "SYS_02" }
                }
                div { class: "si-card",
                    div { class: "si-card-label", "Processor" }
                    div { class: "si-card-value", "{cpu_info}" }
                    div { class: "si-card-tag", "SYS_03" }
                }
                div { class: "si-card",
                    div { class: "si-card-label", "Uptime" }
                    div { class: "si-card-value", "{uptime_info}" }
                    div { class: "si-card-tag", "SYS_04" }
                }
                div { class: "si-card",
                    div { class: "si-card-label", "Public IP" }
                    div { class: "si-card-value", "{ip_info}" }
                    div { class: "si-card-tag", "SYS_05" }
                }
                div { class: "si-card",
                    div { class: "si-card-label", "Hostname" }
                    div { class: "si-card-value", "{hostname}" }
                    div { class: "si-card-tag", "SYS_06" }
                }
            }
        }
    }
}
