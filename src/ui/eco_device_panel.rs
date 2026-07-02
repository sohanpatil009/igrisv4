use dioxus::prelude::*;
use crate::eco::discovery::{DiscoveredEcoDevice, ECO_NETWORK_DEVICES, PENDING_PAIRING, PendingPair};
use crate::eco::pairing;

#[component]
pub fn EcoDevicePanel(primary_color: String, accent_rgb: String) -> Element {
    let mut devices = use_signal(|| Vec::<DiscoveredEcoDevice>::new());
    let mut is_scanning = use_signal(|| true);

    // Pairing state
    let mut show_otp_display = use_signal(|| false);
    let mut pairing_otp = use_signal(|| String::new());
    let mut pairing_target = use_signal(|| String::new());

    // Incoming pairing state
    let mut show_otp_input = use_signal(|| false);
    let mut incoming_sender_name = use_signal(|| String::new());
    let mut incoming_sender_id = use_signal(|| String::new());
    let mut incoming_pending_id = use_signal(|| String::new());
    let mut incoming_sender_addr = use_signal(|| String::new());
    let mut otp_input = use_signal(|| String::new());
    let mut pairing_status = use_signal(|| String::new());

    let mut dismiss_incoming = use_signal(|| false);

    let css = format!(r#"
        .ed-card {{
            position: relative; padding: 20px; border-radius: 4px; margin-bottom: 12px;
            background: linear-gradient(135deg, rgba(8,12,28,0.85), rgba(4,8,18,0.9));
            border: 1px solid rgba(255,255,255,0.06); transition: all 0.3s ease; overflow: hidden;
        }}
        .ed-card::before {{
            content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        }}
        .ed-card:hover {{ border-color: rgba({accent_rgb},0.25); }}
        .ed-empty {{ color: rgba(255,255,255,0.25); font-size: 12px; padding: 48px 0; text-align: center; font-family: monospace; }}
        .ed-scanline {{
            position: relative; overflow: hidden;
        }}
        .ed-scanline::after {{
            content: ''; position: absolute; top: 0; left: -100%; width: 100%; height: 100%;
            background: linear-gradient(90deg, transparent, rgba({accent_rgb},0.04), transparent);
            animation: scan 4s ease-in-out infinite;
        }}
        @keyframes scan {{
            0% {{ left: -100%; }}
            100% {{ left: 200%; }}
        }}
        @keyframes pulse-dot {{
            0%, 100% {{ opacity: 0.4; transform: scale(1); }}
            50% {{ opacity: 1; transform: scale(1.3); }}
        }}
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(10px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        @keyframes ripple {{
            0% {{ box-shadow: 0 0 0 0 rgba({accent_rgb}, 0.4); }}
            100% {{ box-shadow: 0 0 0 20px rgba({accent_rgb}, 0); }}
        }}
        .ed-device-card {{
            animation: fadeIn 0.4s ease-out;
            position: relative; padding: 16px 20px; border-radius: 4px; margin-bottom: 8px;
            background: linear-gradient(135deg, rgba(12,16,32,0.9), rgba(6,10,20,0.95));
            border: 1px solid rgba(255,255,255,0.06);
            transition: all 0.3s ease; display: flex; align-items: center; gap: 16px;
        }}
        .ed-device-card:hover {{ border-color: rgba({accent_rgb},0.2); }}
        .ed-btn {{
            padding: 6px 16px; border-radius: 4px; font-size: 11px; font-weight: 600;
            letter-spacing: 1px; cursor: pointer; transition: all 0.3s ease;
            font-family: 'JetBrains Mono', monospace; border: 1px solid transparent;
        }}
        .ed-btn-link {{
            background: rgba({accent_rgb},0.15); color: rgba({accent_rgb},0.9);
            border-color: rgba({accent_rgb},0.3);
        }}
        .ed-btn-link:hover {{ background: rgba({accent_rgb},0.25); }}
        .ed-btn-unlink {{
            background: rgba(239,68,68,0.15); color: #ef4444;
            border-color: rgba(239,68,68,0.3);
        }}
        .ed-btn-unlink:hover {{ background: rgba(239,68,68,0.25); }}
        .ed-otp-overlay {{
            position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 9999;
            background: rgba(0,0,0,0.85); backdrop-filter: blur(8px);
            display: flex; align-items: center; justify-content: center;
            animation: fadeIn 0.3s ease-out;
        }}
        .ed-otp-box {{
            background: linear-gradient(135deg, rgba(8,12,28,0.98), rgba(4,8,18,0.99));
            border: 1px solid rgba({accent_rgb},0.3); border-radius: 8px;
            padding: 40px; max-width: 420px; width: 90%; text-align: center;
        }}
        .ed-otp-digits {{
            font-size: 48px; font-weight: 700; letter-spacing: 12px;
            font-family: 'JetBrains Mono', monospace; color: {primary_color};
            padding: 16px; margin: 16px 0;
            background: rgba(0,0,0,0.3); border-radius: 4px;
        }}
        .ed-otp-input {{
            width: 200px; padding: 12px 16px; font-size: 28px; font-weight: 700;
            letter-spacing: 8px; text-align: center; font-family: 'JetBrains Mono', monospace;
            background: rgba(0,0,0,0.3); border: 1px solid rgba({accent_rgb},0.3);
            border-radius: 4px; color: {primary_color}; outline: none;
        }}
        .ed-otp-input:focus {{ border-color: {primary_color}; }}
        .ed-status-msg {{ font-size: 11px; color: rgba(255,255,255,0.4); margin-top: 8px; font-family: monospace; }}
    "#, accent_rgb = accent_rgb);

    // Poll discovered devices and incoming pairing requests
    use_effect(move || {
        spawn(async move {
            loop {
                let net_devices = ECO_NETWORK_DEVICES.read().await;
                let devs = net_devices.clone();
                drop(net_devices);
                devices.set(devs);
                is_scanning.set(false);

                    if !dismiss_incoming() {
                        let pending_list = PENDING_PAIRING.read().await;
                        if let Some(p) = pending_list.last() {
                            if !show_otp_display() {
                                show_otp_input.set(true);
                                incoming_sender_name.set(p.sender_name.clone());
                                incoming_sender_id.set(p.sender_id.clone());
                                incoming_pending_id.set(p.id.clone());
                                incoming_sender_addr.set(p.sender_addr.to_string());
                            }
                        }
                    }

                async_std::task::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    });

    let do_link = move |dev: DiscoveredEcoDevice| {
        let dev_id = dev.id.clone();
        let dev_name = dev.name.clone();
        let dev_ip = dev.ip.clone();
        let dev_port = dev.port;
        spawn(async move {
            let otp = pairing::generate_otp_code();
            let otp_hash = pairing::hash_otp_code(&otp);

            pairing_otp.set(otp.clone());
            pairing_target.set(dev_name.clone());
            show_otp_display.set(true);

            let local_id = pairing::get_local_device_id().unwrap_or_else(|| "unknown".to_string());
            let local_name = pairing::get_local_device_name().unwrap_or_else(|| whoami::username());

            let payload = serde_json::json!({
                "sender_id": local_id,
                "sender_name": local_name,
                "sender_port": 53328,
                "otp_hash": otp_hash,
            });

            let url = format!("https://{}:{}/api/ecosystem/v1/pair/request", dev_ip, dev_port);
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();

            match client.post(&url).json(&payload).send().await {
                Ok(_) => {}
                Err(e) => {
                    pairing_status.set(format!("Failed to send pairing request: {}", e));
                }
            }
        });
    };

    let do_unlink = move |dev: DiscoveredEcoDevice| {
        let dev_id = dev.id.clone();
        let dev_ip = dev.ip.clone();
        let dev_port = dev.port;
        spawn(async move {
            let payload = serde_json::json!({ "device_id": dev_id });
            let url = format!("https://{}:{}/api/ecosystem/v1/pair/untrust", dev_ip, dev_port);
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();
            let _ = client.post(&url).json(&payload).send().await;

            let mut net_devices = ECO_NETWORK_DEVICES.write().await;
            for d in net_devices.iter_mut() {
                if d.id == dev_id {
                    d.is_trusted = false;
                }
            }
        });
    };

    let do_submit_otp = move |_| {
        let otp_val = otp_input();
        let pending_id = incoming_pending_id();
        let sender_addr = incoming_sender_addr();
        let sender_name_val = incoming_sender_name();
        let sender_id_val = incoming_sender_id();
        let local_id = pairing::get_local_device_id().unwrap_or_else(|| "unknown".to_string());

        if otp_val.len() != 6 {
            pairing_status.set("OTP must be 6 digits".to_string());
            return;
        }

        spawn(async move {
            if sender_addr.is_empty() {
                pairing_status.set("Cannot verify: unknown sender address".to_string());
                return;
            }

            let payload = serde_json::json!({
                "pending_id": pending_id,
                "otp": otp_val,
                "remote_device_id": local_id,
            });

            let url = format!("https://{}/api/ecosystem/v1/pair/verify", sender_addr);
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();

            match client.post(&url).json(&payload).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data.get("trusted").and_then(|v| v.as_bool()).unwrap_or(false) {
                            let initiator_id = data.get("initiator_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&sender_id_val)
                                .to_string();
                            let mut net_devices = ECO_NETWORK_DEVICES.write().await;
                            for dev in net_devices.iter_mut() {
                                if dev.id == initiator_id {
                                    dev.is_trusted = true;
                                }
                            }
                            drop(net_devices);
                            let mut list = PENDING_PAIRING.write().await;
                            list.retain(|r| r.sender_id == initiator_id);
                            drop(list);
                            pairing_status.set(format!("Trusted with {}!", sender_name_val));
                            show_otp_input.set(false);
                            dismiss_incoming.set(true);
                            otp_input.set(String::new());
                        } else {
                            pairing_status.set("Wrong OTP. Try again.".to_string());
                        }
                    } else {
                        pairing_status.set("Invalid response from sender".to_string());
                    }
                }
                Err(e) => {
                    pairing_status.set(format!("Verification failed: {}", e));
                }
            }
        });
    };

    let do_dismiss_otp_display = move |_| {
        show_otp_display.set(false);
        pairing_otp.set(String::new());
        pairing_target.set(String::new());
    };

    let do_dismiss_incoming = move |_| {
        let pend_id = incoming_pending_id();
        show_otp_input.set(false);
        dismiss_incoming.set(true);
        otp_input.set(String::new());
        pairing_status.set(String::new());
        spawn(async move {
            let mut list = PENDING_PAIRING.write().await;
            list.retain(|r| r.id != pend_id);
        });
    };

    rsx! {
        div { style: format!("padding: 24px 32px; height: 100%; overflow-y: auto;"),
            style { "{css}" }

            div { style: format!("font-size: 14px; font-weight: 700; color: {}; letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; margin-bottom: 24px;", primary_color),
                "// ECOSYSTEM NODES"
            }

            // THIS STATION card
            div { class: "ed-card ed-scanline",
                div { style: "display: flex; align-items: center; gap: 16px;",
                    div { style: format!("width: 40px; height: 40px; border-radius: 50%; border: 1px solid rgba({}, 0.3); display: flex; align-items: center; justify-content: center; flex-shrink: 0;", accent_rgb),
                        div { style: format!("width: 8px; height: 8px; border-radius: 50%; background: {}; box-shadow: 0 0 12px {}; animation: pulse-dot 1.5s ease-in-out infinite;", primary_color, primary_color) }
                    }
                    div { style: "flex: 1;",
                        div { style: "font-size: 16px; font-weight: 600; color: #e5e7eb; font-family: 'JetBrains Mono', monospace;",
                            "THIS STATION"
                        }
                        div { style: "font-size: 11px; color: rgba(255,255,255,0.3); margin-top: 4px; font-family: monospace;",
                            "Clipboard sync :: ONLINE  ||  Port 53327/53328"
                        }
                    }
                    span { style: "padding: 2px 10px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba(34,197,94,0.12); color: #22c55e; font-family: monospace; border: 1px solid rgba(34,197,94,0.2);", "ACTIVE" }
                }
            }

            // Discovered devices list
            div { style: "margin-top: 16px;",
                div { style: format!("font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.4); letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; margin-bottom: 12px;"),
                    if is_scanning() {
                        "// SCANNING NETWORK ..."
                    } else {
                        {
                            let peer_count = devices().len();
                            format!("// PEERS  ( {} )", peer_count)
                        }
                    }
                }

                if devices().is_empty() && !is_scanning() {
                    div { class: "ed-empty",
                        div { style: "font-size: 12px; margin-bottom: 8px; color: rgba(255,255,255,0.3);",
                            "<NO_PEERS>"
                        }
                        div { style: "font-size: 11px; color: rgba(255,255,255,0.15);",
                            "Devices with IGRIS running will appear here automatically"
                        }
                    }
                }

                for dev in devices().iter() {
                    div { class: "ed-device-card", key: "{dev.id}",
                        // Status dot
                        div { style: format!("width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; animation: {};",
                            if dev.is_online { format!("pulse-dot 2s ease-in-out infinite; background: #22c55e; box-shadow: 0 0 8px #22c55e") }
                            else { "background: rgba(255,255,255,0.15)".to_string() }
                        )}

                        // Device info
                        div { style: "flex: 1; min-width: 0;",
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                div { style: "font-size: 14px; font-weight: 600; color: #e5e7eb; font-family: 'JetBrains Mono', monospace;",
                                    "{dev.name}"
                                }
                                if dev.is_trusted {
                                    span { style: "padding: 1px 8px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; background: rgba(34,197,94,0.12); color: #22c55e; font-family: monospace; border: 1px solid rgba(34,197,94,0.2);",
                                        "TRUSTED"
                                    }
                                }
                            }
                            div { style: "font-size: 10px; color: rgba(255,255,255,0.25); margin-top: 2px; font-family: monospace;",
                                "{dev.hostname}  ::  {dev.ip}:{dev.port}"
                            }
                        }

                        // Link / Unlink button
                        if dev.is_trusted {
                            button {
                                class: "ed-btn ed-btn-unlink",
                                onclick: {
                                    let d = dev.clone();
                                    move |_| do_unlink(d.clone())
                                },
                                "UNLINK"
                            }
                        } else {
                            button {
                                class: "ed-btn ed-btn-link",
                                onclick: {
                                    let d = dev.clone();
                                    move |_| do_link(d.clone())
                                },
                                "LINK"
                            }
                        }
                    }
                }
            }

            // Status message
            if !pairing_status().is_empty() {
                div { style: format!("margin-top: 12px; padding: 10px 16px; border-radius: 4px; font-size: 11px; font-family: monospace; color: rgba(255,255,255,0.6); background: rgba({}, 0.08); border: 1px solid rgba({}, 0.15);", accent_rgb, accent_rgb),
                    "{pairing_status}"
                }
            }
        }

        // SENDER: OTP display overlay
        if show_otp_display() {
            div { class: "ed-otp-overlay",
                div { class: "ed-otp-box",
                    div { style: format!("font-size: 24px; margin-bottom: 8px;"), "🔐" }
                    div { style: "font-size: 14px; font-weight: 600; color: #e5e7eb; margin-bottom: 4px; font-family: 'JetBrains Mono', monospace;",
                        "Pairing with {pairing_target}"
                    }
                    div { style: "font-size: 11px; color: rgba(255,255,255,0.4); margin-bottom: 8px; font-family: monospace;",
                        "Enter this code on the other device"
                    }
                    div { class: "ed-otp-digits", "{pairing_otp}" }
                    div { style: "font-size: 10px; color: rgba(255,255,255,0.2); margin-bottom: 16px; font-family: monospace;",
                        "Code expires in 2 minutes"
                    }
                    button {
                        class: "ed-btn",
                        style: format!("background: rgba(255,255,255,0.1); color: rgba(255,255,255,0.6); border-color: rgba(255,255,255,0.15);"),
                        onclick: do_dismiss_otp_display,
                        "DISMISS"
                    }
                }
            }
        }

        // RECEIVER: OTP input overlay
        if show_otp_input() {
            div { class: "ed-otp-overlay",
                div { class: "ed-otp-box",
                    div { style: "font-size: 24px; margin-bottom: 8px;", "🔗" }
                    div { style: "font-size: 14px; font-weight: 600; color: #e5e7eb; margin-bottom: 4px; font-family: 'JetBrains Mono', monospace;",
                        "Pairing Request"
                    }
                    div { style: "font-size: 11px; color: rgba(255,255,255,0.4); margin-bottom: 16px; font-family: monospace;",
                        "Enter the 6-digit code shown on {incoming_sender_name}'s screen"
                    }
                    input {
                        class: "ed-otp-input",
                        value: "{otp_input}",
                        oninput: move |e| otp_input.set(e.value()),
                        maxlength: "6",
                        placeholder: "------",
                        autofocus: "true",
                    }
                    div { style: "display: flex; gap: 12px; justify-content: center; margin-top: 16px;",
                        button {
                            class: "ed-btn",
                            style: format!("background: rgba({}, 0.2); color: {}; border-color: rgba({}, 0.4);", accent_rgb, primary_color, accent_rgb),
                            onclick: do_submit_otp,
                            "VERIFY & TRUST"
                        }
                        button {
                            class: "ed-btn",
                            style: "background: rgba(239,68,68,0.15); color: #ef4444; border-color: rgba(239,68,68,0.3);",
                            onclick: do_dismiss_incoming,
                            "DECLINE"
                        }
                    }
                }
            }
        }
    }
}
