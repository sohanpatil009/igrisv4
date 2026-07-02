use dioxus::prelude::*;
use crate::{CHAT_MESSAGES, add_chat_message, clear_chat_history, get_selected_model, set_selected_model, get_selected_provider, set_selected_provider};

#[component]
pub fn ChatPanel(primary_color: String, accent_rgb: String) -> Element {
    let mut input = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut trigger = use_signal(|| 0);
    let mut show_model_dropdown = use_signal(|| false);
    let mut show_provider_dropdown = use_signal(|| false);
    let mut selected_model = use_signal(|| get_selected_model());
    let mut selected_provider = use_signal(|| get_selected_provider());

    let messages: Vec<(usize, String, String)> = {
        let guard = CHAT_MESSAGES.lock().unwrap();
        guard.iter().enumerate().map(|(i, m)| (i, m.role.clone(), m.content.clone())).collect()
    };

    let css = format!(r#"
        .chat-wrap {{ display: flex; flex-direction: column; height: 100%; }}
        .chat-msgs {{
            flex: 1; overflow-y: auto; padding: 20px 0;
            display: flex; flex-direction: column; gap: 16px;
        }}
        .chat-msg {{ display: flex; gap: 12px; width: 100%; padding: 0 50px; animation: fadeIn 0.25s ease-out; }}
        .chat-msg.usr {{ align-self: flex-end; flex-direction: row-reverse; }}
        .chat-msg.ast {{ align-self: flex-start; }}
        .chat-avatar {{
            width: 32px; height: 32px; border-radius: 50%; flex-shrink: 0;
            display: flex; align-items: center; justify-content: center;
            font-size: 13px; font-weight: 700; font-family: monospace;
        }}
        .chat-avatar.usr {{ background: rgba({accent_rgb}, 0.2); color: {primary}; }}
        .chat-avatar.ast {{ background: rgba(255,255,255,0.06); color: rgba(255,255,255,0.5); }}
        .chat-bubble {{
            padding: 10px 16px; font-size: 13px; line-height: 1.6;
            word-wrap: break-word; white-space: pre-wrap; position: relative;
        }}
        .chat-bubble.usr {{
            border-radius: 16px 16px 4px 16px;
            background: rgba({accent_rgb}, 0.12);
            border: 1px solid rgba({accent_rgb}, 0.15);
            color: #e5e7eb;
        }}
        .chat-bubble.ast {{
            border-radius: 16px 16px 16px 4px;
            background: rgba(255,255,255,0.04);
            border: 1px solid rgba(255,255,255,0.06);
            color: #d1d5db;
        }}
        .chat-role-label {{
            font-size: 10px; font-weight: 600; letter-spacing: 0.5px;
            margin-bottom: 4px; opacity: 0.5;
        }}
        .chat-empty {{
            color: rgba(255,255,255,0.12); font-family: monospace; font-size: 12px;
            text-align: center; padding: 60px 20px; display: flex; flex-direction: column;
            align-items: center; gap: 12px;
        }}
        .chat-empty-icon {{ font-size: 32px; opacity: 0.2; }}

        /* Typing indicator */
        .typing-dots {{ display: flex; gap: 4px; padding: 4px 0; }}
        .typing-dot {{
            width: 8px; height: 8px; border-radius: 50%;
            background: rgba(255,255,255,0.25);
            animation: typingBounce 1.2s ease-in-out infinite;
        }}
        .typing-dot:nth-child(1) {{ animation-delay: 0s; }}
        .typing-dot:nth-child(2) {{ animation-delay: 0.2s; }}
        .typing-dot:nth-child(3) {{ animation-delay: 0.4s; }}

        @keyframes typingBounce {{
            0%, 60%, 100% {{ transform: translateY(0); opacity: 0.25; }}
            30% {{ transform: translateY(-6px); opacity: 0.8; }}
        }}
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(6px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}

        .model-selector {{ position: relative; }}
        .model-btn, .provider-btn {{
            padding: 4px 10px; border-radius: 6px; border: 1px solid rgba({accent_rgb}, 0.08);
            background: rgba(0,0,0,0.15); cursor: pointer; color: rgba(255,255,255,0.35);
            font-family: monospace; font-size: 10px; letter-spacing: 0.3px;
            transition: all 0.2s; display: flex; align-items: center; gap: 5px;
            white-space: nowrap;
        }}
        .model-btn:hover, .provider-btn:hover {{ border-color: rgba({accent_rgb}, 0.2); color: rgba(255,255,255,0.7); }}
        .model-dropdown, .provider-dropdown {{
            position: absolute; top: calc(100% + 4px); right: 0;
            width: 200px; background: rgba(10,16,30,0.98);
            border: 1px solid rgba({accent_rgb}, 0.15);
            border-radius: 10px; overflow: hidden;
            box-shadow: 0 -8px 40px rgba(0,0,0,0.6);
            z-index: 50; max-height: 280px; overflow-y: auto;
        }}
        .model-dropdown {{ width: 280px; }}
        .model-option, .provider-option {{
            padding: 10px 14px; cursor: pointer;
            font-family: monospace; font-size: 11px;
            color: rgba(255,255,255,0.5);
            transition: all 0.15s;
            border-bottom: 1px solid rgba(255,255,255,0.03);
            display: flex; align-items: center; justify-content: space-between;
        }}
        .model-option:hover, .provider-option:hover {{ background: rgba({accent_rgb}, 0.06); color: rgba(255,255,255,0.85); }}
        .model-option.active, .provider-option.active {{ color: {primary}; background: rgba({accent_rgb}, 0.1); }}
        .model-check, .provider-check {{ opacity: 0.5; font-size: 12px; }}
        .model-option.active .model-check, .provider-option.active .provider-check {{ opacity: 1; }}

        /* Edge-to-edge input row with send button inside */
        .chat-input-row {{
            display: flex; padding: 0; flex-shrink: 0; position: relative;
            background: rgba(5,10,20,0.6);
        }}
        .chat-input-wrap {{
            flex: 1; position: relative; display: flex;
        }}
        .chat-input {{
            width: 100%; height: 50px; padding: 14px 56px 14px 20px;
            border: none; border-top: 1px solid rgba({accent_rgb}, 0.06);
            background: rgba(0,0,0,0.3); color: #e5e7eb;
            font-family: 'Inter', 'Segoe UI', sans-serif; font-size: 14px;
            outline: none; transition: border-color 0.2s;
            resize: none;
        }}
        .chat-input::placeholder {{ color: rgba(255,255,255,0.12); }}
        .chat-send {{
            position: absolute; right: 10px; top: 50%; transform: translateY(-50%);
            width: 36px; height: 36px; border-radius: 8px; border: none; cursor: pointer;
            display: flex; align-items: center; justify-content: center;
            transition: all 0.2s;
            background: rgba({accent_rgb}, 0.15);
            color: {primary}; font-size: 16px;
        }}
        .chat-send:hover {{ background: rgba({accent_rgb}, 0.35); }}
        .chat-send:disabled {{ opacity: 0.2; cursor: not-allowed; }}
    "#, primary = primary_color, accent_rgb = accent_rgb);

    let do_send = move |_evt: dioxus::prelude::Event<dioxus::prelude::MouseData>| {
        let text = input().trim().to_string();
        if text.is_empty() || loading() { return; }
        add_chat_message("user", &text);
        input.set(String::new());
        loading.set(true);
        trigger.set(trigger() + 1);
        spawn(async move {
            let system_prompt = format!(
                "You are IGRIS, a helpful AI assistant. Keep responses concise but friendly. \
                 Current date: {}. Respond naturally to the user's query.",
                chrono::Local::now().format("%Y-%m-%d %H:%M"),
            );
            match crate::online::reason_online(&system_prompt, &text).await {
                Ok(resp) => add_chat_message("assistant", &resp),
                Err(e) => add_chat_message("assistant", &format!("[Error: {}]", e)),
            }
            loading.set(false);
            trigger.set(trigger() + 1);
        });
    };

    rsx! {
        div { style: "height: 100%; display: flex; flex-direction: column; overflow: hidden; position: relative;",
            style { "{css}" }

            // Header
            div { style: format!("display: flex; align-items: center; justify-content: space-between; padding: 14px 24px; border-bottom: 1px solid rgba({}, 0.06); flex-shrink: 0;", accent_rgb),
                div { style: "display: flex; align-items: center; gap: 12px;",
                    div { style: format!("width: 8px; height: 8px; border-radius: 50%; background: {}; box-shadow: 0 0 10px {};", primary_color, primary_color) }
                    div { style: format!("font-size: 14px; font-weight: 700; color: {}; letter-spacing: 1.5px; font-family: 'JetBrains Mono', monospace;", primary_color),
                        "Chat"
                    }
                }
                div { style: "display: flex; align-items: center; gap: 6px;",
                    // Provider selector
                    div { class: "model-selector",
                        button {
                            class: "provider-btn",
                            onclick: move |_| show_provider_dropdown.set(!show_provider_dropdown()),
                            "{selected_provider} ▾"
                        }
                        if show_provider_dropdown() {
                            div { class: "provider-dropdown",
                                for provider in crate::online::LlmProvider::all() {
                                    div {
                                        class: if selected_provider() == provider.key() { "provider-option active" } else { "provider-option" },
                                        onclick: move |_| {
                                            set_selected_provider(provider.key());
                                            selected_provider.set(provider.key().to_string());
                                            show_provider_dropdown.set(false);
                                        },
                                        span { "{provider.name()}" }
                                        span { class: "provider-check",
                                            if selected_provider() == provider.key() { "✓" } else { "" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Model selector
                    div { class: "model-selector",
                        button {
                            class: "model-btn",
                            onclick: move |_| show_model_dropdown.set(!show_model_dropdown()),
                            "Model: {selected_model} ▾"
                        }
                        if show_model_dropdown() {
                            div { class: "model-dropdown",
                                for (model_id, display_name) in crate::online::AVAILABLE_MODELS {
                                    div {
                                        class: if selected_model() == *model_id { "model-option active" } else { "model-option" },
                                        onclick: move |_| {
                                            set_selected_model(model_id);
                                            selected_model.set(model_id.to_string());
                                            show_model_dropdown.set(false);
                                        },
                                        span { "{display_name}" }
                                        span { class: "model-check",
                                            if selected_model() == *model_id { "✓" } else { "" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    button {
                        style: format!(
                            "padding: 6px 12px; border-radius: 8px; border: 1px solid rgba(239,68,68,0.15); \
                             background: rgba(239,68,68,0.06); cursor: pointer; color: rgba(239,68,68,0.6); \
                             font-family: monospace; font-size: 9px; letter-spacing: 1px; transition: all 0.2s;"
                        ),
                        onclick: move |_| {
                            clear_chat_history();
                            trigger.set(trigger() + 1);
                        },
                        "Clear"
                    }
                }
            }

            // Messages area
            div { class: "chat-msgs",
                if messages.is_empty() {
                    div { class: "chat-empty",
                        div { class: "chat-empty-icon", "💬" }
                        div { "Start a conversation with IGRIS" }
                        div { style: "font-size: 11px; color: rgba(255,255,255,0.08);", "Ask me anything!" }
                    }
                } else {
                    for (i, role, content) in messages.clone().into_iter() {
                        div {
                            class: if role == "user" { "chat-msg usr" } else { "chat-msg ast" },
                            key: "{i}",
                            div {
                                class: if role == "user" { "chat-avatar usr" } else { "chat-avatar ast" },
                                if role == "user" { "U" } else { "AI" }
                            }
                            div {
                                class: if role == "user" { "chat-bubble usr" } else { "chat-bubble ast" },
                                div { class: "chat-role-label",
                                    if role == "user" { "You" } else { "IGRIS" }
                                }
                                "{content}"
                            }
                        }
                    }
                }
                if loading() {
                    div { class: "chat-msg ast", key: "typing_{trigger}",
                        div { class: "chat-avatar ast", "AI" }
                        div { class: "chat-bubble ast",
                            div { class: "typing-dots",
                                div { class: "typing-dot" }
                                div { class: "typing-dot" }
                                div { class: "typing-dot" }
                            }
                        }
                    }
                }
            }

            // Chat input area
            div { class: "chat-input-row",
                div { class: "chat-input-wrap",
                    input {
                        class: "chat-input",
                        placeholder: "Message IGRIS...",
                        value: "{input}",
                        oninput: move |e| input.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !e.data().modifiers().contains(dioxus::prelude::Modifiers::SHIFT) {
                                let text = input().trim().to_string();
                                if !text.is_empty() && !loading() {
                                    add_chat_message("user", &text);
                                    input.set(String::new());
                                    loading.set(true);
                                    trigger.set(trigger() + 1);
                                    spawn(async move {
                                        let sp = format!(
                                            "You are IGRIS, a helpful AI assistant. Keep responses concise but friendly. \
                                             Current date: {}. Respond naturally to the user's query.",
                                            chrono::Local::now().format("%Y-%m-%d %H:%M"),
                                        );
                                        match crate::online::reason_online(&sp, &text).await {
                                            Ok(resp) => add_chat_message("assistant", &resp),
                                            Err(e) => add_chat_message("assistant", &format!("[Error: {}]", e)),
                                        }
                                        loading.set(false);
                                        trigger.set(trigger() + 1);
                                    });
                                }
                            }
                        },
                    }
                    button {
                        class: "chat-send",
                        disabled: loading() || input().trim().is_empty(),
                        onclick: do_send,
                        "➤"
                    }
                }
            }
        }
    }
}
