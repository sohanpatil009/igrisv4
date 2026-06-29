// src/main.rs - IGRIS Voice Assistant
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use dioxus::prelude::*;
use std::path::PathBuf;
use tokio::sync::{mpsc, RwLock};

// Import from library
use igrisv3::{
    config, ui, core, nlu, commands, plugins, utils, platform, platform_utils,
    setup_manager, media, fastswap,
    SearchState, SearchResultData, SEARCH_STATE, RESET_FLAG,
};

use dioxus::desktop::{Config, WindowBuilder};
use core::audio_capture::{capture_audio_vad, CaptureConfig};
use nlu::engine::GLOBAL_NLU;
use setup_manager::gui::{is_setup_complete, SetupGui};
use setup_manager::{SetupManager, SetupUI};
use utils::shared_memory::init_shared_memory;
use commands::app_utils::list_running_apps;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use core::stt::{init_whisper_context, transcribe_audio, hybrid_transcribe_audio};
use core::tts::TTS_ENGINE;
use core::wake_word::listen_for_wake_word;
#[cfg(feature = "candle")]
use core::local_llm::{is_local_llm_ready, global_reason, default_tool_system_prompt, parse_tool_call};
use config::CONFIG;
use ui::{SettingsPanel, MenuButton, SearchResultsPanel, SearchResultItem, CameraPanel, PresentationPanel, FastSwapPanel, IncomingTransferPopup};

// Global state for voice assistant
static ASSISTANT_STATE: once_cell::sync::Lazy<Arc<Mutex<AssistantState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(AssistantState::default())));



// Global UI state for voice-triggered panels
static UI_PANEL_STATE: once_cell::sync::Lazy<Arc<Mutex<UiPanelState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(UiPanelState::default())));

#[derive(Default)]
struct UiPanelState {
    show_fastswap: bool,
}

// Camera panel state - use directly from commands module
use commands::ffmpeg_camera::{CameraPanelState, CAMERA_PANEL_STATE};




#[derive(Clone, Debug)]
struct AssistantState {
    is_initialized: bool,
    is_listening: bool,
    is_awake: bool,
    current_status: String,
    last_command: String,
    logs: Vec<(String, LogLevel)>,
    running_apps: Vec<String>,
    setup_in_progress: bool,
}

impl Default for AssistantState {
    fn default() -> Self {
        Self {
            is_initialized: false,
            is_listening: false,
            is_awake: false,
            current_status: "Running First-Time Setup...".to_string(),
            last_command: String::new(),
            logs: vec![(
                "Welcome to IGRIS - Your Voice Assistant".to_string(),
                LogLevel::Info,
            )],
            running_apps: Vec::new(),
            setup_in_progress: true,
        }
    }
}

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
    println!("[LAUNCH] IGRIS v3 - Offline Voice Assistant");
    println!("═══════════════════════════════════════════════════════\n");

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
        // STEP 3: INITIALIZE VOICE ASSISTANT
        // ═══════════════════════════════════════════════════════
        println!("\n[MIC] STEP 3: VOICE ASSISTANT");
        println!("─────────────────────────────────────────────────────\n");

        // Now start the voice assistant
        start_voice_assistant().await;
}

