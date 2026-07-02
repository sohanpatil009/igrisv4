use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use igrisv3::{
    core, ui, fastswap, online, utils, RESET_FLAG, FORCE_LISTEN,
};
use igrisv3::core::stt::{init_stt_engine, SttEngine, hybrid_transcribe_audio};
use igrisv3::core::wake_word::{listen_for_wake_word, listen_for_wake_word_async};
use igrisv3::core::audio_capture::{capture_audio_vad, CaptureConfig};
use igrisv3::core::tts::TTS_ENGINE;
use igrisv3::nlu::engine::GLOBAL_NLU;

#[cfg(feature = "candle")]
use igrisv3::core::local_llm::init_local_llm;

use crate::state::*;
use crate::processor::process_voice_command;
use crate::tools::{refresh_running_apps, cleanup_and_exit};

pub async fn start_voice_assistant() {
    // Initialize
    update_status("Initializing...");
    add_log("Starting speech recognition engine...", LogLevel::Info);

    // Skip offline NLU init when in online mode (loaded on-demand if needed)
    if !online::is_online_mode() {
        add_log("Initializing NLU engine with SBERT...", LogLevel::Info);
        if let Err(e) = GLOBAL_NLU.initialize() {
            add_log(&format!("NLU initialization warning: {}", e), LogLevel::Warning);
            add_log("Falling back to basic command matching", LogLevel::Info);
        } else {
            NLU_READY.store(true, Ordering::Relaxed);
            if GLOBAL_NLU.is_sbert_enabled() {
                add_log("SBERT semantic engine active - enhanced understanding enabled", LogLevel::Success);
            } else {
                add_log("NLU engine ready (keyword mode)", LogLevel::Success);
            }
        }
    } else {
        add_log("[Online Mode] Skipping offline NLU initialization (will init on-demand if needed)", LogLevel::Info);
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
    if !online::is_online_mode() {
        let llm_model_path = "pkg/models/qwen2.5-1.5b-instruct-q4_k_m.gguf";
        if std::path::Path::new(llm_model_path).exists() {
            match init_local_llm(llm_model_path) {
                Ok(_) => add_log("Local reasoning LLM loaded", LogLevel::Success),
                Err(e) => add_log(&format!("Local LLM init error: {}", e), LogLevel::Warning),
            }
        } else {
            add_log("Local LLM model not found — download via setup to enable smart reasoning", LogLevel::Info);
        }
    }
    // In online mode, skip loading the 940MB SenseVoice model — Parakeet handles everything
    let stt_engine: Option<SttEngine> = if !online::is_online_mode() {
        match init_stt_engine() {
            Ok(engine) => {
                update_status("Initialized - Waiting for wake word");
                add_log("SenseVoice model loaded successfully", LogLevel::Success);

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

                Some(engine)
            }
            Err(e) => {
                update_status("Initialization Failed");
                add_log(&format!("Failed to initialize STT: {}", e), LogLevel::Error);
                let _ = core::tts::speak(
                    "Sorry, I failed to initialize. Please check the model files and restart.",
                );
                return;
            }
        }
    } else {
        update_status("Initialized - Waiting for wake word (Online mode)");
        add_log("[Online Mode] SenseVoice model skipped (940MB) — using Parakeet STT", LogLevel::Success);
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
        None
    };

    // Spawn background internet connectivity monitor (checks every 15s)
    // Automatically switches between online and offline mode as connectivity changes
    let _connectivity_handle = tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let online_now = online::is_online_mode();
            let has_internet = online::check_internet_connectivity().await;

            if online_now && !has_internet {
                println!("[NET] Internet lost — switching to offline mode");
                add_log("[NET] Internet disconnected — switching to offline mode", LogLevel::Warning);
                online::disable_online_mode();
                let _ = core::tts::speak("Internet connection lost. Switching to offline mode.");
            } else if !online_now && has_internet {
                let has_api_key = std::env::var("NVIDIA_API_KEY").is_ok();
                if has_api_key {
                    ONLINE_FAIL_COUNT.store(0, Ordering::Relaxed);
                    println!("[NET] Internet restored — switching to online mode");
                    add_log("[NET] Internet restored — switching to online mode", LogLevel::Success);
                    online::enable_online_mode();
                    let _ = core::tts::speak("Internet connection restored. Switching to online mode.");
                }
            }
        }
    });

    // Main wake word loop
    loop {
        // Check for force-listen signal from hotkey (skip wake word, enter listening mode)
        if FORCE_LISTEN.swap(false, Ordering::Relaxed) {
            add_log("Hotkey forced wake - entering listening mode", LogLevel::Info);
            {
                let mut state = ASSISTANT_STATE.lock().unwrap();
                state.is_awake = true;
                state.is_listening = false;
            }
            update_status("Awake - Listening for command");
            let _ = core::tts::speak("Yes, I'm listening. What can I do for you?");
            match continuous_listening_mode(stt_engine.as_ref()).await {
                Ok(should_exit) => {
                    if should_exit {
                        {
                            let mut state = ASSISTANT_STATE.lock().unwrap();
                            state.is_awake = false;
                        }
                        update_status("Shutting down...");
                        add_log("Goodbye!", LogLevel::Info);
                        let _ = core::tts::speak("Goodbye! See you next time.");
                        cleanup_and_exit();
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
            continue;
        }

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
            thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        update_status("Sleeping - Say 'hello' to wake me");
        add_log("Listening for wake word 'hello'...", LogLevel::Info);

        // Use Parakeet STT for wake word in online mode, local SenseVoice otherwise
        let wake_result = if online::is_online_mode() {
            listen_for_wake_word_async(|samples| {
                let owned_samples = samples.to_vec();
                async move {
                    online::transcribe_online(&owned_samples)
                        .await
                        .map_err(|e| e.into())
                }
            })
            .await
        } else if let Some(ref engine) = stt_engine {
            listen_for_wake_word(engine)
        } else {
            // Shouldn't happen in offline mode since we load it above, but handle gracefully
            thread::sleep(std::time::Duration::from_millis(100));
            continue;
        };

        match wake_result {
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

                match continuous_listening_mode(stt_engine.as_ref()).await {
                    Ok(should_exit) => {
                        if should_exit {
                            {
                                let mut state = ASSISTANT_STATE.lock().unwrap();
                                state.is_awake = false;
                            }
                            update_status("Shutting down...");
                            add_log("Goodbye!", LogLevel::Info);
                            let _ = core::tts::speak("Goodbye! See you next time.");
                            cleanup_and_exit();
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

pub async fn continuous_listening_mode(
    stt_engine: Option<&SttEngine>,
) -> Result<bool, Box<dyn std::error::Error>> {
    update_status("Listening Mode");
    add_log("Entering continuous listening mode (VAD-optimized)", LogLevel::Info);

    {
        let mut state = ASSISTANT_STATE.lock().unwrap();
        state.is_listening = true;
    }

    // Consume any FORCE_LISTEN flag that arrived while entering — no-op inside listen mode
    FORCE_LISTEN.swap(false, Ordering::Relaxed);

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
            thread::sleep(std::time::Duration::from_millis(100));
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

        let command = if online::is_online_mode() {
            match online::transcribe_online(&capture_result.samples).await {
                Ok(text) => {
                    add_log("[Online STT] Parakeet ASR", LogLevel::Info);
                    text.trim().to_string()
                }
                Err(e) => {
                    add_log(&format!("[Online STT] Failed ({}), no local fallback available", e), LogLevel::Warning);
                    continue;
                }
            }
        } else if let Some(engine) = stt_engine {
            match hybrid_transcribe_audio(&capture_result.samples, engine).await {
                Ok(text) => text.trim().to_string(),
                Err(_) => continue,
            }
        } else {
            add_log("[STT] No local STT engine available — skipping", LogLevel::Warning);
            continue;
        };

        if command.is_empty() {
            continue;
        }

        add_log(&format!("You said: \"{}\"", command), LogLevel::Info);

        {
            let mut state = ASSISTANT_STATE.lock().unwrap();
            state.last_command = command.clone();
        }

        let should_exit = process_voice_command(&command, stt_engine).await?;

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
