use std::sync::atomic::Ordering;

use igrisv3::{
    core, nlu, commands, plugins, utils, platform, platform_utils, online,
    RESET_FLAG,
};
use igrisv3::core::stt::SttEngine;
use igrisv3::nlu::engine::GLOBAL_NLU;
use igrisv3::online::reasoning::{extract_json_string_field, parse_tool_call, parse_tool_calls};
use igrisv3::online::task_planner::{self, TaskPlan, TaskStep};

#[cfg(feature = "candle")]
use igrisv3::core::local_llm::{is_local_llm_ready, global_reason, default_tool_system_prompt};

use crate::state::*;
use crate::tools::*;

pub async fn process_voice_command(
    command: &str,
    _stt_engine: Option<&SttEngine>,
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

        // Clean up tracked apps and exit
        cleanup_and_exit();
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

    // Check for online/offline mode switch
    if cmd_lower.contains("switch to online mode")
        || cmd_lower.contains("go online")
        || cmd_lower.contains("enable online mode")
        || cmd_lower.contains("turn on online mode") {
        if online::is_online_mode() {
            let _ = core::tts::speak("Already in online mode.");
        } else {
            online::enable_online_mode();
            add_log("Switched to ONLINE mode (NVIDIA NIM)", LogLevel::Success);
            let _ = core::tts::speak("Switched to online mode. Using cloud STT and reasoning.");
        }
        return Ok(false);
    }
    if cmd_lower.contains("switch to offline mode")
        || cmd_lower.contains("go offline")
        || cmd_lower.contains("disable online mode")
        || cmd_lower.contains("turn off online mode")
        || cmd_lower.contains("switch to local mode") {
        if !online::is_online_mode() {
            let _ = core::tts::speak("Already in offline mode.");
        } else {
            online::disable_online_mode();
            add_log("Switched to OFFLINE mode (local models)", LogLevel::Success);
            let _ = core::tts::speak("Switched to offline mode. Using local models now.");
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

    // ── Online mode: intent router first (LLM-powered, understands multi-intent) ──
    if online::is_online_mode() {
        add_log("[IntentRouter] Classifying intent via LLM...", LogLevel::Info);
        if let Some(plan) = online::intent_router::route_intent(command_to_use).await {
            add_log(&format!("[IntentRouter] Plan: {} steps", plan.steps.len()), LogLevel::Info);
            let summary = if plan.steps.len() > 1 {
                let s = execute_task_plan(&plan, command_to_use).await;
                nlu::context::add_to_context(
                    command.to_string(), s.clone(), "intent_router_plan".to_string(), vec![],
                );
                s
            } else if let Some(step) = plan.steps.first() {
                let msg = route_task_step(step).await;
                nlu::context::add_to_context(
                    command.to_string(), msg.clone(), format!("intent_router_{}", step.tool), vec![],
                );
                msg
            } else {
                "Not sure how to handle that.".to_string()
            };
            add_log(&summary, LogLevel::Success);
            let _ = core::tts::speak(&summary);
            return Ok(false);
        }
        // Intent router failed — fall through to plugin system, then full reasoning
    }

    // ── Offline mode: sentence splitting for multi-intent ──
    // Split on "and", "then", commas, etc., resolve each segment independently
    if !online::is_online_mode() {
        let segments = nlu::sentence_splitter::split_utterance(command_to_use);
        if segments.len() > 1 {
            add_log(&format!("[Splitter] Split into {} segments", segments.len()), LogLevel::Info);
            let mut all_results: Vec<String> = Vec::new();
            let mut all_ok = true;

            for segment in &segments {
                add_log(&format!("[Splitter] Resolving: {}", segment), LogLevel::Info);
                if let Some(msg) = try_resolve_plugin_segment(segment) {
                    all_results.push(msg);
                } else {
                    all_ok = false;
                    break;
                }
            }

            if all_ok && !all_results.is_empty() {
                let summary = if all_results.len() == 1 {
                    all_results.into_iter().next().unwrap()
                } else {
                    let mut buf = format!("Done. {} tasks.", all_results.len());
                    for (i, r) in all_results.iter().enumerate() {
                        buf.push_str(&format!("\n  {}. {}", i + 1, r));
                    }
                    let _ = core::tts::speak(&format!("I've completed {} tasks.", all_results.len()));
                    buf
                };
                add_log(&summary, LogLevel::Success);
                nlu::context::add_to_context(command.to_string(), summary, "split_plan".to_string(), vec![]);
                return Ok(false);
            }
        }
    }

    // ── Plugin system (keyword fallback for all modes) ──
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

    // Intent recognition: online uses LLM-powered reasoning, offline uses hash-based NLU
    if online::is_online_mode() {
        // Online mode: falls through to full reasoning below
    } else {
        // Offline mode: on-demand NLU init
        if !NLU_READY.load(Ordering::Relaxed) {
            add_log("[NLU] Initializing on demand...", LogLevel::Info);
            if GLOBAL_NLU.initialize().is_ok() {
                NLU_READY.store(true, Ordering::Relaxed);
                add_log("[NLU] NLU engine ready (on-demand)", LogLevel::Success);
            } else {
                add_log("[NLU] On-demand init failed, using basic fallback", LogLevel::Warning);
            }
        }

        // Offline mode: hash-based NLU intent recognition
        let nlu_result = GLOBAL_NLU.process_input(command_to_use);

        if let Ok(ref intent_result) = nlu_result {
            if intent_result.intent_name != "UnknownIntent" {
                add_log(
                    &format!("NLU: {} (confidence: {:.2})", intent_result.intent_name, intent_result.confidence),
                    LogLevel::Info,
                );

                match intent_result.intent_name.as_str() {
                    "assistant_control" => {
                        let cmd_lower = command_to_use.to_lowercase();

                        if cmd_lower.contains("exit")
                            || cmd_lower.contains("quit")
                            || cmd_lower.contains("terminate")
                            || cmd_lower.contains("shutdown assistant")
                            || cmd_lower.contains("close assistant")
                            || cmd_lower.contains("turn off") {
                            add_log("Exit command received - Shutting down IGRIS", LogLevel::Success);
                            let _ = core::tts::speak("Goodbye! Thank you for using IGRIS. See you next time.");
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                            nlu::context::add_to_context(command.to_string(), "Goodbye!".to_string(), "assistant_control".to_string(), vec![]);
                            cleanup_and_exit();
                        }

                        if cmd_lower.contains("sleep")
                            || cmd_lower.contains("standby")
                            || cmd_lower.contains("hibernate") {
                            let _ = core::tts::speak("Okay, going to sleep. Say hello to wake me.");
                            add_log("Entering sleep mode", LogLevel::Info);
                            nlu::context::add_to_context(command.to_string(), "Going to sleep".to_string(), "assistant_control".to_string(), vec![]);
                            return Ok(false);
                        }
                    }
                    "open_app" => {
                        if let Some(app) = intent_result.entities.get("app") {
                            add_log(&format!("Opening app: {}", app), LogLevel::Info);
                        }
                        if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                            if let Ok(response) = plugins::execute_plugin_command(&plugin_result) {
                                let result = handle_plugin_custom_fn(response, "open_app", |app_name| {
                                    match platform::app_launcher::AppLauncherImpl::new().open_app(app_name) {
                                        Ok(m) => { refresh_running_apps(); m }
                                        Err(e) => format!("Failed to open {}: {}", app_name, e),
                                    }
                                });
                                add_log(&result, LogLevel::Success);
                                let _ = core::tts::speak(&result);
                                refresh_running_apps();
                                let entities: Vec<String> = intent_result.entities.values().cloned().collect();
                                nlu::context::add_to_context(command.to_string(), result.clone(), "open_app".to_string(), entities);
                                return Ok(false);
                            }
                        }
                    }
                    "close_app" => {
                        if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                            if let Ok(response) = plugins::execute_plugin_command(&plugin_result) {
                                let result = handle_plugin_custom_fn(response, "close_app", |app_name| {
                                    match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                                        Ok(m) => { refresh_running_apps(); m }
                                        Err(e) => format!("Failed to close {}: {}", app_name, e),
                                    }
                                });
                                add_log(&result, LogLevel::Success);
                                let _ = core::tts::speak(&result);
                                refresh_running_apps();
                                nlu::context::add_to_context(command.to_string(), result.clone(), "close_app".to_string(), intent_result.entities.values().cloned().collect());
                                return Ok(false);
                            }
                        }
                    }
                    "camera_control" => {
                        add_log("Starting camera", LogLevel::Info);
                        match commands::ffmpeg_camera::handle_camera_command("start") {
                            Ok(msg) => { add_log(&msg, LogLevel::Success); let _ = core::tts::speak(&msg); }
                            Err(e) => { add_log(&format!("Camera error: {}", e), LogLevel::Error); let _ = core::tts::speak(&format!("Camera error: {}", e)); }
                        }
                        return Ok(false);
                    }
                    "greeting" => {
                        let _ = core::tts::speak("Hello! How can I help you today?");
                        return Ok(false);
                    }
                    "web_search" => {
                        add_log(&format!("Web search: {}", command), LogLevel::Info);
                        if let Some(response) = commands::web::process_search_command(command).await {
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
                                nlu::context::add_to_context(command.to_string(), "Introduced myself".to_string(), "about_igris".to_string(), vec![]);
                            }
                            Err(e) => { add_log(&format!("About error: {}", e), LogLevel::Error); }
                        }
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
    }

    // Try reasoning: Online NIM or local LLM
    if online::is_online_mode() {
        println!("[Online] Reasoning via NVIDIA NIM...");
        add_log("[Online] Reasoning via NVIDIA NIM...", LogLevel::Info);

        // Build rich context from conversation history + task history + entities
        let context_str = nlu::context::build_llm_context();

        // Timeout for online reasoning: 15s total (API timeout is 60s, but we want
        // to fall back to local faster if the cloud is slow)
        let system_prompt = online::reasoning::online_tool_system_prompt(&context_str);
        let online_fut = online::reason_online(&system_prompt, command_to_use);
        match tokio::time::timeout(std::time::Duration::from_secs(15), online_fut).await {
            Ok(Ok(output)) => {
                ONLINE_FAIL_COUNT.store(0, Ordering::Relaxed);
                println!("[Online] Raw output: {}", &output[..output.len().min(200)]);
                add_log(
                    &format!("[Online] Raw output: {}", &output[..output.len().min(120)]),
                    LogLevel::Info,
                );

                // Try multi-tool plan first, fall back to single tool
                if let Ok(plan) = task_planner::parse_llm_response(&output) {
                    if plan.steps.len() > 1 {
                        add_log(&format!("[Online] Task plan: {} steps", plan.steps.len()), LogLevel::Info);
                        let summary = execute_task_plan(&plan, command_to_use).await;
                        nlu::context::add_to_context(
                            command.to_string(),
                            summary.clone(),
                            "online_task_plan".to_string(),
                            vec![],
                        );
                        return Ok(false);
                    }
                }

                // Single tool fallback
                if let Some((tool, args)) = parse_tool_call(&output) {
                    println!("[Online] Tool: {} | args: {}", tool, args);
                    add_log(
                        &format!("[Online] Tool: {} | args: {}", tool, args),
                        LogLevel::Info,
                    );
                    let response = route_llm_tool(tool, args, command_to_use).await;
                    nlu::context::add_to_context(
                        command.to_string(),
                        response.clone(),
                        format!("online_{}", tool),
                        vec![],
                    );
                    return Ok(false);
                }
            }
            Ok(Err(e)) => {
                println!("[Online] NIM error ({}), trying local fallback", e);
                add_log(&format!("[Online] NIM error ({}), trying local fallback", e), LogLevel::Warning);
                // After 3 consecutive failures, auto-switch to offline mode
                ONLINE_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
                if ONLINE_FAIL_COUNT.load(Ordering::Relaxed) >= 3 {
                    println!("[Online] 3 consecutive failures — auto-switching to offline mode");
                    add_log("[Online] 3 consecutive failures — auto-switching to offline mode", LogLevel::Warning);
                    online::disable_online_mode();
                    let _ = core::tts::speak("Online mode keeps failing. Switching to offline mode.");
                }
            }
            Err(_timeout) => {
                println!("[Online] NIM timed out after 15s — trying local fallback");
                add_log("[Online] NIM timed out — trying local fallback", LogLevel::Warning);
                // Timeouts also count toward the failure threshold
                ONLINE_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
                if ONLINE_FAIL_COUNT.load(Ordering::Relaxed) >= 3 {
                    println!("[Online] 3 consecutive failures — auto-switching to offline mode");
                    add_log("[Online] 3 consecutive failures — auto-switching to offline mode", LogLevel::Warning);
                    online::disable_online_mode();
                    let _ = core::tts::speak("Online mode keeps timing out. Switching to offline mode.");
                }
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

                // Try multi-tool plan first
                if let Ok(plan) = task_planner::parse_llm_response(&output) {
                    if plan.steps.len() > 1 {
                        add_log(&format!("[LocalLLM] Task plan: {} steps", plan.steps.len()), LogLevel::Info);
                        let summary = execute_task_plan(&plan, command_to_use).await;
                        nlu::context::add_to_context(
                            command.to_string(),
                            summary.clone(),
                            "llm_task_plan".to_string(),
                            vec![],
                        );
                        return Ok(false);
                    }
                }

                // Single tool fallback
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
                // First check if this is a tracked web resource (site opened via browser)
                if utils::is_tracked_site(app_name) {
                    match utils::close_site(app_name) {
                        Ok(msg) => {
                            add_log(&msg, LogLevel::Success);
                            let _ = core::tts::speak(&msg);
                            refresh_running_apps();
                            nlu::context::add_to_context(
                                command.to_string(),
                                msg.clone(),
                                "close_site".to_string(),
                                vec![app_name.to_string()],
                            );
                            return Ok(false);
                        }
                        Err(e) => {
                            add_log(&e, LogLevel::Warning);
                            // Fall through to try plugin system
                        }
                    }
                }

                // Execute through plugin system
                if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                    if let Ok(response) = plugins::execute_plugin_command(&plugin_result) {
                        let result = handle_plugin_custom_fn(response, "close_app", |app_name| {
                            match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                                Ok(m) => { refresh_running_apps(); m }
                                Err(e) => format!("Failed to close {}: {}", app_name, e),
                            }
                        });
                        add_log(&result, LogLevel::Success);
                        let _ = core::tts::speak(&result);
                        refresh_running_apps();

                        // Add to context
                        nlu::context::add_to_context(
                            command.to_string(),
                            result.clone(),
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
        if let Some(response) = commands::web::process_search_command(command_to_use).await {
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
        if let Some(answer) = commands::web::search_and_read_results(command_to_use).await {
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
