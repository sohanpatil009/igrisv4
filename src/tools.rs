use std::process::Command;

use igrisv3::{
    core, nlu, commands, plugins, utils, platform, platform_utils, online,
};
use igrisv3::online::reasoning::extract_json_string_field;
use igrisv3::online::task_planner::{TaskPlan, TaskStep};

use crate::state::{ASSISTANT_STATE, add_log, LogLevel, NLU_READY};

pub async fn route_llm_tool(tool: &str, _args: &str, command_to_use: &str) -> String {
    println!("[ToolRouter] LLM called tool: \"{}\" with args: \"{}\"", tool, _args);
    match tool {
        "open_app" => {
            if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                match plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        let result = handle_plugin_custom_fn(msg, "open_app", |app_name| {
                            match platform::app_launcher::AppLauncherImpl::new().open_app(app_name) {
                                Ok(m) => { refresh_running_apps(); m }
                                Err(e) => format!("Failed to open {}: {}", app_name, e),
                            }
                        });
                        add_log(&result, LogLevel::Success);
                        let _ = core::tts::speak(&result);
                        refresh_running_apps();
                        result
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
            let app = extract_json_string_field(_args, "app").unwrap_or_default();
            // Check if it's a tracked web resource first
            if !app.is_empty() && utils::is_tracked_site(&app) {
                let response = utils::close_site(&app).unwrap_or_else(|e| format!("Error: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            } else if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                match plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        let result = handle_plugin_custom_fn(msg, "close_app", |app_name| {
                            match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                                Ok(m) => { refresh_running_apps(); m }
                                Err(e) => format!("Failed to close {}: {}", app_name, e),
                            }
                        });
                        add_log(&result, LogLevel::Success);
                        let _ = core::tts::speak(&result);
                        refresh_running_apps();
                        result
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
            let response = commands::web::search_and_read_results(command_to_use).await
                .unwrap_or_else(|| "Search failed.".to_string());
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "open_website" => {
            let url = extract_json_string_field(_args, "url")
                .unwrap_or_else(|| command_to_use.to_string());
            let browser = extract_json_string_field(_args, "browser");
            let response = format!("Opening {}...", url);
            let _ = core::tts::speak(&response);
            open_url(&url, browser.as_deref());
            response
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
            let action = extract_json_string_field(_args, "action").unwrap_or_else(|| "photo".to_string());
            // Try plugin system first, then fallback to direct command
            if let Some(plugin_result) = plugins::process_plugin_command(command_to_use) {
                match plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        if let Some(camera_action) = msg.strip_prefix("CAMERA_MODE:") {
                            if camera_action.starts_with("ffmpeg_") {
                                let inner = camera_action.strip_prefix("ffmpeg_").unwrap_or("");
                                let response = commands::ffmpeg_camera::handle_camera_command(inner)
                                    .unwrap_or_else(|e| format!("Camera error: {}", e));
                                add_log(&response, LogLevel::Success);
                                let _ = core::tts::speak(&response);
                                return response;
                            }
                        }
                        add_log(&msg, LogLevel::Success);
                        let _ = core::tts::speak(&msg);
                        return msg;
                    }
                    Err(_) => {}
                }
            }
            let response = commands::ffmpeg_camera::handle_camera_command(&action)
                .unwrap_or_else(|e| format!("Camera error: {}", e));
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "file_operation" => {
            let action = extract_json_string_field(_args, "action").unwrap_or_default();
            let path = extract_json_string_field(_args, "path").unwrap_or_default();
            if action == "read" && !path.is_empty() {
                let response = commands::files::read_text_from_file(&path)
                    .unwrap_or_else(|e| format!("Error reading file: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            } else if action == "write" && !path.is_empty() {
                let content = extract_json_string_field(_args, "content").unwrap_or_default();
                let response = commands::files::write_text_to_file(&path, &content)
                    .unwrap_or_else(|e| format!("Error writing file: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            } else {
                let _ = core::tts::speak("Processing file command...");
                if let Some(response) = commands::files::process_file_command_async(command_to_use).await {
                    add_log(&response, LogLevel::Success);
                    let _ = core::tts::speak(&response);
                    response
                } else {
                    let response = "I couldn't complete that file operation.";
                    let _ = core::tts::speak(response);
                    response.to_string()
                }
            }
        }
        "read_file" => {
            let path = extract_json_string_field(_args, "path").unwrap_or_default();
            let response = commands::files::read_text_from_file(&path)
                .unwrap_or_else(|e| format!("Error reading file: {}", e));
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "write_file" => {
            let path = extract_json_string_field(_args, "path").unwrap_or_default();
            let content = extract_json_string_field(_args, "content").unwrap_or_default();
            let response = commands::files::write_text_to_file(&path, &content)
                .unwrap_or_else(|e| format!("Error writing file: {}", e));
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
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
            let location = extract_json_string_field(_args, "location").unwrap_or_default();
            add_log(&format!("[Weather] Fetching weather via wttr.in for: '{}'", location), LogLevel::Info);
            let response = commands::web::get_weather_via_api(&location).await
                .unwrap_or_else(|| {
                    let fallback = "I couldn't find weather for that location.";
                    add_log(fallback, LogLevel::Warning);
                    fallback.to_string()
                });
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "tell_fact" => {
            let response = commands::web::get_random_fact().await
                .unwrap_or_else(|| {
                    "I couldn't find a fact right now.".to_string()
                });
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "tell_joke" => {
            let response = commands::web::get_random_joke().await
                .unwrap_or_else(|| {
                    "I couldn't think of a joke right now.".to_string()
                });
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "take_screenshot" => {
            let response = commands::system::take_screenshot();
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "get_system_info" => {
            let info_type = extract_json_string_field(_args, "info").unwrap_or_default();
            let response = commands::system::get_system_info(&info_type);
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "clipboard_action" => {
            let action = extract_json_string_field(_args, "action").unwrap_or_default();
            let text = extract_json_string_field(_args, "text").unwrap_or_default();
            let response = commands::system::clipboard_action(&action, &text);
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "compose_email" => {
            let to = extract_json_string_field(_args, "to").unwrap_or_default();
            let subject = extract_json_string_field(_args, "subject").unwrap_or_default();
            let body = extract_json_string_field(_args, "body").unwrap_or_default();
            let to_enc = urlencoding::encode(&to);
            let subj_enc = urlencoding::encode(&subject);
            let body_enc = urlencoding::encode(&body);
            let uri = format!("mailto:{}?subject={}&body={}", to_enc, subj_enc, body_enc);
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&uri).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd").args(["/C", "start", "", &uri]).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&uri).spawn();
            let response = format!("Opening email to {} about {}", to, subject);
            add_log(&response, LogLevel::Success);
            let _ = core::tts::speak(&response);
            response
        }
        "generate_code" => {
            let language = extract_json_string_field(_args, "language").unwrap_or_else(|| "txt".to_string());
            let content = extract_json_string_field(_args, "code")
                .or_else(|| extract_json_string_field(_args, "content"))
                .unwrap_or_default();
            let filename = extract_json_string_field(_args, "filename")
                .unwrap_or_else(|| format!("generated_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), language));
            let response = match commands::files::write_text_to_file(&filename, &content) {
                Ok(msg) => {
                    let opened = std::process::Command::new("code")
                        .arg(&filename)
                        .spawn()
                        .is_ok();
                    if !opened {
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("notepad").arg(&filename).spawn();
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").args(["-e", &filename]).spawn();
                        #[cfg(target_os = "linux")]
                        let _ = std::process::Command::new("gedit").arg(&filename).spawn()
                            .or_else(|_| std::process::Command::new("xdg-open").arg(&filename).spawn());
                    }
                    format!("{} ({} file)", msg, language)
                }
                Err(e) => format!("Error writing code: {}", e),
            };
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

pub async fn execute_task_plan(plan: &TaskPlan, original_command: &str) -> String {
    let mut results: Vec<String> = Vec::new();

    for (i, step) in plan.steps.iter().enumerate() {
        let step_desc = if step.description.is_empty() {
            format!("{}", step.tool)
        } else {
            step.description.clone()
        };
        add_log(
            &format!("[Task {}/{}] {} — {}", i + 1, plan.steps.len(), step.tool, step_desc),
            LogLevel::Info,
        );

        let step_msg = route_task_step(step).await;
        add_log(&format!("[Task] Result: {}", step_msg), LogLevel::Success);

        nlu::context::add_task_step(
            step.tool.clone(),
            original_command.to_string(),
            step_msg.clone(),
        );

        results.push(step_msg);
    }

    let summary = if results.len() == 1 {
        results.into_iter().next().unwrap()
    } else {
        let mut buf = format!("Done. {} tasks completed.", results.len());
        for (i, r) in results.iter().enumerate() {
            buf.push_str(&format!("\n  {}. {}", i + 1, r));
        }
        let spoken = format!("I've completed {} tasks.", results.len());
        let _ = core::tts::speak(&spoken);
        buf
    };

    add_log(&summary, LogLevel::Success);
    summary
}

pub async fn route_task_step(step: &TaskStep) -> String {
    match step.tool.as_str() {
        "open_app" => {
            let app = extract_json_string_field(&step.args, "app")
                .unwrap_or_else(|| step.description.clone());
            let cmd = format!("open {}", app);
            if let Some(plugin_result) = plugins::process_plugin_command(&cmd) {
                match plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => handle_plugin_custom_fn(msg, "open_app", |app_name| {
                        match platform::app_launcher::AppLauncherImpl::new().open_app(app_name) {
                            Ok(m) => { refresh_running_apps(); m }
                            Err(e) => format!("Failed to open {}: {}", app_name, e),
                        }
                    }),
                    Err(e) => format!("Failed to open {}: {}", app, e),
                }
            } else {
                format!("Couldn't find how to open {}", app)
            }
        }
        "close_app" => {
            let app = extract_json_string_field(&step.args, "app")
                .unwrap_or_else(|| step.description.clone());
            if utils::is_tracked_site(&app) {
                utils::close_site(&app).unwrap_or_else(|e| format!("Error: {}", e))
            } else {
                let cmd = format!("close {}", app);
                if let Some(plugin_result) = plugins::process_plugin_command(&cmd) {
                    match plugins::execute_plugin_command(&plugin_result) {
                        Ok(msg) => handle_plugin_custom_fn(msg, "close_app", |app_name| {
                            match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                                Ok(m) => { refresh_running_apps(); m }
                                Err(e) => format!("Failed to close {}: {}", app_name, e),
                            }
                        }),
                        Err(e) => format!("Failed to close {}: {}", app, e),
                    }
                } else {
                    format!("Couldn't close {}", app)
                }
            }
        }
        "close_all_apps" => {
            let r = commands::app_utils::close_all_apps().unwrap_or_default();
            refresh_running_apps();
            r
        }
        "search_web" => {
            let query = extract_json_string_field(&step.args, "query")
                .unwrap_or_else(|| step.description.clone());
            commands::web::search_and_read_results(&query).await
                .unwrap_or_else(|| format!("Searched for {}", query))
        }
        "open_website" => {
            let url = extract_json_string_field(&step.args, "url")
                .unwrap_or_else(|| step.description.clone());
            let browser = extract_json_string_field(&step.args, "browser");
            open_url(&url, browser.as_deref());
            format!("Opened {}", url)
        }
        "system_command" => {
            let cmd = extract_json_string_field(&step.args, "command")
                .unwrap_or_else(|| step.description.clone());
            commands::system::process_system_command(&cmd)
                .unwrap_or_else(|| format!("System command: {}", cmd))
        }
        "camera_action" => {
            let action = extract_json_string_field(&step.args, "action")
                .unwrap_or_else(|| "photo".to_string());
            let cmd = format!("camera {}", action);
            if let Some(plugin_result) = plugins::process_plugin_command(&cmd) {
                match plugins::execute_plugin_command(&plugin_result) {
                    Ok(msg) => {
                        if let Some(camera_action) = msg.strip_prefix("CAMERA_MODE:") {
                            if camera_action.starts_with("ffmpeg_") {
                                let inner = camera_action.strip_prefix("ffmpeg_").unwrap_or("");
                                commands::ffmpeg_camera::handle_camera_command(inner)
                                    .unwrap_or_else(|e| format!("Camera error: {}", e))
                            } else {
                                msg
                            }
                        } else {
                            msg
                        }
                    }
                    Err(_) => commands::ffmpeg_camera::handle_camera_command(&action)
                        .unwrap_or_else(|e| format!("Camera error: {}", e)),
                }
            } else {
                commands::ffmpeg_camera::handle_camera_command(&action)
                    .unwrap_or_else(|e| format!("Camera error: {}", e))
            }
        }
        "file_operation" => {
            let action = extract_json_string_field(&step.args, "action").unwrap_or_default();
            let path = extract_json_string_field(&step.args, "path")
                .unwrap_or_else(|| step.description.clone());
            match action.as_str() {
                "read" => {
                    commands::files::read_text_from_file(&path)
                        .unwrap_or_else(|e| format!("Error reading {}: {}", path, e))
                }
                "write" => {
                    let content = extract_json_string_field(&step.args, "content").unwrap_or_default();
                    commands::files::write_text_to_file(&path, &content)
                        .unwrap_or_else(|e| format!("Error writing {}: {}", path, e))
                }
                _ => commands::files::process_file_command_async(&path).await
                    .unwrap_or_else(|| format!("File operation on {}", path))
            }
        }
        "read_file" => {
            let path = extract_json_string_field(&step.args, "path")
                .unwrap_or_else(|| step.description.clone());
            commands::files::read_text_from_file(&path)
                .unwrap_or_else(|e| format!("Error reading {}: {}", path, e))
        }
        "write_file" => {
            let path = extract_json_string_field(&step.args, "path")
                .unwrap_or_else(|| step.description.clone());
            let content = extract_json_string_field(&step.args, "content").unwrap_or_default();
            commands::files::write_text_to_file(&path, &content)
                .unwrap_or_else(|e| format!("Error writing {}: {}", path, e))
        }
        "set_alarm" => {
            commands::reminders::handle_alarm_command("alarm_set", &step.description)
                .unwrap_or_else(|e| format!("Alarm error: {}", e))
        }
        "set_reminder" => {
            commands::reminders::handle_reminder_command("reminder_set", &step.description)
                .unwrap_or_else(|e| format!("Reminder error: {}", e))
        }
        "get_weather" => {
            let location = extract_json_string_field(&step.args, "location")
                .unwrap_or_default();
            commands::web::get_weather_via_api(&location).await
                .unwrap_or_else(|| "Weather unavailable.".to_string())
        }
        "tell_fact" => {
            commands::web::get_random_fact().await
                .unwrap_or_else(|| "No fact available.".to_string())
        }
        "tell_joke" => {
            commands::web::get_random_joke().await
                .unwrap_or_else(|| "No joke available.".to_string())
        }
        "take_screenshot" => commands::system::take_screenshot(),
        "get_system_info" => {
            let info = extract_json_string_field(&step.args, "info")
                .unwrap_or_else(|| "all".to_string());
            commands::system::get_system_info(&info)
        }
        "clipboard_action" => {
            let action = extract_json_string_field(&step.args, "action")
                .unwrap_or_else(|| "read".to_string());
            let text = extract_json_string_field(&step.args, "text")
                .unwrap_or_default();
            commands::system::clipboard_action(&action, &text)
        }
        "switch_mode" => {
            let mode = extract_json_string_field(&step.args, "mode")
                .unwrap_or_else(|| "offline".to_string());
            if mode == "online" {
                online::enable_online_mode();
                "Switched to online mode.".to_string()
            } else {
                online::disable_online_mode();
                "Switched to offline mode.".to_string()
            }
        }
        "compose_email" => {
            let to = extract_json_string_field(&step.args, "to").unwrap_or_default();
            let subject = extract_json_string_field(&step.args, "subject").unwrap_or_default();
            let body = extract_json_string_field(&step.args, "body").unwrap_or_default();
            let to_enc = urlencoding::encode(&to);
            let subj_enc = urlencoding::encode(&subject);
            let body_enc = urlencoding::encode(&body);
            let uri = format!("mailto:{}?subject={}&body={}", to_enc, subj_enc, body_enc);
            #[cfg(target_os = "macos")]
            let _ = Command::new("open").arg(&uri).spawn();
            #[cfg(target_os = "windows")]
            let _ = Command::new("cmd").args(["/C", "start", "", &uri]).spawn();
            #[cfg(target_os = "linux")]
            let _ = Command::new("xdg-open").arg(&uri).spawn();
            format!("Opening email to {} about {}", to, subject)
        }
        "generate_code" => {
            let language = extract_json_string_field(&step.args, "language").unwrap_or_else(|| "txt".to_string());
            let content = extract_json_string_field(&step.args, "code")
                .or_else(|| extract_json_string_field(&step.args, "content"))
                .unwrap_or_default();
            let filename = extract_json_string_field(&step.args, "filename")
                .unwrap_or_else(|| format!("generated_{}.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"), language));
            match commands::files::write_text_to_file(&filename, &content) {
                Ok(msg) => {
                    let opened = Command::new("code")
                        .arg(&filename)
                        .spawn()
                        .is_ok();
                    if !opened {
                        #[cfg(target_os = "windows")]
                        let _ = Command::new("notepad").arg(&filename).spawn();
                        #[cfg(target_os = "macos")]
                        let _ = Command::new("open").args(["-e", &filename]).spawn();
                        #[cfg(target_os = "linux")]
                        let _ = Command::new("gedit").arg(&filename).spawn()
                            .or_else(|_| Command::new("xdg-open").arg(&filename).spawn());
                    }
                    format!("{} ({} file)", msg, language)
                }
                Err(e) => format!("Error writing code: {}", e),
            }
        }
        "general_chat" | _ => {
            extract_json_string_field(&step.args, "response")
                .unwrap_or_else(|| "Done.".to_string())
        }
    }
}

pub fn handle_plugin_custom_fn<F>(response: String, action_prefix: &str, handler: F) -> String
where
    F: FnOnce(&str) -> String,
{
    if let Some(action) = response.strip_prefix("CUSTOM_FN:") {
        let prefix_with_colon = format!("{}:", action_prefix);
        if action == action_prefix {
            handler("")
        } else if let Some(arg) = action.strip_prefix(&prefix_with_colon) {
            handler(arg)
        } else {
            response
        }
    } else {
        response
    }
}

pub fn try_resolve_plugin_segment(segment: &str) -> Option<String> {
    let plugin_result = plugins::process_plugin_command(segment)?;
    let response = plugins::execute_plugin_command(&plugin_result).ok()?;

    match response.as_str() {
        r if r.starts_with("CUSTOM_FN:") => {
            let action = r.strip_prefix("CUSTOM_FN:")?;
            if action.starts_with("open_app:") {
                let app_name = action.strip_prefix("open_app:")?;
                match platform::app_launcher::AppLauncherImpl::new().open_app(app_name) {
                    Ok(msg) => { refresh_running_apps(); Some(msg) }
                    Err(e) => Some(format!("Failed to open {}: {}", app_name, e)),
                }
            } else if action.starts_with("close_app:") {
                let app_name = action.strip_prefix("close_app:")?;
                match platform::app_launcher::AppLauncherImpl::new().close_app(app_name) {
                    Ok(msg) => { refresh_running_apps(); Some(msg) }
                    Err(e) => Some(format!("Failed to close {}: {}", app_name, e)),
                }
            } else if action == "close_all_apps" {
                Some(commands::app_utils::close_all_apps().unwrap_or_default())
            } else if action.starts_with("system_") {
                commands::system::process_system_command(segment)
            } else if action.starts_with("alarm_") {
                commands::reminders::handle_alarm_command(action, segment).ok()
            } else if action.starts_with("reminder_") {
                commands::reminders::handle_reminder_command(action, segment).ok()
            } else {
                None
            }
        }
        r if r.starts_with("CAMERA_MODE:") => None,
        _ => Some(response.to_string()),
    }
}

pub fn extract_chat_response(args: &str) -> Option<String> {
    let re = regex::Regex::new(r#""response"\s*:\s*"([^"]+)""#).ok()?;
    let caps = re.captures(args)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

fn open_url(url: &str, browser: Option<&str>) {
    let browser_cmd = browser.and_then(|b| match b.to_lowercase().as_str() {
        "chrome" | "google chrome" => {
            Some(match std::env::consts::OS {
                "macos" => "Google Chrome",
                "windows" => "chrome",
                _ => "google-chrome",
            })
        }
        "firefox" => Some(match std::env::consts::OS {
            "macos" => "Firefox",
            _ => "firefox",
        }),
        "edge" | "microsoft edge" => Some(match std::env::consts::OS {
            "macos" => "Microsoft Edge",
            "windows" => "msedge",
            _ => "microsoft-edge",
        }),
        "safari" if cfg!(target_os = "macos") => Some("Safari"),
        "brave" => Some(match std::env::consts::OS {
            "macos" => "Brave Browser",
            "windows" => "brave",
            _ => "brave-browser",
        }),
        _ => None,
    });

    if let Some(cmd) = browser_cmd {
        #[cfg(target_os = "macos")]
        let _ = Command::new("open").args(["-a", cmd, url]).spawn();
        #[cfg(target_os = "windows")]
        let _ = Command::new(cmd).arg(url).spawn();
        #[cfg(target_os = "linux")]
        let _ = Command::new(cmd).arg(url).spawn();
    } else {
        #[cfg(target_os = "macos")]
        let _ = Command::new("open").arg(url).spawn();
        #[cfg(target_os = "windows")]
        let _ = Command::new("cmd").args(["/C", "start", url]).spawn();
        #[cfg(target_os = "linux")]
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

pub fn cleanup_and_exit() -> ! {
    println!("[CLEANUP] Closing all apps opened by IGRIS...");
    match utils::close_all_apps() {
        Ok(msg) => println!("[CLEANUP] {}", msg),
        Err(e) => println!("[CLEANUP] Error: {}", e),
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}

pub fn refresh_running_apps() {
    let tracked_apps: Vec<String> = if let Ok(mut tracker) = utils::PROCESS_TRACKER.lock() {
        tracker.cleanup();
        tracker.get_by_category(utils::ProcessCategory::App)
            .iter()
            .filter(|p| tracker.is_running(&p.exe_name))
            .map(|p| p.name.clone())
            .collect()
    } else {
        Vec::new()
    };

    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.running_apps = tracked_apps;
}