async fn start_voice_assistant() {
    // Initialize
    update_status("Initializing...");
    add_log("Starting speech recognition engine...", LogLevel::Info);

    // Initialize NLU engine with SBERT semantic understanding
    add_log("Initializing NLU engine with SBERT...", LogLevel::Info);
    if let Err(e) = GLOBAL_NLU.initialize() {
        add_log(&format!("NLU initialization warning: {}", e), LogLevel::Warning);
        add_log("Falling back to basic command matching", LogLevel::Info);
    } else {
        if GLOBAL_NLU.is_sbert_enabled() {
            add_log("SBERT semantic engine active - enhanced understanding enabled", LogLevel::Success);
        } else {
            add_log("NLU engine ready (keyword mode)", LogLevel::Success);
        }
    }

    // Initialize optimized TTS engine for low latency
    if let Err(e) = TTS_ENGINE.initialize() {
        add_log(
            &format!("TTS engine init warning: {}", e),
            LogLevel::Warning,
        );
    }

    // Initialize app monitoring (now handled by plugin system)
    add_log("Application plugin system initialized", LogLevel::Info);

    // Initialize FastSwap (stored but not started — starts on demand via UI or voice cmd)
    let local_ip = local_ip_address::local_ip()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)))
        .to_string();
    let local_device = fastswap::Device::new_local(
        format!("IGRIS-{}", whoami::username()),
        53317,
        local_ip.clone(),
    );
    if let Ok(mut dev_guard) = fastswap::FASTSWAP_DEVICE.lock() {
        *dev_guard = Some(local_device);
    }
    let fastswap_manager = fastswap::FastSwapManager::new(53317);
    if let Ok(mut manager_guard) = fastswap::FASTSWAP_MANAGER.lock() {
        *manager_guard = Some(fastswap_manager);
    }
    add_log("FastSwap ready (starts on demand)", LogLevel::Info);

    #[cfg(feature = "candle")]
    {
        let llm_model_path = "pkg/models/qwen2.5-1.5b-instruct-q4_k_m.gguf";
        if std::path::Path::new(llm_model_path).exists() {
            match core::local_llm::init_local_llm(llm_model_path) {
                Ok(_) => add_log("Local reasoning LLM loaded", LogLevel::Success),
                Err(e) => add_log(&format!("Local LLM init error: {}", e), LogLevel::Warning),
            }
        } else {
            add_log("Local LLM model not found — download via setup to enable smart reasoning", LogLevel::Info);
        }
    }

    let whisper_ctx = match init_whisper_context() {
        Ok(ctx) => {
            update_status("Initialized - Waiting for wake word");
            add_log("Whisper model loaded successfully", LogLevel::Success);

            // Speak IGRIS greeting on app launch
            if let Err(e) = utils::greetings::speak_invoke_greeting() {
                add_log(&format!("Greeting error: {}", e), LogLevel::Warning);
            }
            add_log("IGRIS greeting spoken", LogLevel::Info);

            {
                let mut state = ASSISTANT_STATE.lock().unwrap();
                state.is_initialized = true;
                state.is_awake = false;
                state.setup_in_progress = false;
            }

            ctx
        }
        Err(e) => {
            update_status("Initialization Failed");
            add_log(&format!("Failed to initialize: {}", e), LogLevel::Error);
            let _ = core::tts::speak(
                "Sorry, I failed to initialize. Please check the model files and restart.",
            );
            return;
        }
    };

    // Main wake word loop
    loop {
        // Check for reset signal from hotkey
        if RESET_FLAG.swap(false, Ordering::Relaxed) {
            add_log("Reset signal received - restarting from wake word", LogLevel::Info);
            {
                let mut state = ASSISTANT_STATE.lock().unwrap();
                state.is_awake = false;
                state.is_listening = false;
            }
        }
        
        // Skip wake word listening while presentation is active
        if ui::is_presentation_active() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }
        
update_status("Sleeping - Say 'hello' to wake me");
add_log("Listening for wake word 'hello'...", LogLevel::Info);

        match listen_for_wake_word(&whisper_ctx) {
            Ok(_) => {
                // Check for reset signal right after wake word
                if RESET_FLAG.swap(false, Ordering::Relaxed) {
                    add_log("Reset during wake - going back to sleep", LogLevel::Info);
                    continue;
                }
                
                {
                    let mut state = ASSISTANT_STATE.lock().unwrap();
                    state.is_awake = true;
                }

                update_status("Awake - Listening for command");
                add_log("Wake word detected!", LogLevel::Success);
                let _ = core::tts::speak("Yes, I'm listening. What can I do for you?");

                match continuous_listening_mode(&whisper_ctx).await {
                    Ok(should_exit) => {
                        if should_exit {
                            {
                                let mut state = ASSISTANT_STATE.lock().unwrap();
                                state.is_awake = false;
                            }
                            update_status("Shutting down...");
                            add_log("Goodbye!", LogLevel::Info);
                            let _ = core::tts::speak("Goodbye! See you next time.");
                            std::process::exit(0);
                        } else {
                            let mut state = ASSISTANT_STATE.lock().unwrap();
                            state.is_awake = false;
                        }
                    }
                    Err(e) => {
                        {
                            let mut state = ASSISTANT_STATE.lock().unwrap();
                            state.is_awake = false;
                        }
                        add_log(&format!("Error: {}", e), LogLevel::Error);
                        let _ = core::tts::speak("I encountered an error. Going back to sleep.");
                    }
                }
            }
            Err(e) => {
                add_log(&format!("Wake word error: {}", e), LogLevel::Warning);
            }
        }
    }
}

