// src/ui/menu_button.rs
// Menu button with dropdown for settings and file share

use dioxus::prelude::*;

#[component]
pub fn MenuButton(
    settings_open: Signal<bool>,
    fastswap_open: Signal<bool>,
) -> Element {
    let mut menu_open = use_signal(|| false);

    rsx! {
        div {
            style: "position: fixed; top: clamp(12px, 3vh, 24px); right: clamp(12px, 3vw, 24px); z-index: 50;",
            
            // Menu button
            button {
                style: "background: rgba(255, 255, 255, 0.1); border: 2px solid rgba(255, 255, 255, 0.2); color: white; width: 48px; height: 48px; border-radius: 12px; cursor: pointer; font-size: 24px; display: flex; align-items: center; justify-content: center; transition: all 0.3s ease; backdrop-filter: blur(10px);",
                onclick: move |_| menu_open.set(!menu_open()),
                "☰"
            }

            // Dropdown menu
            if menu_open() {
                div {
                    style: "position: absolute; top: 60px; right: 0; background: linear-gradient(135deg, #1e293b 0%, #334155 100%); border-radius: 16px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5); border: 1px solid rgba(255, 255, 255, 0.1); overflow: hidden; min-width: 220px; backdrop-filter: blur(10px);",
                    
                    // FastSwap option
                    button {
                        style: "width: 100%; background: transparent; border: none; color: white; padding: 16px 20px; text-align: left; cursor: pointer; font-size: 15px; display: flex; align-items: center; gap: 12px; transition: background 0.2s;",
                        onclick: move |_| {
                            menu_open.set(false);
                            fastswap_open.set(true);
                        },
                        span { style: "font-size: 20px;", "⚡" }
                        span { "FastSwap" }
                    }
                    
                    // Divider
                    div {
                        style: "height: 1px; background: rgba(255, 255, 255, 0.1); margin: 4px 0;"
                    }
                    
                    // Settings option
                    button {
                        style: "width: 100%; background: transparent; border: none; color: white; padding: 16px 20px; text-align: left; cursor: pointer; font-size: 15px; display: flex; align-items: center; gap: 12px; transition: background 0.2s;",
                        onclick: move |_| {
                            menu_open.set(false);
                            settings_open.set(true);
                        },
                        span { style: "font-size: 20px;", "⚙️" }
                        span { "Settings" }
                    }
                }

                // Backdrop to close menu
                div {
                    style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: -1;",
                    onclick: move |_| menu_open.set(false),
                }
            }
        }

        // Hover styles
        style { "
            button:hover {{
                background: rgba(255, 255, 255, 0.15) !important;
            }}
        " }
    }
}
