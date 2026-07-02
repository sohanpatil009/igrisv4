#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod state;
mod tools;
mod processor;
mod voice;

use dioxus::prelude::*;
use igrisv3::eco;
use dioxus::desktop::{Config, WindowBuilder};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use tokio::sync::mpsc;

use igrisv3::{
    config, utils, setup_manager, fastswap, online,
    SEARCH_STATE, RESET_FLAG,
};
use igrisv3::config::CONFIG;
use igrisv3::setup_manager::gui::{is_setup_complete, SetupGui};
use igrisv3::setup_manager::{SetupManager, SetupUI};
use igrisv3::utils::shared_memory::init_shared_memory;
use igrisv3::ui::{
    SettingsPanel, MenuButton, SearchResultsPanel, SearchResultItem,
    CameraPanel, PresentationPanel, FastSwapPanel, IncomingTransferPopup,
    Sidebar, Tab, AlarmReminderPanel, SystemInfoPanel, EcoDevicePanel, ChatPanel,
};
use igrisv3::commands::ffmpeg_camera::{CameraPanelState, CAMERA_PANEL_STATE};

use crate::state::*;
use crate::tools::*;
use crate::voice::*;

fn main() {
    if let Err(e) = utils::hotkey::register_global_hotkey(|| {
        println!("[HOTKEY] Ctrl+Shift+Space pressed - Resetting IGRIS");
        RESET_FLAG.store(true, Ordering::Relaxed);
        if let Err(e) = utils::greetings::speak_invoke_greeting() {
            eprintln!("[HOTKEY] Failed to speak greeting: {}", e);
        }
    }) {
        eprintln!("[HOTKEY] Failed to register global hotkey: {}", e);
        eprintln!("[HOTKEY] You can still use the application window");
    }

    thread::spawn(|| {
        start_setup_and_assistant();
    });

    let window = WindowBuilder::new()
        .with_title("IGRIS Voice Assistant")
        .with_visible(true)
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1100.0, 700.0))
        .with_window_icon(Some(load_icon()));

    let cfg = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_disable_context_menu(true);

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .launch(App);
}

fn load_icon() -> dioxus::desktop::tao::window::Icon {
    let icon_data = include_bytes!("../icons/igris_icon.ico");
    let image = image::load_from_memory(icon_data)
        .expect("Failed to load icon")
        .to_rgba8();
    let (width, height) = image.dimensions();
    dioxus::desktop::tao::window::Icon::from_rgba(image.into_raw(), width, height)
        .expect("Failed to create icon")
}

fn start_setup_and_assistant() {
    thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            run_setup_and_assistant().await;
        });
    });
}

