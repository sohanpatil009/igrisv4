use dioxus::prelude::*;

#[component]
pub fn EcoDevicePanel(primary_color: String, accent_rgb: String) -> Element {
    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { r#"
                .ed-header {{ font-size: 22px; font-weight: bold; margin-bottom: 24px; letter-spacing: 0.5px; }}
                .ed-card {{ padding: 20px; border-radius: 12px; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); margin-bottom: 12px; display: flex; align-items: center; gap: 16px; transition: all 0.3s; }}
                .ed-card:hover {{ background: rgba(255,255,255,0.07); border-color: rgba(255,255,255,0.12); }}
                .ed-empty {{ color: #6b7280; font-size: 14px; padding: 48px 0; text-align: center; }}
                .ed-badge {{ padding: 3px 10px; border-radius: 6px; font-size: 11px; font-weight: 600; }}
            "# }

            div { class: "ed-header", style: format!("color: {};", primary_color), "🔗 Ecosystem Devices" }

            div { class: "ed-empty",
                div { style: "font-size: 48px; margin-bottom: 16px;", "🔍" }
                div { style: "font-size: 16px; color: #9ca3af; margin-bottom: 8px;", "Ecosystem clipboard sync is active" }
                div { style: "font-size: 13px; color: #6b7280;", "Devices on your network with IGRIS will appear here automatically" }
            }
        }
    }
}
