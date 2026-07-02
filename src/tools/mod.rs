pub mod registry;
pub mod command_modifier;

use std::process::Command;

use igrisv3::{
    core, nlu, commands, plugins, utils, platform, platform_utils, online,
};
use igrisv3::online::reasoning::extract_json_string_field;
use igrisv3::online::task_planner::{TaskPlan, TaskStep};

use crate::state::{ASSISTANT_STATE, add_log, LogLevel, NLU_READY};
use crate::tools::registry::{execute_registered_tool, GLOBAL_TOOL_REGISTRY};

pub async fn route_llm_tool(tool: &str, args: &str, command_to_use: &str) -> String {
    println!("[ToolRouter] LLM called tool: \"{}\" with args: \"{}\"", tool, args);
    match execute_registered_tool(tool, args, command_to_use).await {
        Some(response) => response,
        None => {
            let response = if tool == "general_chat" {
                extract_chat_response(args).unwrap_or_else(|| "I'm not sure how to respond to that.".to_string())
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
    let mut inherited_params: std::collections::HashMap<String, String> = std::collections::HashMap::new();

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

        // Parameter inheritance: fill missing args from context (previous steps, entity pool)
        let enriched_args = enrich_step_args(step, &inherited_params, i).await;

        let step_msg = if let Some(response) = execute_registered_tool(&step.tool, &enriched_args, original_command).await {
            response
        } else {
            format!("Unknown tool: {}", step.tool)
        };
        add_log(&format!("[Task] Result: {}", step_msg), LogLevel::Success);

        // Inherit params from this step for next steps
        inherit_from_step(&step.tool, &enriched_args, &step_msg, &mut inherited_params);

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

async fn enrich_step_args(step: &TaskStep, inherited: &std::collections::HashMap<String, String>, step_index: usize) -> String {
    let mut args = step.args.clone();

    // Inherit known entities from context if step is missing key params
    let known_keys = ["app", "browser", "location", "query", "path", "url", "file"];
    for key in &known_keys {
        if !args.contains(key) && inherited.contains_key(*key) {
            if let Some(val) = inherited.get(*key) {
                // Only inject if the key makes sense for this tool
                if param_relevant_for_tool(&step.tool, key) {
                    // Inject into JSON args
                    args = inject_json_field(&args, key, val);
                    add_log(&format!("[Inherit] Injected {}={} into step {}", key, val, step_index + 1), LogLevel::Info);
                }
            }
        }
    }

    args
}

fn param_relevant_for_tool(tool: &str, param: &str) -> bool {
    matches!(
        (tool, param),
        ("open_app", "app")
            | ("close_app", "app")
            | ("search_web", "query")
            | ("open_website", "url")
            | ("open_website", "browser")
            | ("get_weather", "location")
            | ("file_operation", "path")
            | ("read_file", "path")
            | ("write_file", "path")
    )
}

fn inject_json_field(json: &str, key: &str, value: &str) -> String {
    if json.trim() == "{}" {
        format!("{{\"{}\":\"{}\"}}", key, value)
    } else if !json.contains(&format!("\"{}\"", key)) {
        // Insert before closing brace
        if let Some(pos) = json.rfind('}') {
            let prefix = &json[..pos].trim_end();
            let needs_comma = prefix.ends_with('"') || prefix.ends_with('}');
            format!("{}{}\"{}\":\"{}\"\n}}", prefix, if needs_comma { ",\n  " } else { "\n  " }, key, value)
        } else {
            json.to_string()
        }
    } else {
        json.to_string()
    }
}

fn inherit_from_step(tool: &str, args: &str, _result: &str, pool: &mut std::collections::HashMap<String, String>) {
    // Extract key params from the step args and add them to the inheritance pool
    let relevant_fields = match tool {
        "open_app" => &["app"][..],
        "open_website" => &["url", "browser"],
        "search_web" => &["query"],
        "get_weather" => &["location"],
        "set_alarm" => &["time"],
        "set_reminder" => &["text"],
        "file_operation" | "read_file" | "write_file" => &["path"],
        _ => &[],
    };
    for field in relevant_fields {
        if let Some(val) = extract_json_string_field(args, field) {
            pool.insert(field.to_string(), val);
        }
    }
}

pub async fn route_task_step(step: &TaskStep) -> String {
    execute_registered_tool(&step.tool, &step.args, &step.description).await
        .unwrap_or_else(|| {
            extract_json_string_field(&step.args, "response")
                .unwrap_or_else(|| "Done.".to_string())
        })
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