async fn run_setup_and_assistant() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("[LAUNCH] IGRIS v3 - Voice Assistant (Hybrid Offline/Online)");
    println!("═══════════════════════════════════════════════════════\n");

    let _ = dotenv::dotenv();

    println!("[NET] Checking internet connectivity...");
    let has_internet = online::check_internet_connectivity().await;

    if has_internet {
        let has_api_key = std::env::var("NVIDIA_API_KEY").is_ok();
        if has_api_key {
            println!("[NET] Internet OK + NVIDIA_API_KEY found — auto-enabling online mode");
            online::enable_online_mode();
            println!("[ONLINE] Online mode active (NVIDIA NIM: Parakeet STT + NIM chat)");
            println!("[ONLINE] Offline STT loaded for wake word; SBERT NLU & local LLM skipped");
            println!("[ONLINE] Will auto-switch to offline if internet lost or online API fails");
        } else {
            println!("[NET] Internet OK but no NVIDIA_API_KEY set — staying offline");
            println!("[OFFLINE] Set NVIDIA_API_KEY in .env and restart for online mode");
        }
    } else {
        println!("[NET] No internet — staying offline with local models");
        println!("[OFFLINE] Models: Whisper STT + SBERT NLU + Piper TTS");
    }

    match init_shared_memory().await {
        Ok(_) => { println!("[OK] Shared memory thread pools initialized"); }
        Err(e) => { eprintln!("[FAIL] Failed to initialize shared memory: {}", e); }
    }

    let pkg_dir = PathBuf::from("./pkg");

    println!("\n[LIST] STEP 1: PERMISSIONS");
    println!("─────────────────────────────────────────────────────\n");

    let mut permissions = match setup_manager::permissions::PermissionsConfig::load() {
        Ok(perms) => perms,
        Err(_) => setup_manager::permissions::PermissionsConfig::default_config(),
    };

    let pending_count = permissions.get_pending().len();
    if pending_count > 0 {
        println!("[LOCK] PERMISSIONS REQUIRED\n");
        println!("The following modules need your permission:\n");
        let pending_modules: Vec<(String, String, String, f32, bool)> = permissions.modules
            .iter()
            .filter(|(_, module)| module.status == setup_manager::permissions::PermissionStatus::Pending)
            .map(|(id, module)| (
                id.clone(),
                module.name.clone(),
                module.description.clone(),
                module.download_size_mb,
                module.required,
            ))
            .collect();

        for (_module_id, name, description, size, required) in &pending_modules {
            println!("  [PKG] {}", name);
            println!("     {}", description);
            println!("     Size: {:.0} MB", size);
            if *required { println!("     [REQUIRED]"); }
            println!();
        }

        for (_module_id, name, _, _, required) in pending_modules {
            if required {
                let _ = permissions.grant_permission(&name);
                println!("[OK] Granted: {}", name);
            }
        }
        let _ = permissions.save();
    } else {
        println!("[OK] All permissions already granted\n");
    }

    println!("\n[SETUP] STEP 2: SETUP");
    println!("─────────────────────────────────────────────────────\n");

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let setup_manager = SetupManager::new(pkg_dir.clone(), event_tx.clone());
    let mut setup_ui = SetupUI::new(event_rx);

    let setup_handle = tokio::spawn(async move {
        match setup_manager.run_setup().await {
            Ok(_) => {
                let mut state = ASSISTANT_STATE.lock().unwrap();
                state.setup_in_progress = false;
                state.current_status = "Setup Complete - Initializing...".to_string();
                state.logs.push(("Setup completed successfully".to_string(), LogLevel::Success));
            }
            Err(e) => {
                let mut state = ASSISTANT_STATE.lock().unwrap();
                state.setup_in_progress = false;
                state.current_status = "Setup Failed".to_string();
                state.logs.push((format!("Setup error: {}", e), LogLevel::Error));
            }
        }
    });

    setup_ui.run().await;
    let _ = setup_handle.await;

    println!("\n[ECO] Initializing clipboard sync...");
    let pkg_dir = PathBuf::from("./pkg");
    match eco::init_eco_manager_async(&pkg_dir).await {
        Ok(_) => {
            let config_path = pkg_dir.join("ecosystem/ecosystem_config.json");
            if let Some(mut guard) = eco::get_eco_manager() {
                if let Some(ref mut manager) = *guard {
                    manager.enable_clipboard_sync();
                    manager.config_mut().enabled = true;
                    manager.config_mut().save(&config_path);
                }
            }
            match eco::start_eco_manager_async().await {
                Ok(_) => println!("[ECO] Clipboard sync started on ports 53327/53328"),
                Err(e) => eprintln!("[ECO] Start failed: {}", e),
            }
        }
        Err(e) => eprintln!("[ECO] Init failed: {}", e),
    }

    println!("\n[MIC] STEP 4: VOICE ASSISTANT");
    println!("─────────────────────────────────────────────────────\n");

    start_voice_assistant().await;
}

