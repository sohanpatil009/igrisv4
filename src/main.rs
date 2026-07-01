// src/main.rs - IGRIS Voice Assistant
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

// Import from library
use igrisv3::{
    config, utils, setup_manager, fastswap, online,
    SEARCH_STATE, RESET_FLAG,
};
use igrisv3::config::CONFIG;
use igrisv3::setup_manager::gui::{is_setup_complete, SetupGui};
use igrisv3::setup_manager::{SetupManager, SetupUI};
use igrisv3::utils::shared_memory::init_shared_memory;
use igrisv3::ui::{SettingsPanel, MenuButton, SearchResultsPanel, SearchResultItem, CameraPanel, PresentationPanel, FastSwapPanel, IncomingTransferPopup};
use igrisv3::commands::ffmpeg_camera::{CameraPanelState, CAMERA_PANEL_STATE};

use crate::state::*;
use crate::tools::*;
use crate::voice::*;

fn main() {
    // Register global hotkey (Ctrl+Shift+Space) - resets voice loop
    if let Err(e) = utils::hotkey::register_global_hotkey(|| {
        println!("[HOTKEY] Ctrl+Shift+Space pressed - Resetting IGRIS");

        // Signal all loops to reset back to wake word detection
        RESET_FLAG.store(true, Ordering::Relaxed);

        // Speak the invoke greeting
        if let Err(e) = utils::greetings::speak_invoke_greeting() {
            eprintln!("[HOTKEY] Failed to speak greeting: {}", e);
        }
    }) {
        eprintln!("[HOTKEY] Failed to register global hotkey: {}", e);
        eprintln!("[HOTKEY] You can still use the application window");
    }

    // Run setup on a separate thread
    thread::spawn(|| {
        start_setup_and_assistant();
    });

    let window = WindowBuilder::new()
        .with_title("IGRIS Voice Assistant")
        .with_visible(true)
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(800.0, 600.0))
        .with_window_icon(Some(load_icon()));

    let cfg = Config::new()
        .with_window(window)
        .with_menu(None) // Remove default menu bar (File, Edit, Window, Help)
        .with_disable_context_menu(true); // Disable right-click menu

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
    // Create a dedicated thread with its own Tokio runtime
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

    // Load .env file for API keys (online mode)
    let _ = dotenv::dotenv();

    // First, check internet connectivity before deciding which mode to use
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

    // Initialize shared memory thread pools for faster response
    match init_shared_memory().await {
        Ok(_) => {
            println!("[OK] Shared memory thread pools initialized");
        }
        Err(e) => {
            eprintln!("[FAIL] Failed to initialize shared memory: {}", e);
        }
    }

        let pkg_dir = PathBuf::from("./pkg");

        // ═══════════════════════════════════════════════════════
        // STEP 1: PERMISSIONS
        // ═══════════════════════════════════════════════════════
        println!("\n[LIST] STEP 1: PERMISSIONS");
        println!("─────────────────────────────────────────────────────\n");

        let mut permissions = match setup_manager::permissions::PermissionsConfig::load() {
            Ok(perms) => perms,
            Err(_) => setup_manager::permissions::PermissionsConfig::default_config(),
        };

        // Check if there are pending permissions
        let pending_count = permissions.get_pending().len();
        if pending_count > 0 {
            println!("[LOCK] PERMISSIONS REQUIRED\n");
            println!("The following modules need your permission:\n");

            // Collect pending modules
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
                if *required {
                    println!("     [REQUIRED]");
                }
                println!();
            }

            // Auto-grant all required modules
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

        // ═══════════════════════════════════════════════════════
        // STEP 2: SETUP (Download, Extract, Validate)
        // ═══════════════════════════════════════════════════════
        println!("\n[SETUP] STEP 2: SETUP");
        println!("─────────────────────────────────────────────────────\n");

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create setup manager
        let setup_manager = SetupManager::new(pkg_dir.clone(), event_tx.clone());

        // Create UI for setup
        let mut setup_ui = SetupUI::new(event_rx);

        // Run setup in background
        let setup_handle = tokio::spawn(async move {
            match setup_manager.run_setup().await {
                Ok(_) => {
                    let mut state = ASSISTANT_STATE.lock().unwrap();
                    state.setup_in_progress = false;
                    state.current_status = "Setup Complete - Initializing...".to_string();
                    state.logs.push((
                        "Setup completed successfully".to_string(),
                        LogLevel::Success,
                    ));

                }
                Err(e) => {
                    let mut state = ASSISTANT_STATE.lock().unwrap();
                    state.setup_in_progress = false;
                    state.current_status = "Setup Failed".to_string();
                    state
                        .logs
                        .push((format!("Setup error: {}", e), LogLevel::Error));
                }
            }
        });

        // Display setup progress
        setup_ui.run().await;

        // Wait for setup to complete
        let _ = setup_handle.await;

        // ═══════════════════════════════════════════════════════
        // STEP 3: ECOSYSTEM (clipboard sync)
        // ═══════════════════════════════════════════════════════
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

        // ═══════════════════════════════════════════════════════
        // STEP 4: INITIALIZE VOICE ASSISTANT
        // ═══════════════════════════════════════════════════════
        println!("\n[MIC] STEP 4: VOICE ASSISTANT");
        println!("─────────────────────────────────────────────────────\n");

        // Now start the voice assistant
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

    // Search results state
    let mut show_search_results = use_signal(|| false);
    let mut search_results = use_signal(|| Vec::<SearchResultItem>::new());
    let mut search_query = use_signal(|| String::new());
    let mut is_searching = use_signal(|| false);

    // Camera panel state
    let mut show_camera_panel = use_signal(|| false);

    // Incoming transfer state (for popup)
    let mut pending_transfers = use_signal(|| Vec::<fastswap::PendingTransfer>::new());

    // Note: File sharing is handled by FastSwap (integrated Rust implementation)

    use_effect(move || {
        spawn(async move {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(200)).await;
                update_trigger.set(update_trigger() + 1);

                // Refresh running apps from tracker (cleanup dead processes)
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

                // Update assistant name and personality from config
                assistant_name.set(CONFIG.assistant_name());
                is_igris.set(CONFIG.get().personality == config::Personality::Igris);

                // Update pending transfers (for incoming transfer popup)
                let pending = fastswap::get_pending_transfers().await;
                pending_transfers.set(pending);

                // Update search state from global
                let search_state = SEARCH_STATE.lock().unwrap();
                show_search_results.set(search_state.is_open);
                is_searching.set(search_state.is_searching);
                search_query.set(search_state.query.clone());

                // Convert search results
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

                // Update camera panel state from global
                if let Ok(camera_state) = CAMERA_PANEL_STATE.lock() {
                    let is_open = camera_state.is_open;
                    if is_open != show_camera_panel() {
                        println!("[UI] Camera panel state changed: {}", is_open);
                    }
                    show_camera_panel.set(is_open);
                }

                // Update FastSwap panel state from global (voice commands)
                if let Ok(mut ui_state) = UI_PANEL_STATE.lock() {
                    if ui_state.show_fastswap && !show_fastswap() {
                        show_fastswap.set(true);
                        ui_state.show_fastswap = false;
                        // Start server on first show
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

    // Color scheme based on personality and awake state
    // IGRIS: cyan (standby) -> purple (awake)
    // Alita: cyan (standby) -> lavender/pink (awake)
    let (primary_color, secondary_color, glow_color, accent_rgb) = if awake {
        if igris_mode {
            // IGRIS awake: Purple theme
            ("#a855f7", "#7c3aed", "rgba(168, 85, 247, 0.8)", "168, 85, 247")
        } else {
            // Alita awake: Lavender/Pink theme
            ("#e879f9", "#f0abfc", "rgba(232, 121, 249, 0.8)", "232, 121, 249")
        }
    } else {
        // Standby: Cyan/Blue theme (default)
        ("#06b6d4", "#3b82f6", "rgba(34, 211, 238, 0.8)", "34, 211, 238")
    };

    rsx! {
        style {
            r#"
            * {{
                margin: 0;
                padding: 0;
                box-sizing: border-box;
            }}

            /* Hide scrollbars but keep functionality - Webkit browsers */
            ::-webkit-scrollbar {{
                width: 0px;
                height: 0px;
            }}

            /* Hide scrollbars but keep functionality - Firefox */
            * {{
                scrollbar-width: none;
            }}

            /* Hide scrollbars but keep functionality - IE and Edge */
            * {{
                -ms-overflow-style: none;
            }}

            /* Smooth color transitions */
            .color-transition {{
                transition: all 0.5s ease-in-out;
            }}
            "#
        }

        // Settings Panel (modal)
        SettingsPanel { is_open: show_settings }

        // FastSwap Panel (modal)
        if show_fastswap() {
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

        // Search Results Panel (modal)
        SearchResultsPanel {
            is_open: show_search_results,
            results: search_results,
            search_query,
            is_searching,
        }

        // Camera Panel (modal)
        if show_camera_panel() {
            CameraPanel {
                on_close: move |_| {
                    // Close camera panel
                    if let Ok(mut state) = CAMERA_PANEL_STATE.lock() {
                        state.is_open = false;
                    }
                }
            }
        }


        // Presentation Panel (full screen overlay with TTS narration)
        PresentationPanel {}

        // Incoming Transfer Popup (highest z-index, appears on top of everything)
        IncomingTransferPopup { pending_transfers }

        div { style: "width: 100vw; height: 100vh; display: flex; flex-direction: column; background: #000; color: #fff; font-family: 'Inter', sans-serif; position: relative; overflow: hidden;",

            if show_setup && setup_progress {
                div { style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 100; background: #000;",
                    SetupGui {}
                }
            }

            if !show_setup || !setup_progress {
                // Top Left - Brand (dynamic name from config with color transition)
                div { style: "position: fixed; top: clamp(12px, 3vh, 24px); left: clamp(12px, 3vw, 24px); z-index: 50;",
                    h1 {
                        style: format!(
                            "font-size: clamp(18px, 4vw, 30px); font-weight: bold; letter-spacing: 2px; text-shadow: 0 0 20px {}; transition: all 0.5s ease-in-out;",
                            glow_color,
                        ),
                        span { style: format!("color: {}; transition: color 0.5s ease-in-out;", primary_color),
                            "{name}"
                        }
                        span { style: "color: #6b7280; font-size: clamp(8px, 1.5vw, 12px); margin-left: 8px;",
                            "v1.0"
                        }
                    }
                }

                MenuButton {
                    settings_open: show_settings,
                    fastswap_open: show_fastswap
                }

                // Central Animated Panel
                div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center; gap: clamp(24px, 5vh, 48px); width: 100%; height: 100%; padding: clamp(10px, 2vw, 20px);",

                    // Animated Orb with dynamic colors - responsive sizing
                    div { style: "position: relative; width: clamp(120px, 30vmin, 256px); height: clamp(120px, 30vmin, 256px); display: flex; align-items: center; justify-content: center;",
                        div {
                            style: format!(
                                "position: absolute; width: 100%; height: 100%; border: 2px solid {}; border-radius: 50%; opacity: 0.3; animation: spin 20s linear infinite; transition: border-color 0.5s ease-in-out;",
                                primary_color,
                            ),
                        }
                        div {
                            style: format!(
                                "position: absolute; width: 75%; height: 75%; border: 2px solid {}; border-radius: 50%; opacity: 0.4; animation: pulse 2s ease-in-out infinite; transition: border-color 0.5s ease-in-out;",
                                secondary_color,
                            ),
                        }
                        div {
                            style: format!(
                                "position: absolute; width: 62.5%; height: 62.5%; border: 2px solid {}; border-radius: 50%; opacity: 0.5; transition: border-color 0.5s ease-in-out;",
                                primary_color,
                            ),
                        }
                        div {
                            style: format!(
                                "position: absolute; width: 50%; height: 50%; background: linear-gradient(135deg, {}, {}); border-radius: 50%; filter: blur(clamp(24px, 5vmin, 48px)); opacity: 0.6; animation: pulse 3s ease-in-out infinite; transition: background 0.5s ease-in-out;",
                                primary_color,
                                secondary_color,
                            ),
                        }
                        div {
                            style: format!(
                                "position: relative; width: 12.5%; height: 12.5%; min-width: 16px; min-height: 16px; background: {}; border-radius: 50%; box-shadow: 0 0 20px {}, 0 0 40px rgba({}, 0.4); animation: pulse-intense 2s ease-in-out infinite; transition: all 0.5s ease-in-out;",
                                if awake { if igris_mode { "#e9d5ff" } else { "#fce7f3" } } else { "#cffafe" },
                                glow_color,
                                accent_rgb,
                            ),
                        }
                    }

                    // Audio wave bars with dynamic colors - responsive
                        div { style: "display: flex; align-items: center; justify-content: center; gap: clamp(2px, 0.5vw, 4px); height: clamp(24px, 6vh, 48px);",
                            div {
                                style: format!(
                                    "height: 17%; width: clamp(2px, 0.5vw, 4px); background: {}; border-radius: 2px; animation: wave 0.8s ease-in-out infinite; animation-delay: 0s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "height: 50%; width: clamp(2px, 0.5vw, 4px); background: {}; border-radius: 2px; animation: wave 0.8s ease-in-out infinite; animation-delay: 0.1s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "height: 83%; width: clamp(2px, 0.5vw, 4px); background: {}; border-radius: 2px; animation: wave 0.8s ease-in-out infinite; animation-delay: 0.2s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "height: 67%; width: clamp(2px, 0.5vw, 4px); background: {}; border-radius: 2px; animation: wave 0.8s ease-in-out infinite; animation-delay: 0.3s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "height: 100%; width: clamp(2px, 0.5vw, 4px); background: {}; border-radius: 2px; animation: wave 0.8s ease-in-out infinite; animation-delay: 0.4s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                        }

                        // Last command box with dynamic border - responsive
                        div {
                            style: format!(
                                "width: 100%; padding: clamp(10px, 2vh, 16px) clamp(12px, 2vw, 24px); background: linear-gradient(135deg, rgba({}, 0.1), rgba({}, 0.15)); border: 1px solid rgba({}, 0.5); border-radius: 8px; backdrop-filter: blur(10px); transition: all 0.5s ease-in-out;",
                                accent_rgb,
                                accent_rgb,
                                accent_rgb,
                            ),
                            div { style: "font-size: clamp(10px, 1.5vw, 12px); letter-spacing: 1px; color: #9ca3af; margin-bottom: clamp(4px, 1vh, 8px); text-transform: uppercase;",
                                "Last Command"
                            }
                            div {
                                style: format!(
                                    "font-size: clamp(12px, 2vw, 18px); color: {}; font-family: monospace; min-height: clamp(16px, 3vh, 24px); transition: color 0.5s ease-in-out; word-break: break-word;",
                                    if awake { if igris_mode { "#e9d5ff" } else { "#fce7f3" } } else { "#cffafe" },
                                ),
                                "\"{command}\""
                            }
                        }

                    // Status indicator dots - responsive
                    div { style: "display: flex; flex-direction: column; align-items: center; gap: clamp(4px, 1vh, 8px);",
                        div {
                            style: format!(
                                "font-size: clamp(10px, 1.5vw, 12px); letter-spacing: 1px; color: {}; text-transform: uppercase; transition: color 0.5s ease-in-out;",
                                if awake { primary_color } else { "#9ca3af" },
                            ),
                            if awake {
                                "Listening"
                            } else {
                                "Standby"
                            }
                        }
                        div { style: "display: flex; gap: clamp(2px, 0.5vw, 4px);",
                            div {
                                style: format!(
                                    "width: clamp(8px, 1.5vw, 12px); height: clamp(8px, 1.5vw, 12px); background: {}; border-radius: 50%; animation: blink-1 1.2s ease-in-out infinite; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "width: clamp(8px, 1.5vw, 12px); height: clamp(8px, 1.5vw, 12px); background: {}; border-radius: 50%; animation: blink-2 1.2s ease-in-out infinite 0.2s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                            div {
                                style: format!(
                                    "width: clamp(8px, 1.5vw, 12px); height: clamp(8px, 1.5vw, 12px); background: {}; border-radius: 50%; animation: blink-3 1.2s ease-in-out infinite 0.4s; transition: background 0.5s ease-in-out;",
                                    primary_color,
                                ),
                            }
                        }
                    }
                }

                // Apps Panel - Bottom Left with dynamic border - responsive
                div {
                    style: format!(
                        "position: fixed; bottom: clamp(12px, 3vh, 24px); left: clamp(12px, 3vw, 24px); width: clamp(160px, 28vw, 320px); max-height: clamp(100px, 25vh, 192px); background: rgba(0, 0, 0, 0.8); border: 1px solid rgba({}, 0.4); border-radius: 8px; padding: clamp(8px, 2vh, 16px); backdrop-filter: blur(10px); overflow-y: auto; overflow-x: hidden; z-index: 40; transition: border-color 0.5s ease-in-out;",
                        accent_rgb,
                    ),
                    div { style: "font-size: clamp(10px, 1.5vw, 12px); letter-spacing: 1px; color: #9ca3af; margin-bottom: clamp(6px, 1.5vh, 12px); text-transform: uppercase;",
                        "Active Applications"
                    }
                    div { style: "display: flex; flex-direction: column; gap: clamp(2px, 0.5vh, 4px);",
                        if apps.is_empty() {
                            div { style: "font-size: clamp(10px, 1.3vw, 12px); color: #6b7280;",
                                "No applications running"
                            }
                        } else {
                            for app in apps.iter() {
                                div {
                                    style: format!(
                                        "font-size: clamp(10px, 1.3vw, 12px); color: {}; display: flex; align-items: center; gap: clamp(4px, 1vw, 8px); transition: color 0.5s ease-in-out;",
                                        secondary_color,
                                    ),
                                    span {
                                        style: format!(
                                            "width: clamp(6px, 1vw, 8px); height: clamp(6px, 1vw, 8px); background: {}; border-radius: 50%; transition: background 0.5s ease-in-out; flex-shrink: 0;",
                                            secondary_color,
                                        ),
                                    }
                                    "{app}"
                                }
                            }
                        }
                    }
                }
            }

            style {
                r#"
                @keyframes spin {{ from {{ transform: rotate(0deg); }} to {{ transform: rotate(360deg); }} }}
                @keyframes pulse {{ 0%, 100% {{ opacity: 0.4; transform: scale(1); }} 50% {{ opacity: 0.8; transform: scale(1.05); }} }}
                @keyframes pulse-intense {{ 0%, 100% {{ box-shadow: 0 0 20px rgba(168, 85, 247, 0.8), 0 0 40px rgba(168, 85, 247, 0.4); transform: scale(1); }} 50% {{ box-shadow: 0 0 30px rgba(168, 85, 247, 1), 0 0 60px rgba(168, 85, 247, 0.6); transform: scale(1.1); }} }}
                @keyframes wave {{ 0%, 100% {{ height: 0.5rem; opacity: 0.7; }} 50% {{ height: 3rem; opacity: 1; }} }}
                @keyframes fade-in-out {{ 0%, 100% {{ opacity: 1; }} 50% {{ opacity: 0.7; }} }}
                @keyframes blink-1 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
                @keyframes blink-2 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
                @keyframes blink-3 {{ 0%, 100% {{ opacity: 0.3; }} 50% {{ opacity: 1; }} }}
                "#
            }
        }
    }
}