async fn continuous_listening_mode(
    whisper_ctx: &whisper_rs::WhisperContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    update_status("Listening Mode");
    add_log("Entering continuous listening mode (VAD-optimized)", LogLevel::Info);

    {
        let mut state = ASSISTANT_STATE.lock().unwrap();
        state.is_listening = true;
    }

    loop {
        // Check for reset signal from hotkey - bail out to wake word loop
        if RESET_FLAG.swap(false, Ordering::Relaxed) {
            add_log("Reset signal received in command mode", LogLevel::Info);
            let _ = core::tts::speak("Going back to sleep.");
            let mut state = ASSISTANT_STATE.lock().unwrap();
            state.is_listening = false;
            return Ok(false);
        }
        
        // Skip voice listening while presentation is active
        if ui::is_presentation_active() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }
        
        add_log("Listening for command...", LogLevel::Info);

        let capture_result = match capture_audio_vad(CaptureConfig::command()) {
            Ok(result) => result,
            Err(e) => {
                add_log(&format!("Recording failed: {}", e), LogLevel::Warning);
                continue;
            }
        };

        if !capture_result.speech_detected || capture_result.samples.is_empty() {
            continue;
        }

        if let Some(time_ms) = capture_result.time_to_speech_ms {
            add_log(&format!("Speech detected in {}ms", time_ms), LogLevel::Info);
        }

        let command = match hybrid_transcribe_audio(&capture_result.samples, whisper_ctx).await {
            Ok(text) => text.trim().to_string(),
            Err(_) => continue,
        };

        if command.is_empty() {
            continue;
        }

        add_log(&format!("You said: \"{}\"", command), LogLevel::Info);

        {
            let mut state = ASSISTANT_STATE.lock().unwrap();
            state.last_command = command.clone();
        }

        let should_exit = process_voice_command(&command, whisper_ctx).await?;

        if should_exit {
            let mut state = ASSISTANT_STATE.lock().unwrap();
            state.is_listening = false;
            return Ok(true);
        }

        // Check for sleep/standby/hibernate commands
        let cmd_lower = command.to_lowercase();
        if cmd_lower.contains("sleep") 
            || cmd_lower.contains("standby")
            || cmd_lower.contains("hibernate") {
            let _ = core::tts::speak("Okay, going to sleep. Say hello to wake me.");
            add_log("Entering sleep mode", LogLevel::Info);
            let mut state = ASSISTANT_STATE.lock().unwrap();
            state.is_listening = false;
            return Ok(false);
        }
    }
}