#[component]
fn App() -> Element {
    let mut update_trigger = use_signal(|| 0);
    let mut is_awake = use_signal(|| false);
    let mut setup_in_progress = use_signal(|| true);
    let mut show_setup_gui = use_signal(|| true);
    let show_settings = use_signal(|| false);
    let mut show_fastswap = use_signal(|| false);
    let mut status_text = use_signal(|| "Running First-Time Setup...".to_string());
    let mut last_command_text = use_signal(|| "Waiting for voice input...".to_string());
    let mut assistant_name = use_signal(|| CONFIG.assistant_name());
    let mut is_igris = use_signal(|| CONFIG.get().personality == config::Personality::Igris);
    let mut apps_list = use_signal(|| Vec::new());

    let mut show_search_results = use_signal(|| false);
    let mut search_results = use_signal(|| Vec::<SearchResultItem>::new());
    let mut search_query = use_signal(|| String::new());
    let mut is_searching = use_signal(|| false);

    let mut show_camera_panel = use_signal(|| false);

    let mut pending_transfers = use_signal(|| Vec::<fastswap::PendingTransfer>::new());

    let mut current_tab = use_signal(|| Tab::Dashboard);

    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(200)).await;
                update_trigger.set(update_trigger() + 1);

                refresh_running_apps();

                let state = ASSISTANT_STATE.lock().unwrap();
                is_awake.set(state.is_awake);
                setup_in_progress.set(state.setup_in_progress);
                status_text.set(state.current_status.clone());

                if is_setup_complete() && !state.setup_in_progress {
                    show_setup_gui.set(false);
                }

                if !state.last_command.is_empty() {
                    last_command_text.set(state.last_command.clone());
                }

                apps_list.set(state.running_apps.clone());

                assistant_name.set(CONFIG.assistant_name());
                is_igris.set(CONFIG.get().personality == config::Personality::Igris);

                let pending = fastswap::get_pending_transfers().await;
                pending_transfers.set(pending);

                let search_state = SEARCH_STATE.lock().unwrap();
                show_search_results.set(search_state.is_open);
                is_searching.set(search_state.is_searching);
                search_query.set(search_state.query.clone());

                let items: Vec<SearchResultItem> = search_state.results.iter().map(|r| {
                    SearchResultItem {
                        path: r.path.clone(),
                        name: r.name.clone(),
                        drive: r.drive.clone(),
                        score: r.score,
                        is_folder: r.is_folder,
                    }
                }).collect();
                search_results.set(items);
                drop(search_state);

                if let Ok(camera_state) = CAMERA_PANEL_STATE.lock() {
                    let is_open = camera_state.is_open;
                    if is_open != show_camera_panel() {
                        println!("[UI] Camera panel state changed: {}", is_open);
                    }
                    show_camera_panel.set(is_open);
                }

                if let Ok(mut ui_state) = UI_PANEL_STATE.lock() {
                    if ui_state.show_fastswap && !show_fastswap() {
                        show_fastswap.set(true);
                        current_tab.set(Tab::FastSwap);
                        ui_state.show_fastswap = false;
                        spawn(async { fastswap::start_on_demand().await });
                    }
                }
            }
        });
    });

    let _ = update_trigger();
    let awake = is_awake();
    let setup_progress = setup_in_progress();
    let status = status_text();
    let command = last_command_text();
    let apps = apps_list();
    let show_setup = show_setup_gui();
    let name = assistant_name();
    let igris_mode = is_igris();

    let (primary_color, secondary_color, glow_color, accent_rgb) = if awake {
        if igris_mode {
            ("#a855f7", "#7c3aed", "rgba(168, 85, 247, 0.8)", "168, 85, 247")
        } else {
            ("#e879f9", "#f0abfc", "rgba(232, 121, 249, 0.8)", "232, 121, 249")
        }
    } else {
        ("#06b6d4", "#3b82f6", "rgba(34, 211, 238, 0.8)", "34, 211, 238")
    };

    let orb_size = "clamp(150px, 32vmin, 280px)";
    let orbit_dots: Vec<(f64, i32, String, f64, f64, String)> = (0..8).map(|i| {
        let angle = i as f64 * 45.0;
        let sz = if i % 2 == 0 { 9 } else { 5 };
        let col = if i % 2 == 0 { primary_color.to_string() } else { secondary_color.to_string() };
        let rad = if i % 2 == 0 { 85.0 } else { 58.0 };
        let spd = if i % 2 == 0 { 7.0 } else { 5.5 };
        let dly = -(angle / 360.0 * spd);
        let dr = if i % 2 == 0 { "normal".to_string() } else { "reverse".to_string() };
        (dly, sz, col, rad, spd, dr)
    }).collect();

    rsx! {
        style { r#"
            * {{ margin: 0; padding: 0; box-sizing: border-box; }}
            ::-webkit-scrollbar {{ width: 0px; height: 0px; }}
            * {{ scrollbar-width: none; }}
            * {{ -ms-overflow-style: none; }}
            .color-transition {{ transition: all 0.5s ease-in-out; }}

            @keyframes spin {{ from {{ transform: rotate(0deg); }} to {{ transform: rotate(360deg); }} }}
            @keyframes pulse {{ 0%, 100% {{ opacity: 0.4; transform: scale(1); }} 50% {{ opacity: 0.8; transform: scale(1.05); }} }}
            @keyframes pulse-intense {{ 0%, 100% {{ box-shadow: 0 0 20px rgba(168, 85, 247, 0.8), 0 0 40px rgba(168, 85, 247, 0.4); transform: scale(1); }} 50% {{ box-shadow: 0 0 30px rgba(168, 85, 247, 1), 0 0 60px rgba(168, 85, 247, 0.6); transform: scale(1.1); }} }}
            @keyframes wave {{ 0%, 100% {{ height: 0.5rem; opacity: 0.7; }} 50% {{ height: 3rem; opacity: 1; }} }}
            @keyframes scanline {{
                0% {{ transform: translateY(-100%); }}
                100% {{ transform: translateY(100vh); }}
            }}
            @keyframes grid-scroll {{
                0% {{ transform: translateY(0); }}
                100% {{ transform: translateY(32px); }}
            }}
            @keyframes hud-flicker {{
                0%, 100% {{ opacity: 1; }}
                50% {{ opacity: 0.97; }}
            }}
            @keyframes bracket-pulse {{
                0%, 100% {{ opacity: 0.5; }}
                50% {{ opacity: 1; }}
            }}
            @keyframes data-stream {{
                0% {{ transform: translateX(-100%); }}
                100% {{ transform: translateX(200%); }}
            }}
            @keyframes glow-breath {{
                0%, 100% {{ filter: blur(8px); opacity: 0.3; }}
                50% {{ filter: blur(12px); opacity: 0.5; }}
            }}
            @keyframes spin-reverse {{
                from {{ transform: rotate(360deg); }}
                to {{ transform: rotate(0deg); }}
            }}
            @keyframes float-3d {{
                0%, 100% {{ transform: rotateX(8deg) rotateY(12deg); }}
                50% {{ transform: rotateX(5deg) rotateY(8deg); }}
            }}
            @keyframes pulse-text {{
                0%, 100% {{ transform: scale(1); opacity: 0.85; }}
                50% {{ transform: scale(1.03); opacity: 1; }}
            }}
            @keyframes scan {{
                0% {{ left: -100%; }}
                100% {{ left: 200%; }}
            }}
            @keyframes blink-1 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
            @keyframes blink-2 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
            @keyframes blink-3 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
        "# }

        SettingsPanel { is_open: show_settings }

        if show_fastswap() && current_tab() != Tab::FastSwap {
            div {
                style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 100; background: rgba(0,0,0,0.5); backdrop-filter: blur(4px);",
                onclick: move |_| show_fastswap.set(false),
                div {
                    style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); width: 90%; max-width: 800px; max-height: 90vh; overflow: auto; border-radius: 20px; box-shadow: 0 20px 60px rgba(0,0,0,0.5);",
                    onclick: move |e| e.stop_propagation(),
                    FastSwapPanel {}
                }
            }
        }

        SearchResultsPanel {
            is_open: show_search_results,
            results: search_results,
            search_query,
            is_searching,
        }

        if show_camera_panel() {
            CameraPanel {
                on_close: move |_| {
                    if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                        state.is_open = false;
                    }
                }
            }
        }

        PresentationPanel {}

        IncomingTransferPopup { pending_transfers }

        // Main layout with sidebar
        div { style: "width: 100vw; height: 100vh; display: flex; background: #050a14; color: #fff; font-family: 'Inter', sans-serif; position: relative; overflow: hidden;",

            if show_setup && setup_progress {
                div { style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 100; background: #050a14;",
                    SetupGui {}
                }
            }

            if !show_setup || !setup_progress {
                // Sidebar
                Sidebar {
                    active_tab: current_tab,
                    is_awake: awake,
                    primary_color: primary_color,
                    accent_rgb: accent_rgb,
                }

                // Main content area
                div { style: "flex: 1; display: flex; flex-direction: column; position: relative; overflow: hidden; background: #050a14;",

                    // Grid overlay
                    div { style: "position: absolute; inset: 0; pointer-events: none; z-index: 0; opacity: 0.03;",
                        div { style: "width: 100%; height: 100%; background-image: linear-gradient(rgba(255,255,255,0.3) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.3) 1px, transparent 1px); background-size: 32px 32px; animation: grid-scroll 4s linear infinite;" }
                    }

                    // Scan line overlay
                    div { style: "position: absolute; inset: 0; pointer-events: none; z-index: 1; overflow: hidden; opacity: 0.04;",
                        div { style: "width: 100%; height: 2px; background: linear-gradient(90deg, transparent, rgba(255,255,255,0.5), transparent); animation: scanline 4s linear infinite;" }
                    }

                    // Accent gradient glow in top-right
                    div { style: format!("position: absolute; top: -200px; right: -200px; width: 500px; height: 500px; border-radius: 50%; background: radial-gradient(circle, rgba({}, 0.06), transparent 70%); pointer-events: none; z-index: 0; transition: all 0.5s;", accent_rgb) }
                    div { style: format!("position: absolute; bottom: -150px; left: -150px; width: 400px; height: 400px; border-radius: 50%; background: radial-gradient(circle, rgba({}, 0.04), transparent 70%); pointer-events: none; z-index: 0; transition: all 0.5s;", accent_rgb) }

                    // Top bar
                    div { style: format!("position: relative; z-index: 10; display: flex; align-items: center; justify-content: space-between; padding: 14px 24px; border-bottom: 1px solid rgba({}, 0.08); flex-shrink: 0; transition: border-color 0.5s; background: rgba(5,10,20,0.8); backdrop-filter: blur(8px);", accent_rgb),
                        // Corner brackets on top bar
                        div { style: format!("position: absolute; bottom: -1px; left: 0; width: 20px; height: 8px; border-left: 1px solid rgba({}, 0.2); border-bottom: 1px solid rgba({}, 0.2); transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                        div { style: format!("position: absolute; bottom: -1px; right: 0; width: 20px; height: 8px; border-right: 1px solid rgba({}, 0.2); border-bottom: 1px solid rgba({}, 0.2); transition: border-color 0.5s;", accent_rgb, accent_rgb) }

                        div { style: "display: flex; align-items: center; gap: 14px;",
                            div { style: format!("font-size: 11px; font-weight: 700; letter-spacing: 2px; font-family: 'JetBrains Mono', monospace; color: {}; text-shadow: 0 0 20px {}; transition: all 0.5s;", primary_color, glow_color),
                                "// {name}"
                            }
                            span { style: "font-size: 9px; color: rgba(255,255,255,0.12); font-family: monospace; letter-spacing: 0.5px;", "v1.0.0_eco" }
                            div {
                                style: format!(
                                    "padding: 2px 10px; border-radius: 4px; font-size: 9px; letter-spacing: 1px; font-weight: 600; font-family: monospace; \
                                     background: {}; border: 1px solid {}; color: {}; transition: all 0.5s;",
                                    if awake { format!("rgba({}, 0.1)", accent_rgb) } else { "rgba(34,197,94,0.08)".to_string() },
                                    if awake { format!("rgba({}, 0.3)", accent_rgb) } else { "rgba(34,197,94,0.2)".to_string() },
                                    if awake { &primary_color } else { "#22c55e" },
                                ),
                                if awake { ">> AWAKE" } else { ">> STANDBY" }
                            }
                        }
                        MenuButton {
                            settings_open: show_settings,
                            fastswap_open: show_fastswap,
                        }
                    }

                    // Content area based on active tab
                    div { style: "flex: 1; overflow: hidden; display: flex; position: relative; z-index: 5;",
                        match current_tab() {
                            Tab::Dashboard => rsx! {
                                div { style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: clamp(20px, 4vh, 40px); padding: clamp(10px, 2vw, 20px); position: relative;",
                                    // Top corner brackets (decorative)
                                    div { style: format!("position: absolute; top: 12px; left: 12px; width: 24px; height: 24px; border-left: 1px solid rgba({}, 0.15); border-top: 1px solid rgba({}, 0.15); transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                    div { style: format!("position: absolute; top: 12px; right: 12px; width: 24px; height: 24px; border-right: 1px solid rgba({}, 0.15); border-top: 1px solid rgba({}, 0.15); transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                    div { style: format!("position: absolute; bottom: 12px; left: 12px; width: 24px; height: 24px; border-left: 1px solid rgba({}, 0.15); border-bottom: 1px solid rgba({}, 0.15); transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                    div { style: format!("position: absolute; bottom: 12px; right: 12px; width: 24px; height: 24px; border-right: 1px solid rgba({}, 0.15); border-bottom: 1px solid rgba({}, 0.15); transition: border-color 0.5s;", accent_rgb, accent_rgb) }

                                    // 3D Holographic Sphere with orbiting gradient dots + AI name
                                    div { style: format!("position: relative; width: {orb_size}; height: {orb_size}; display: flex; align-items: center; justify-content: center; perspective: 800px;"),
                                        // 3D sphere body with lighting gradient
                                        div { style: format!(
                                            "position: absolute; inset: 0; border-radius: 50%; \
                                             background: radial-gradient(circle at 30% 28%, rgba(255,255,255,0.13) 0%, {} 25%, {} 60%, rgba(0,0,0,0.5) 100%); \
                                             box-shadow: 0 0 60px {}, 0 0 120px rgba({}, 0.15), inset 0 -30px 50px rgba(0,0,0,0.4); \
                                             transition: all 0.5s; animation: float-3d 5s ease-in-out infinite; transform: rotateX(8deg) rotateY(12deg);",
                                            primary_color, secondary_color, glow_color, accent_rgb
                                        )}
                                        // Tilted orbital ring for 3D depth
                                        div { style: format!(
                                            "position: absolute; inset: -6%; border-radius: 50%; \
                                             border: 1px solid rgba({}, 0.1); transition: border-color 0.5s; \
                                             transform: rotateX(65deg) rotateZ(0deg); animation: spin 14s linear infinite;",
                                            accent_rgb
                                        )}
                                        // Inner orbital ring (counter-rotating)
                                        div { style: format!(
                                            "position: absolute; inset: 10%; border-radius: 50%; \
                                             border: 1px solid rgba({}, 0.06); transition: border-color 0.5s; \
                                             transform: rotateX(-45deg) rotateZ(0deg); animation: spin 18s linear infinite reverse;",
                                            accent_rgb
                                        )}
                                        // Orbiting gradient dots
                                        for (delay, size, color, radius, speed, dir) in orbit_dots.clone().into_iter() {
                                            div { style: format!(
                                                "position: absolute; top: 50%; left: 50%; width: 0; height: 0; \
                                                 animation: spin {}s linear infinite; \
                                                 animation-direction: {}; \
                                                 animation-delay: {}s;",
                                                speed, dir, delay
                                            ),
                                                div { style: format!(
                                                    "width: {}px; height: {}px; background: linear-gradient(135deg, {}, rgba(255,255,255,0.5)); \
                                                     border-radius: 50%; transform: translateX({}px); \
                                                     box-shadow: 0 0 8px {};",
                                                    size, size, color, radius, color
                                                )}
                                            }
                                        }
                                        // Glow aura
                                        div { style: format!(
                                            "position: absolute; width: 70%; height: 70%; \
                                             background: radial-gradient(circle, {} 0%, rgba({}, 0.25) 40%, transparent 70%); \
                                             border-radius: 50%; filter: blur(25px); opacity: 0.35; \
                                             animation: glow-breath 3s ease-in-out infinite; transition: all 0.5s;",
                                            primary_color, accent_rgb
                                        )}
                                        // AI name in center
                                        div { style: format!(
                                            "position: relative; z-index: 10; text-align: center; \
                                             font-family: 'JetBrains Mono', monospace; font-weight: 900; \
                                             font-size: clamp(26px, 5.5vmin, 44px); color: #ffffff; \
                                             text-shadow: 0 0 10px rgba(0,0,0,0.8), 0 0 30px {}, 0 0 60px rgba({}, 0.6), 0 0 100px rgba({}, 0.3); \
                                             letter-spacing: 4px; animation: pulse-text 2.5s ease-in-out infinite;",
                                            glow_color, accent_rgb, accent_rgb
                                        ),
                                            "{name}"
                                        }
                                    }

                                    // Audio wave bars
                                    div { style: "display: flex; align-items: center; justify-content: center; gap: clamp(2px, 0.5vw, 4px); height: clamp(20px, 5vh, 40px); opacity: 0.6;",
                                        for i in 0..5 {
                                            div { style: format!("height: {}%; width: clamp(2px, 0.4vw, 3px); background: {}; border-radius: 1px; animation: wave 0.8s ease-in-out infinite; animation-delay: {}s; transition: background 0.5s;",
                                                [17, 50, 83, 67, 100][i], primary_color, i as f64 * 0.1,
                                            )}
                                        }
                                    }

                                    // Last command box (HUD style)
                                    div { style: format!("width: min(88%, 580px); position: relative; padding: clamp(10px, 2vh, 14px) clamp(14px, 2vw, 22px); background: rgba(5,10,20,0.85); border: 1px solid rgba({}, 0.12); animation: hud-flicker 4s ease-in-out infinite;", accent_rgb),
                                        // Corner brackets on command box
                                        div { style: format!("position: absolute; top: -1px; left: -1px; width: 10px; height: 10px; border-left: 1px solid rgba({}, 0.25); border-top: 1px solid rgba({}, 0.25); animation: bracket-pulse 2s ease-in-out infinite; transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                        div { style: format!("position: absolute; top: -1px; right: -1px; width: 10px; height: 10px; border-right: 1px solid rgba({}, 0.25); border-top: 1px solid rgba({}, 0.25); animation: bracket-pulse 2s ease-in-out infinite 0.5s; transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                        div { style: format!("position: absolute; bottom: -1px; left: -1px; width: 10px; height: 10px; border-left: 1px solid rgba({}, 0.25); border-bottom: 1px solid rgba({}, 0.25); animation: bracket-pulse 2s ease-in-out infinite 1s; transition: border-color 0.5s;", accent_rgb, accent_rgb) }
                                        div { style: format!("position: absolute; bottom: -1px; right: -1px; width: 10px; height: 10px; border-right: 1px solid rgba({}, 0.25); border-bottom: 1px solid rgba({}, 0.25); animation: bracket-pulse 2s ease-in-out infinite 1.5s; transition: border-color 0.5s;", accent_rgb, accent_rgb) }

                                        div { style: "font-size: clamp(9px, 1.2vw, 10px); letter-spacing: 2px; color: rgba(255,255,255,0.25); margin-bottom: clamp(4px, 1vh, 6px); text-transform: uppercase; font-family: monospace;",
                                            "// LAST_INPUT"
                                        }
                                        div { style: format!("font-size: clamp(12px, 1.8vw, 16px); color: {}; font-family: 'JetBrains Mono', monospace; min-height: clamp(14px, 2.5vh, 20px); transition: color 0.5s; word-break: break-word; letter-spacing: 0.5px;", primary_color),
                                            "> {command}"
                                        }
                                    }

                                    // Bottom HUD row: status + apps
                                    div { style: "display: flex; align-items: stretch; gap: 12px; width: min(88%, 580px);",
                                        // Status panel
                                        div { style: "flex: 1; position: relative; padding: 14px; border-radius: 4px; background: rgba(5,10,20,0.8); border: 1px solid rgba(255,255,255,0.04); display: flex; flex-direction: column; align-items: center; gap: 6px;",
                                            div { style: format!("font-size: 9px; letter-spacing: 2px; color: rgba(255,255,255,0.25); text-transform: uppercase; font-family: monospace; transition: color 0.5s;"),
                                                ">> {status}"
                                            }
                                            div { style: "display: flex; gap: 4px;",
                                                for i in 0..3 {
                                                    div { style: format!("width: 8px; height: 8px; border: 1px solid rgba({}, 0.3); background: {}; border-radius: 50%; animation: blink-{} 1.2s ease-in-out infinite; transition: background 0.5s;", accent_rgb, primary_color, (i + 1).to_string()) }
                                                }
                                            }
                                            div { style: "font-size: 9px; color: rgba(255,255,255,0.12); font-family: monospace; margin-top: 2px;",
                                                if awake { "AUDIO_INPUT :: ACTIVE" } else { "STANDBY :: WAITING" }
                                            }
                                        }
                                        // Apps panel
                                        div { style: "flex: 2; position: relative; padding: 14px; border-radius: 4px; background: rgba(5,10,20,0.8); border: 1px solid rgba(255,255,255,0.04);",
                                            div { style: "font-size: 9px; letter-spacing: 2px; color: rgba(255,255,255,0.2); margin-bottom: 8px; font-family: monospace;",
                                                "// RUNNING_PROCESSES"
                                            }
                                            if apps.is_empty() {
                                                div { style: "font-size: 10px; color: rgba(255,255,255,0.12); font-family: monospace;", "<NONE>" }
                                            } else {
                                                for (idx, app) in apps.iter().enumerate() {
                                                    div { style: format!("font-size: 10px; color: rgba(255,255,255,0.35); display: flex; align-items: center; gap: 6px; padding: 2px 0; font-family: 'JetBrains Mono', monospace; transition: color 0.5s;"),
                                                        span { style: format!("width: 4px; height: 4px; background: {}; flex-shrink: 0; border-radius: 50%; transition: background 0.5s;", secondary_color) }
                                                        "{app}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Tab::Chat => rsx! {
                                ChatPanel {
                                    primary_color,
                                    accent_rgb,
                                }
                            },
                            Tab::FastSwap => rsx! {
                                div { style: "flex: 1; overflow-y: auto; padding: 24px;",
                                    FastSwapPanel {}
                                }
                            },
                            Tab::Devices => rsx! {
                                EcoDevicePanel {
                                    primary_color,
                                    accent_rgb,
                                }
                            },
                            Tab::Alarms | Tab::Reminders => rsx! {
                                AlarmReminderPanel {
                                    primary_color,
                                    accent_rgb,
                                }
                            },
                            Tab::SystemInfo => rsx! {
                                SystemInfoPanel {
                                    primary_color,
                                    accent_rgb,
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}