async fn process_voice_command(
    command: &str,
    whisper_ctx: &whisper_rs::WhisperContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Check for reset signal from hotkey
    if RESET_FLAG.swap(false, Ordering::Relaxed) {
        return Ok(false);
    }
    
    // Quick check for exit commands (fastest path)
    // But exclude "exit camera", "close camera" etc - those should go to plugin system
    let cmd_lower = command.to_lowercase();
    let is_camera_command = cmd_lower.contains("camera") 
        || cmd_lower.contains("photo") 
        || cmd_lower.contains("recording")
        || cmd_lower.contains("preview");
    
    if !is_camera_command && (
        cmd_lower.contains("exit") 
        || cmd_lower.contains("quit") 
        || cmd_lower.contains("terminate")
        || cmd_lower.contains("close assistant")
        || cmd_lower.contains("turn off")
    ) {
        add_log("Exit command received - Shutting down IGRIS", LogLevel::Success);
        let _ = core::tts::speak("Goodbye! Thank you for using IGRIS. See you next time.");
        
        
        // Give audio time to play
        std::thread::sleep(std::time::Duration::from_millis(1500));
        
        nlu::context::add_to_context(
            command.to_string(),
            "Goodbye!".to_string(),
            "exit".to_string(),
            vec![],
        );
        
        // Exit the entire application
        std::process::exit(0);
    }
    
    // Resolve references using context memory (it, that, this, etc.)
    let resolved_command = nlu::context::resolve_references(command);
    let command_to_use = if resolved_command != command {
        add_log(&format!("Resolved: {} → {}", command, resolved_command), LogLevel::Info);
        &resolved_command
    } else {
        command
    };
    
    // PRIORITY: Check for "about igris" / "tell me about yourself" BEFORE plugins
    let cmd_lower = command_to_use.to_lowercase();
    if cmd_lower.contains("tell me about yourself") 
        || cmd_lower.contains("tell me about you")
        || cmd_lower.contains("who are you")
        || cmd_lower.contains("what are you")
        || cmd_lower.contains("introduce yourself")
        || cmd_lower.contains("about yourself")
        || cmd_lower.contains("tell me about igris")
        || cmd_lower.contains("what is igris")
        || cmd_lower.contains("describe yourself")
        || cmd_lower.contains("explain yourself") {
        add_log("About IGRIS query detected", LogLevel::Info);
        
        match commands::about::handle_about_command(command_to_use) {
            Ok(response) => {
                add_log("IGRIS presentation started", LogLevel::Success);
                nlu::context::add_to_context(
                    command.to_string(),
                    "Introduced myself".to_string(),
                    "about_igris".to_string(),
                    vec![],
                );
            }
            Err(e) => {
                add_log(&format!("About error: {}", e), LogLevel::Error);
            }
        }
        return Ok(false);
    }
    
    // Check for FastSwap / file sharing commands
    if cmd_lower.contains("fastswap") 
        || cmd_lower.contains("fast swap")
        || cmd_lower.contains("start fastswap")
        || cmd_lower.contains("share files")
        || cmd_lower.contains("file sharing")
        || cmd_lower.contains("open share")
        || cmd_lower.contains("file share") {
        add_log("FastSwap command detected", LogLevel::Info);
        let _ = core::tts::speak("Opening FastSwap file sharing interface");
        
        // Set the global UI state to show FastSwap
        if let Ok(mut ui_state) = UI_PANEL_STATE.lock() {
            ui_state.show_fastswap = true;
        }
        
        add_log("FastSwap interface opened", LogLevel::Success);
        return Ok(false);
    }
    
    // First, try plugin system
    if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
        add_log(&format!("Plugin: {} - {}", plugin_result.plugin_name, plugin_result.command.description), LogLevel::Info);
        
        match plugins::execute_plugin_command(&plugin_result) {
            Ok(response) => {
                // Check if this is a custom function (file operations, etc.)
                if response.starts_with("CUSTOM_FN:") {
                    let action = response.strip_prefix("CUSTOM_FN:").unwrap_or("");
                    add_log(&format!("Custom function: {}", action), LogLevel::Info);
                    
                    // Handle open_app commands
                    if action.starts_with("open_app:") {
                        let app_name = action.strip_prefix("open_app:").unwrap_or("");
                        match platform::app_launcher::AppLauncherImpl::new().open_app(app_name) {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                                refresh_running_apps();
                            }
                            Err(e) => {
                                add_log(&format!("Error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak(&format!("Failed to open {}", app_name));
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle close_app commands
                    if action.starts_with("close_app:") {
                        let app_name = action.strip_prefix("close_app:").unwrap_or("");
                        match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                                refresh_running_apps();
                            }
                            Err(e) => {
                                add_log(&format!("Error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak(&format!("Failed to close {}", app_name));
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle close all apps
                    if action == "close_all_apps" {
                        match utils::close_all_apps() {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                            }
                            Err(e) => {
                                add_log(&format!("Error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak("Failed to close apps");
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle close all camera processes
                    if action == "close_all_camera" {
                        match utils::close_all_camera() {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                            }
                            Err(e) => {
                                add_log(&format!("Error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak("Failed to close camera");
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle file operations
                    if action.starts_with("file:") || action.starts_with("folder:") {
                        // Pass to file command handler
                        if let Some(file_response) = commands::files::process_file_command_async(command_to_use).await {
                            add_log(&file_response, LogLevel::Success);
                            let _ = core::tts::speak(&file_response);
                        } else {
                            let _ = core::tts::speak("I couldn't complete that file operation.");
                        }
                        return Ok(false);
                    }
                    
                    // Handle alarm commands
                    if action.starts_with("alarm_") {
                        match commands::reminders::handle_alarm_command(action, command_to_use) {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                            }
                            Err(e) => {
                                add_log(&format!("Alarm error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak(&e);
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle reminder commands
                    if action.starts_with("reminder_") {
                        match commands::reminders::handle_reminder_command(action, command_to_use) {
                            Ok(msg) => {
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                            }
                            Err(e) => {
                                add_log(&format!("Reminder error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak(&e);
                            }
                        }
                        return Ok(false);
                    }
                    
                    // Handle system control commands
                    if action.starts_with("system_") {
                        if let Some(response) = commands::system::process_system_command(command_to_use) {
                            add_log(&response, LogLevel::Success);
                            let _ = core::tts::speak(&response);
                        } else {
                            add_log("System command failed", LogLevel::Error);
                            let _ = core::tts::speak("I couldn't execute that system command");
                        }
                        return Ok(false);
                    }
                    
                }
                
                // Check if this is a camera mode command
                if response.starts_with("CAMERA_MODE:") {
                    let camera_action = response.strip_prefix("CAMERA_MODE:").unwrap_or("");
                    println!("[DEBUG] Camera mode response: {}", camera_action);
                    
                    // Handle FFmpeg camera actions
                    if camera_action.starts_with("ffmpeg_") {
                        let action = camera_action.strip_prefix("ffmpeg_").unwrap_or("");
                        add_log(&format!("Camera action: {}", action), LogLevel::Info);
                        println!("[DEBUG] Calling handle_camera_command with: {}", action);
                        
                        match commands::ffmpeg_camera::handle_camera_command(action) {
                            Ok(msg) => {
                                println!("[DEBUG] Camera command success: {}", msg);
                                add_log(&msg, LogLevel::Success);
                                let _ = core::tts::speak(&msg);
                            }
                            Err(e) => {
                                println!("[DEBUG] Camera command error: {}", e);
                                add_log(&format!("Camera error: {}", e), LogLevel::Error);
                                let _ = core::tts::speak(&format!("Camera error: {}", e));
                            }
                        }
                        return Ok(false);
                    }
                }
                
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                
                // Add to context memory
                nlu::context::add_to_context(
                    command.to_string(),
                    response.clone(),
                    "plugin".to_string(),
                    vec![plugin_result.plugin_name.clone()],
                );
                
                return Ok(false);
            }
            Err(e) => {
                add_log(&format!("Plugin execution error: {}", e), LogLevel::Error);
                // Continue to fallback processing instead of getting stuck
                add_log("Trying alternative command processing...", LogLevel::Info);
            }
        }
    }
    
    // First, try NLU-based intent recognition
    let nlu_result = GLOBAL_NLU.process_input(command_to_use);
    
    if let Ok(ref intent_result) = nlu_result {
        if intent_result.intent_name != "UnknownIntent" {
            add_log(
                &format!("NLU: {} (confidence: {:.2})", intent_result.intent_name, intent_result.confidence),
                LogLevel::Info,
            );
            
            // Process based on recognized intent
            match intent_result.intent_name.as_str() {
                "assistant_control" => {
                    let cmd_lower = command_to_use.to_lowercase();
                    
                    // Exit/Terminate commands - close the assistant and window
                    if cmd_lower.contains("exit") 
                        || cmd_lower.contains("quit") 
                        || cmd_lower.contains("terminate")
                        || cmd_lower.contains("shutdown assistant")
                        || cmd_lower.contains("close assistant")
                        || cmd_lower.contains("turn off") {
                        add_log("Exit command received - Shutting down IGRIS", LogLevel::Success);
                        let _ = core::tts::speak("Goodbye! Thank you for using IGRIS. See you next time.");
                        
                        // Give audio time to play
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        
                        nlu::context::add_to_context(
                            command.to_string(),
                            "Goodbye!".to_string(),
                            "assistant_control".to_string(),
                            vec![],
                        );
                        
                        // Exit the entire application
                        std::process::exit(0);
                    }
                    
                    // Sleep/Standby commands - go back to wake word listening
                    if cmd_lower.contains("sleep") 
                        || cmd_lower.contains("standby")
                        || cmd_lower.contains("hibernate") {
                        let _ = core::tts::speak("Okay, going to sleep. Say hello to wake me.");
                        add_log("Entering sleep mode", LogLevel::Info);
                        nlu::context::add_to_context(
                            command.to_string(),
                            "Going to sleep".to_string(),
                            "assistant_control".to_string(),
                            vec![],
                        );
                        return Ok(false);
                    }
                }
                "open_app" => {
                    if let Some(app) = intent_result.entities.get("app") {
                        add_log(&format!("Opening app: {}", app), LogLevel::Info);
                    }
                    if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                        if let Ok(response) = crate::plugins::execute_plugin_command(&plugin_result) {
                            add_log(&response, LogLevel::Success);
                            let _ = core::tts::speak(&response);
                            refresh_running_apps();
                        
                            // Add to context
                            let entities: Vec<String> = intent_result.entities.values().cloned().collect();
                            nlu::context::add_to_context(
                                command.to_string(),
                                response.clone(),
                                "open_app".to_string(),
                                entities,
                            );
                            
                            return Ok(false);
                        }
                    }
                }
                "close_app" => {
                    // Execute through plugin system for unified app/plugin handling
                    if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                        if let Ok(response) = crate::plugins::execute_plugin_command(&plugin_result) {
                            add_log(&response, LogLevel::Success);
                            let _ = core::tts::speak(&response);
                            refresh_running_apps();
                            
                            // Add to context
                            nlu::context::add_to_context(
                                command.to_string(),
                                response.clone(),
                                "close_app".to_string(),
                                intent_result.entities.values().cloned().collect(),
                            );
                            
                            return Ok(false);
                        }
                    }
                }
                "camera_control" => {
                    // Use FFmpeg camera
                    add_log("Starting camera", LogLevel::Info);
                    match commands::ffmpeg_camera::handle_camera_command("start") {
                        Ok(msg) => {
                            add_log(&msg, LogLevel::Success);
                            let _ = core::tts::speak(&msg);
                        }
                        Err(e) => {
                            add_log(&format!("Camera error: {}", e), LogLevel::Error);
                            let _ = core::tts::speak(&format!("Camera error: {}", e));
                        }
                    }
                    return Ok(false);
                }
                // Commented out - let system commands go through plugin system or fallback
                // "system_control" => {
                //     add_log(&format!("System control: {}", command), LogLevel::Info);
                //     
                //     // Use system_control module with NER
                //     if let Some(response) = commands::system::process_system_command(command) {
                //         add_log(&response, LogLevel::Success);
                //         let _ = core::tts::speak(&response);
                //         return Ok(false);
                //     } else {
                //         let _ = core::tts::speak("I couldn't execute that system command.");
                //         return Ok(false);
                //     }
                // }
                "greeting" => {
                    let _ = core::tts::speak("Hello! How can I help you today?");
                    return Ok(false);
                }
                "web_search" => {
                    add_log(&format!("Web search: {}", command), LogLevel::Info);
                    
                    if let Some(response) = commands::web::process_search_command(command) {
                        add_log(&response, LogLevel::Success);
                        let _ = core::tts::speak(&response);
                        return Ok(false);
                    } else {
                        let _ = core::tts::speak("I couldn't process that search request.");
                        return Ok(false);
                    }
                }
                "about_igris" => {
                    add_log("About IGRIS query", LogLevel::Info);
                    
                    match commands::about::handle_about_command(command) {
                        Ok(response) => {
                            add_log("IGRIS introduction delivered", LogLevel::Success);
                            // TTS is handled inside handle_about_command
                            
                            // Add to context
                            nlu::context::add_to_context(
                                command.to_string(),
                                "Introduced myself".to_string(),
                                "about_igris".to_string(),
                                vec![],
                            );
                        }
                        Err(e) => {
                            add_log(&format!("About error: {}", e), LogLevel::Error);
                        }
                    }
                    return Ok(false);
                }
                _ => {}
            }
        }
    }

    #[cfg(feature = "candle")]
    {
        if is_local_llm_ready() {
            add_log("[LocalLLM] Trying local reasoning model...", LogLevel::Info);
            if let Some(output) = global_reason(&default_tool_system_prompt(), command_to_use) {
                add_log(
                    &format!("[LocalLLM] Raw output: {}", &output[..output.len().min(120)]),
                    LogLevel::Info,
                );
                if let Some((tool, args)) = parse_tool_call(&output) {
                    add_log(
                        &format!("[LocalLLM] Tool: {} | args: {}", tool, args),
                        LogLevel::Info,
                    );
                    let response = route_llm_tool(tool, args, command_to_use).await;
                    nlu::context::add_to_context(
                        command.to_string(),
                        response.clone(),
                        format!("llm_{}", tool),
                        vec![],
                    );
                    return Ok(false);
                }
            }
        }
    }

    // Fallback to keyword-based matching
    let cmd_lower = command_to_use.to_lowercase();

    // Exit/Terminate commands - close the assistant
    if cmd_lower.contains("exit") 
        || cmd_lower.contains("quit") 
        || cmd_lower.contains("terminate")
        || cmd_lower.contains("shutdown") {
        add_log("Exit command received", LogLevel::Success);
        let _ = core::tts::speak("Goodbye! Shutting down.");
        return Ok(true);
    }
    
    // Sleep/Standby/Hibernate commands - go back to wake word listening
    if cmd_lower.contains("sleep") 
        || cmd_lower.contains("standby")
        || cmd_lower.contains("hibernate") {
        let _ = core::tts::speak("Okay, going to sleep. Say hello to wake me.");
        add_log("Entering sleep mode", LogLevel::Info);
        return Ok(false);
    }

    if cmd_lower.contains("close") && !cmd_lower.contains("all") {
        let parts: Vec<&str> = cmd_lower.split("close").collect();
        if parts.len() > 1 {
            let app_name = parts[1].trim();
            if !app_name.is_empty() {
                // Execute through plugin system
                if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                    if let Ok(msg) = crate::plugins::execute_plugin_command(&plugin_result) {
                        add_log(&msg, LogLevel::Success);
                        let _ = core::tts::speak(&msg);
                        refresh_running_apps();
                        
                        // Add to context
                        nlu::context::add_to_context(
                            command.to_string(),
                            msg.clone(),
                            "close_app".to_string(),
                            vec![app_name.to_string()],
                        );
                        
                        return Ok(false);
                    }
                } else {
                    let _ = core::tts::speak(&format!("Couldn't find how to close {}", app_name));
                }
                return Ok(false);
            }
        }
    }

    if cmd_lower.contains("camera") {
        // Use FFmpeg camera
        add_log("Starting camera", LogLevel::Info);
        match commands::ffmpeg_camera::handle_camera_command("start") {
            Ok(msg) => {
                add_log(&msg, LogLevel::Success);
                let _ = core::tts::speak(&msg);
            }
            Err(e) => {
                add_log(&format!("Camera error: {}", e), LogLevel::Error);
                let _ = core::tts::speak(&format!("Camera error: {}", e));
            }
        }
        return Ok(false);
    }

    // Execute app commands through plugin system (removed duplicate - already handled above)
    // This was causing the "stuck" behavior when plugins failed

    if cmd_lower.contains("file") || cmd_lower.contains("folder") {
        if let Some(response) = commands::files::process_file_command_async(command_to_use).await {
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            return Ok(false);
        }
    }

    // Check for system control commands (volume, brightness, wifi, etc.)
    if commands::system::is_system_command(command_to_use) {
        if let Some(response) = commands::system::process_system_command(command_to_use) {
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            return Ok(false);
        }
    }
    
    // Check for web search commands
    if commands::web::is_search_command(command_to_use) {
        if let Some(response) = commands::web::process_search_command(command_to_use) {
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            return Ok(false);
        }
    }

    add_log(&format!("Unknown command: '{}' - trying fallback processing", command), LogLevel::Warning);
    
    // Quick validation - check if command makes sense
    let words: Vec<&str> = cmd_lower.split_whitespace().collect();
    let has_valid_action = words.iter().any(|w| {
        matches!(*w, "open" | "close" | "start" | "stop" | "play" | "pause" | 
                     "search" | "find" | "show" | "create" | "delete" | "set" | 
                     "get" | "increase" | "decrease" | "turn" | "enable" | "disable" |
                     "what" | "where" | "when" | "who" | "why" | "how")
    });
    
    // If no valid action word found, immediately reject
    if !has_valid_action {
        let _ = core::tts::speak("I didn't understand that command.");
        return Ok(false);
    }
    
    // Check for nonsensical word combinations
    let known_apps = ["chrome", "firefox", "notepad", "calculator", "camera", "spotify", "discord"];
    let has_known_target = words.iter().any(|w| known_apps.contains(w));
    let has_unknown_prefix = words.iter().any(|w| {
        matches!(*w, "groupon" | "random" | "weird" | "strange" | "nonsense")
    });
    
    // If unknown prefix with known target, reject immediately
    if has_unknown_prefix && has_known_target {
        let _ = core::tts::speak("That doesn't seem like a valid command.");
        return Ok(false);
    }
    
    // Check if it's a WH-question (what, who, where, when, why, how)
    let is_wh_question = cmd_lower.starts_with("what") 
        || cmd_lower.starts_with("who") 
        || cmd_lower.starts_with("where")
        || cmd_lower.starts_with("when")
        || cmd_lower.starts_with("why")
        || cmd_lower.starts_with("how");
    
    if is_wh_question {
        // Fallback: Search the web for WH-questions
        add_log("Searching the web for answer...", LogLevel::Info);
        let _ = core::tts::speak("I don't have information about that. Let me search the web for you.");
        
        // Try to fetch and read search results
        if let Some(answer) = commands::web::search_and_read_results(command_to_use) {
            add_log(&answer, LogLevel::Success);
            let _ = core::tts::speak(&answer);
        } else {
            // If fetching fails, just open browser
            if let Ok(msg) = commands::web::search_in_browser(command_to_use, commands::web::SearchEngine::Google) {
                add_log(&msg, LogLevel::Success);
            }
        }
    } else {
        // For non-WH questions, just say we don't understand
        let _ = core::tts::speak(
            "I didn't understand that. Try opening an app, controlling the system, or asking a question.",
        );
    }
    
    Ok(false)
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
    let mut logs_list = use_signal(|| {
        vec![(
            format!("Welcome to {} - Your Voice Assistant", CONFIG.assistant_name()),
            LogLevel::Info,
        )]
    });
    let mut apps_list = use_signal(|| Vec::new());
    let _show_logs = use_signal(|| CONFIG.get().ui.show_logs);
    
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

                logs_list.set(state.logs.clone());
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
    let logs = logs_list();
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

                // Menu Button (top right)
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

                    // Status display
                    div { style: "display: flex; flex-direction: column; align-items: center; gap: clamp(16px, 3vh, 32px); max-width: min(90vw, 800px); width: 100%;",
                        div { style: "text-align: center;",
                            div { style: "font-size: clamp(10px, 1.5vw, 12px); letter-spacing: 1px; color: #9ca3af; margin-bottom: clamp(4px, 1vh, 8px); text-transform: uppercase;",
                                "System Status"
                            }
                            h2 {
                                style: format!(
                                    "font-size: clamp(18px, 4vw, 32px); font-weight: bold; background: linear-gradient(90deg, {}, {}); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; animation: fade-in-out 2s ease-in-out infinite; transition: all 0.5s ease-in-out;",
                                    primary_color,
                                    secondary_color,
                                ),
                                "{status}"
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

                // Logs Panel - Bottom Right with dynamic border - responsive
                div {
                    style: format!(
                        "position: fixed; bottom: clamp(12px, 3vh, 24px); right: clamp(12px, 3vw, 24px); width: clamp(200px, 35vw, 384px); max-height: clamp(120px, 30vh, 256px); background: rgba(0, 0, 0, 0.8); border: 1px solid rgba({}, 0.4); border-radius: 8px; padding: clamp(8px, 2vh, 16px); backdrop-filter: blur(10px); overflow-y: auto; overflow-x: hidden; z-index: 40; transition: border-color 0.5s ease-in-out;",
                        accent_rgb,
                    ),
                    div { style: "font-size: clamp(10px, 1.5vw, 12px); letter-spacing: 1px; color: #9ca3af; margin-bottom: clamp(6px, 1.5vh, 12px); text-transform: uppercase;",
                        "System Logs"
                    }
                    div { style: "display: flex; flex-direction: column; gap: clamp(4px, 1vh, 8px);",
                        for (log , level) in logs.iter().rev().take(10) {
                            div {
                                style: format!(
                                    "font-size: clamp(10px, 1.3vw, 12px); color: {}; font-family: monospace; word-break: break-word;",
                                    match level {
                                        LogLevel::Success => "#22c55e",
                                        LogLevel::Error => "#ef4444",
                                        LogLevel::Warning => "#f59e0b",
                                        _ => primary_color,
                                    },
                                ),
                                span { style: "color: #4b5563; margin-right: 4px;", "> " }
                                "{log}"
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

fn update_status(status: &str) {
    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.current_status = status.to_string();
}

fn add_log(message: &str, level: LogLevel) {
    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.logs.push((message.to_string(), level));
    if state.logs.len() > 100 {
        state.logs.remove(0);
    }
}

/// Route an LLM-discovered tool call to the appropriate command handler.
#[cfg(feature = "candle")]
async fn route_llm_tool(tool: &str, _args: &str, command_to_use: &str) -> String {
    match tool {
        "open_app" => {
            if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                match crate::plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        add_log(&msg, LogLevel::Success);
                        let _ = core::tts::speak(&msg);
                        refresh_running_apps();
                        msg
                    }
                    Err(e) => format!("Failed: {}", e),
                }
            } else {
                let response = "I couldn't find that application.";
                let _ = core::tts::speak(response);
                response.to_string()
            }
        }
        "close_app" => {
            if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                match crate::plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        add_log(&msg, LogLevel::Success);
                        let _ = core::tts::speak(&msg);
                        refresh_running_apps();
                        msg
                    }
                    Err(e) => format!("Failed: {}", e),
                }
            } else {
                let response = "I couldn't close that application.";
                let _ = core::tts::speak(response);
                response.to_string()
            }
        }
        "close_all_apps" => {
            let response = commands::app_utils::close_all_apps().unwrap_or_default();
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            refresh_running_apps();
            response
        }
        "search_web" => {
            let response = commands::web::search_and_read_results(command_to_use)
                .unwrap_or_else(|| "Search failed.".to_string());
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "open_website" => {
            let response = "Opening website...";
            let _ = core::tts::speak(response);
            if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                let _ = crate::plugins::execute_plugin_command(&plugin_result);
            }
            response.to_string()
        }
        "system_command" => {
            if let Some(response) = commands::system::process_system_command(command_to_use) {
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            } else {
                let response = "System command failed.";
                let _ = core::tts::speak(response);
                response.to_string()
            }
        }
        "camera_action" => {
            let _ = core::tts::speak("Processing camera command...");
            // Try plugin system first, then fallback to direct command
            if let Some(plugin_result) = crate::plugins::process_plugin_command(command_to_use) {
                match crate::plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        add_log(&msg, LogLevel::Success);
                        return msg;
                    }
                    Err(_) => {}
                }
            }
            let response = "Camera command processed.";
            response.to_string()
        }
        "file_operation" => {
            let _ = core::tts::speak("Processing file command...");
            if let Some(response) = commands::files::process_file_command_async(command_to_use).await {
                add_log(&response, LogLevel::Success);
                response
            } else {
                let response = "I couldn't complete that file operation.";
                let _ = core::tts::speak(response);
                response.to_string()
            }
        }
        "set_alarm" => {
            let response = commands::reminders::handle_alarm_command("alarm_set", command_to_use)
                .unwrap_or_else(|e| format!("Alarm error: {}", e));
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "set_reminder" => {
            let response = commands::reminders::handle_reminder_command("reminder_set", command_to_use)
                .unwrap_or_else(|e| format!("Reminder error: {}", e));
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "get_weather" => {
            let response = commands::web::search_and_read_results("weather today")
                .unwrap_or_else(|| "Weather lookup failed.".to_string());
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "tell_fact" => {
            let response = commands::web::search_and_read_results("tell me an interesting fact")
                .unwrap_or_else(|| "Search failed.".to_string());
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "tell_joke" => {
            let response = commands::web::search_and_read_results("tell me a joke")
                .unwrap_or_else(|| "Search failed.".to_string());
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "general_chat" | _ => {
            let response = if tool == "general_chat" {
                extract_chat_response(_args).unwrap_or_else(|| "I'm not sure how to respond to that.".to_string())
            } else {
                "I don't know how to do that yet.".to_string()
            };
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
    }
}

/// Quick helper to pull the "response" field from a JSON args blob.
#[cfg(feature = "candle")]
fn extract_chat_response(args: &str) -> Option<String> {
    let re = regex::Regex::new(r#""response"\s*:\s*"([^"]+)""#).ok()?;
    let caps = re.captures(args)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

fn refresh_running_apps() {
    // Get apps from our process tracker (apps opened by IGRIS)
    // Clean up dead processes first, then get running apps
    let tracked_apps: Vec<String> = if let Ok(mut tracker) = utils::PROCESS_TRACKER.lock() {
        tracker.cleanup(); // Remove dead processes
        tracker.get_by_category(utils::ProcessCategory::App)
            .iter()
            .filter(|p| tracker.is_running(&p.exe_name)) // Only show running apps
            .map(|p| p.name.clone())
            .collect()
    } else {
        Vec::new()
    };
    
    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.running_apps = tracked_apps;
}
