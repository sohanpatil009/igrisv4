use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use igrisv3::online::reasoning::extract_json_string_field;
use igrisv3::nlu::context;
use igrisv3::{commands, core, nlu, online, platform, plugins, utils};

use crate::state::{add_log, LogLevel};
use crate::tools::{handle_plugin_custom_fn, open_url, refresh_running_apps};

type ToolHandler =
    Arc<dyn Fn(HashMap<String, String>, String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

pub struct ToolArg {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub args: &'static [ToolArg],
    pub category: &'static str,
    pub handler: ToolHandler,
}

pub struct ToolRegistry {
    tools: HashMap<&'static str, ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, tool: ToolDef) {
        self.tools.insert(tool.name, tool);
    }

    pub async fn execute(
        &self,
        name: &str,
        args: &str,
        command: &str,
    ) -> Option<String> {
        let tool = self.tools.get(name)?;
        let parsed = parse_args(args, &tool.args);
        let result = (tool.handler)(parsed, command.to_string()).await;
        // Push to undo stack with computed reverse action
        context::push_undo(name, args, &result);
        Some(result)
    }

    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<&ToolDef> {
        let mut v: Vec<&ToolDef> = self.tools.values().collect();
        v.sort_by_key(|t| t.name);
        v
    }

    pub fn describe_all(&self) -> String {
        let mut lines = vec!["I have the following capabilities:".to_string()];
        let mut by_category: HashMap<&str, Vec<&ToolDef>> = HashMap::new();
        for tool in self.tools.values() {
            by_category.entry(tool.category).or_default().push(tool);
        }
        let mut cats: Vec<&&str> = by_category.keys().collect();
        cats.sort();
        for cat in cats {
            let tools = &by_category[cat];
            lines.push(format!("  [{}]", cat));
            for t in tools {
                let args_desc: Vec<String> = t
                    .args
                    .iter()
                    .map(|a| format!("{}: {}", a.name, a.description))
                    .collect();
                let args_str = if args_desc.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", args_desc.join(", "))
                };
                lines.push(format!("    - {}{}", t.description, args_str));
            }
        }
        lines.join("\n")
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().map(|k| k.to_string()).collect()
    }
}

fn parse_args(json: &str, defs: &[ToolArg]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for arg in defs {
        if let Some(val) = extract_json_string_field(json, arg.name) {
            map.insert(arg.name.to_string(), val);
        }
    }
    map
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_TOOL_REGISTRY: Arc<Mutex<ToolRegistry>> = {
        let reg = build_default_registry();
        Arc::new(Mutex::new(reg))
    };
}

fn build_default_registry() -> ToolRegistry {
    let mut reg = ToolRegistry::new();

    reg.register(ToolDef {
        name: "open_app",
        description: "Open any application (chrome, firefox, vscode, spotify, discord, etc.)",
        args: &[ToolArg { name: "app", description: "Application name", required: true }],
        category: "Applications",
        handler: Arc::new(|args: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let app = args.get("app").map(|s| s.as_str()).unwrap_or("");
                if app.is_empty() {
                    return "Which app should I open?".to_string();
                }
                let plugin_cmd = format!("open {}", app);
                if let Some(plugin_result) = plugins::process_plugin_command(&plugin_cmd) {
                    match plugins::execute_plugin_command(&plugin_result) {
                        Ok(msg) => {
                            let result = handle_plugin_custom_fn(msg, "open_app", |app_name| {
                                let name = if app_name.is_empty() { app } else { app_name };
                                match platform::app_launcher::AppLauncherImpl::new().open_app(name)
                                {
                                    Ok(m) => {
                                        refresh_running_apps();
                                        m
                                    }
                                    Err(e) => format!("Failed to open {}: {}", name, e),
                                }
                            });
                            add_log(&result, LogLevel::Success);
                            let _ = core::tts::speak(&result);
                            refresh_running_apps();
                            result
                        }
                        Err(e) => format!("Failed to open {}: {}", app, e),
                    }
                } else {
                    let msg = format!("I couldn't find how to open {}", app);
                    add_log(&msg, LogLevel::Warning);
                    let _ = core::tts::speak(&msg);
                    msg
                }
            })
        }),
    });

    reg.register(ToolDef {
        name: "close_app",
        description: "Close a running application",
        args: &[ToolArg { name: "app", description: "Application name", required: true }],
        category: "Applications",
        handler: Arc::new(|args: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let app = args.get("app").map(|s| s.as_str()).unwrap_or("");
                if app.is_empty() {
                    return "Which app should I close?".to_string();
                }
                if !app.is_empty() && utils::is_tracked_site(app) {
                    let response =
                        utils::close_site(app).unwrap_or_else(|e| format!("Error: {}", e));
                    add_log(&response, LogLevel::Success);
                    let _ = core::tts::speak(&response);
                    return response;
                }
                let plugin_cmd = format!("close {}", app);
                if let Some(plugin_result) = plugins::process_plugin_command(&plugin_cmd) {
                    match plugins::execute_plugin_command(&plugin_result) {
                        Ok(msg) => {
                            let result = handle_plugin_custom_fn(msg, "close_app", |app_name| {
                                let name = if app_name.is_empty() { app } else { app_name };
                                match platform::app_launcher::AppLauncherImpl::new().close_app(name)
                                {
                                    Ok(m) => {
                                        refresh_running_apps();
                                        m
                                    }
                                    Err(e) => format!("Failed to close {}: {}", name, e),
                                }
                            });
                            add_log(&result, LogLevel::Success);
                            let _ = core::tts::speak(&result);
                            refresh_running_apps();
                            result
                        }
                        Err(e) => format!("Failed to close {}: {}", app, e),
                    }
                } else {
                    let msg = format!("I couldn't close {}", app);
                    add_log(&msg, LogLevel::Warning);
                    let _ = core::tts::speak(&msg);
                    msg
                }
            })
        }),
    });

    reg.register(ToolDef {
        name: "close_all_apps",
        description: "Close all windows and applications launched by IGRIS (does not close other running apps)",
        args: &[],
        category: "Applications",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let r = utils::close_all_apps().unwrap_or_else(|e| e);
                refresh_running_apps();
                add_log(&r, LogLevel::Success);
                let _ = core::tts::speak(&r);
                r
            })
        }),
    });

    reg.register(ToolDef {
        name: "close_current_window",
        description: "Close the currently focused window without quitting the app",
        args: &[],
        category: "Applications",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let r = commands::system::close_current_window();
                add_log(&r, LogLevel::Success);
                let _ = core::tts::speak(&r);
                r
            })
        }),
    });

    reg.register(ToolDef {
        name: "close_current_tab",
        description: "Close the currently focused tab (works in browser, editor, file manager, terminal)",
        args: &[],
        category: "Applications",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let r = commands::browser_automation::close_current_tab();
                add_log(&r, LogLevel::Success);
                let _ = core::tts::speak(&r);
                r
            })
        }),
    });

    reg.register(ToolDef {
        name: "switch_previous_tab",
        description: "Switch to the previous tab in the currently focused application",
        args: &[],
        category: "Applications",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let r = commands::browser_automation::switch_previous_tab();
                add_log(&r, LogLevel::Success);
                let _ = core::tts::speak(&r);
                r
            })
        }),
    });

    reg.register(ToolDef {
        name: "switch_previous_window",
        description: "Switch to the previous window of the same application",
        args: &[],
        category: "Applications",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let r = commands::browser_automation::switch_previous_window();
                add_log(&r, LogLevel::Success);
                let _ = core::tts::speak(&r);
                r
            })
        }),
    });

    reg.register(ToolDef {
        name: "search_web",
        description: "Search the web and read results aloud (for facts, questions, news)",
        args: &[ToolArg { name: "query", description: "Search query", required: true }],
        category: "Web",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let query = args.get("query").map(|s| s.as_str()).unwrap_or("");
                let response = commands::web::search_and_read_results(query).await
                    .unwrap_or_else(|| format!("Searched for {}", query));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "browser_search",
        description: "Search for something on a specific site (amazon, youtube, github, wikipedia, reddit, etc.)",
        args: &[
            ToolArg { name: "site", description: "Site to search on (amazon, youtube, github, etc.)", required: true },
            ToolArg { name: "query", description: "Search query", required: true },
        ],
        category: "Web",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let site = args.get("site").map(|s| s.as_str()).unwrap_or("");
                let query = args.get("query").map(|s| s.as_str()).unwrap_or("");
                let (url, msg) = commands::browser_automation::search_on_site(site, query);
                let _ = core::tts::speak(&msg);
                crate::tools::open_url(&url, None);
                msg
            })
        }),
    });

    reg.register(ToolDef {
        name: "browser_type",
        description: "Type text into the currently focused browser search bar or text field",
        args: &[ToolArg { name: "text", description: "Text to type", required: true }],
        category: "Web",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let text = args.get("text").map(|s| s.as_str()).unwrap_or("");
                match commands::browser_automation::type_into_browser(text) {
                    Ok(msg) => {
                        add_log(&msg, LogLevel::Success);
                        let _ = core::tts::speak(&msg);
                        msg
                    }
                    Err(e) => {
                        let fallback = format!("Typed {}", text);
                        add_log(&fallback, LogLevel::Info);
                        fallback
                    }
                }
            })
        }),
    });

    reg.register(ToolDef {
        name: "open_website",
        description: "Open a URL in the specified browser",
        args: &[
            ToolArg { name: "url", description: "URL to open", required: true },
            ToolArg { name: "browser", description: "Browser (chrome, firefox, safari, edge, brave)", required: false },
        ],
        category: "Web",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let url = args.get("url").map(|s| s.as_str()).unwrap_or("");
                let browser = args.get("browser").map(|s| s.as_str());
                let msg = format!("Opening {}...", url);
                let _ = core::tts::speak(&msg);
                open_url(url, browser);
                msg
            })
        }),
    });

    reg.register(ToolDef {
        name: "system_command",
        description: "System control (shutdown, restart, sleep, lock, volume, mute, wifi, bluetooth)",
        args: &[ToolArg { name: "command", description: "System command action", required: true }],
        category: "System",
        handler: Arc::new(|_: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                if let Some(response) = commands::system::process_system_command(&cmd) {
                    add_log(&response, LogLevel::Success);
                    let _ = core::tts::speak(&response);
                    response
                } else {
                    let msg = "System command failed.".to_string();
                    let _ = core::tts::speak(&msg);
                    msg
                }
            })
        }),
    });

    reg.register(ToolDef {
        name: "camera_action",
        description: "Camera operations (photo, video recording)",
        args: &[ToolArg { name: "action", description: "photo, video_start, or video_stop", required: true }],
        category: "Media",
        handler: Arc::new(|args: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let action = args.get("action").map(|s| s.as_str()).unwrap_or("photo");
                if let Some(plugin_result) = plugins::process_plugin_command(&cmd) {
                    if let Ok(msg) = plugins::execute_plugin_command(&plugin_result) {
                        if let Some(camera_action) = msg.strip_prefix("CAMERA_MODE:") {
                            if camera_action.starts_with("ffmpeg_") {
                                let inner = camera_action.strip_prefix("ffmpeg_").unwrap_or(action);
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
                }
                let response = commands::ffmpeg_camera::handle_camera_command(action)
                    .unwrap_or_else(|e| format!("Camera error: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "file_operation",
        description: "File operations (create, delete, open, list, read, write)",
        args: &[
            ToolArg { name: "action", description: "create/delete/open/list/read/write", required: true },
            ToolArg { name: "path", description: "File or folder path", required: true },
            ToolArg { name: "content", description: "Content to write (for write action)", required: false },
        ],
        category: "Files",
        handler: Arc::new(|args: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let action = args.get("action").map(|s| s.as_str()).unwrap_or("");
                let path = args.get("path").map(|s| s.as_str()).unwrap_or("");
                if action == "read" && !path.is_empty() {
                    let response = commands::files::read_text_from_file(path)
                        .unwrap_or_else(|e| format!("Error reading file: {}", e));
                    add_log(&response, LogLevel::Success);
                    let _ = core::tts::speak(&response);
                    response
                } else if action == "write" && !path.is_empty() {
                    let content = args.get("content").map(|s| s.as_str()).unwrap_or("");
                    let response = commands::files::write_text_to_file(path, content)
                        .unwrap_or_else(|e| format!("Error writing file: {}", e));
                    add_log(&response, LogLevel::Success);
                    let _ = core::tts::speak(&response);
                    response
                } else {
                    let _ = core::tts::speak("Processing file command...");
                    if let Some(response) = commands::files::process_file_command_async(&cmd).await
                    {
                        add_log(&response, LogLevel::Success);
                        let _ = core::tts::speak(&response);
                        response
                    } else {
                        let msg = "I couldn't complete that file operation.".to_string();
                        let _ = core::tts::speak(&msg);
                        msg
                    }
                }
            })
        }),
    });

    reg.register(ToolDef {
        name: "read_file",
        description: "Read the contents of a text file",
        args: &[ToolArg { name: "path", description: "File path", required: true }],
        category: "Files",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let path = args.get("path").map(|s| s.as_str()).unwrap_or("");
                let response = commands::files::read_text_from_file(path)
                    .unwrap_or_else(|e| format!("Error reading file: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "write_file",
        description: "Write or overwrite content to a text file. If file exists, it's overwritten.",
        args: &[
            ToolArg { name: "path", description: "File path", required: true },
            ToolArg { name: "content", description: "Content to write", required: true },
        ],
        category: "Files",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let path = args.get("path").map(|s| s.as_str()).unwrap_or("");
                let content = args.get("content").map(|s| s.as_str()).unwrap_or("");
                let exists = std::path::Path::new(path).exists();
                let action = if exists { "Overwrote" } else { "Created" };
                let response = commands::files::write_text_to_file(path, content)
                    .unwrap_or_else(|e| format!("Error writing file: {}", e));
                let msg = format!("{} {} — {}", action, path, response);
                add_log(&msg, LogLevel::Success);
                let _ = core::tts::speak(&msg);
                msg
            })
        }),
    });

    reg.register(ToolDef {
        name: "set_alarm",
        description: "Set an alarm for a specific time",
        args: &[ToolArg { name: "time", description: "Time for the alarm", required: true }],
        category: "Reminders",
        handler: Arc::new(|_: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let response = commands::reminders::handle_alarm_command("alarm_set", &cmd)
                    .unwrap_or_else(|e| format!("Alarm error: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "set_reminder",
        description: "Set a reminder",
        args: &[ToolArg { name: "text", description: "Reminder text", required: true }],
        category: "Reminders",
        handler: Arc::new(|_: HashMap<String, String>, cmd: String| {
            Box::pin(async move {
                let response = commands::reminders::handle_reminder_command("reminder_set", &cmd)
                    .unwrap_or_else(|e| format!("Reminder error: {}", e));
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "get_weather",
        description: "Get weather forecast for any city",
        args: &[ToolArg { name: "location", description: "City name", required: true }],
        category: "Information",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let location = args.get("location").map(|s| s.as_str()).unwrap_or("");
                let response = commands::web::get_weather_via_api(location).await
                    .unwrap_or_else(|| "Weather unavailable.".to_string());
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "tell_fact",
        description: "Tell an interesting random fact",
        args: &[],
        category: "Information",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let response = commands::web::get_random_fact().await
                    .unwrap_or_else(|| "No fact available.".to_string());
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "tell_joke",
        description: "Tell a random joke",
        args: &[],
        category: "Information",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let response = commands::web::get_random_joke().await
                    .unwrap_or_else(|| "No joke available.".to_string());
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "take_screenshot",
        description: "Take a screenshot of the current screen",
        args: &[],
        category: "System",
        handler: Arc::new(|_: HashMap<String, String>, _: String| {
            Box::pin(async move {
                let response = commands::system::take_screenshot();
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "get_system_info",
        description: "Get system information (OS, memory, CPU, IP, uptime)",
        args: &[ToolArg { name: "info", description: "Type: os/memory/cpu/ip/uptime/all", required: false }],
        category: "System",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let info = args.get("info").map(|s| s.as_str()).unwrap_or("all");
                let response = commands::system::get_system_info(info);
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "clipboard_action",
        description: "Read from or write to clipboard",
        args: &[
            ToolArg { name: "action", description: "read or write", required: true },
            ToolArg { name: "text", description: "Text to write (for write action)", required: false },
        ],
        category: "System",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let action = args.get("action").map(|s| s.as_str()).unwrap_or("read");
                let text = args.get("text").map(|s| s.as_str()).unwrap_or("");
                let response = commands::system::clipboard_action(action, text);
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg.register(ToolDef {
        name: "compose_email",
        description: "Compose an email and open in default email client",
        args: &[
            ToolArg { name: "to", description: "Recipient email address", required: true },
            ToolArg { name: "subject", description: "Email subject", required: false },
            ToolArg { name: "body", description: "Email body", required: false },
        ],
        category: "Communication",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let to = args.get("to").map(|s| s.as_str()).unwrap_or("");
                let subject = args.get("subject").map(|s| s.as_str()).unwrap_or("");
                let body = args.get("body").map(|s| s.as_str()).unwrap_or("");
                let to_enc = urlencoding::encode(to);
                let subj_enc = urlencoding::encode(subject);
                let body_enc = urlencoding::encode(body);
                let uri = format!("mailto:{}?subject={}&body={}", to_enc, subj_enc, body_enc);
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&uri).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd").args(["/C", "start", "", &uri]).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&uri).spawn();
                let msg = format!("Opening email to {} about {}", to, subject);
                add_log(&msg, LogLevel::Success);
                let _ = core::tts::speak(&msg);
                msg
            })
        }),
    });

    reg.register(ToolDef {
        name: "generate_code",
        description: "Generate code in a specified language and open in IDE (extension auto-detected from language)",
        args: &[
            ToolArg { name: "language", description: "Programming language (python, rust, js, etc.)", required: true },
            ToolArg { name: "code", description: "Code content", required: true },
            ToolArg { name: "filename", description: "Output filename (extension auto-added if missing)", required: false },
        ],
        category: "Development",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let language = args.get("language").map(|s| s.as_str()).unwrap_or("txt");
                let content = args.get("code").map(|s| s.as_str()).unwrap_or("");
                let raw_filename = args.get("filename").cloned().unwrap_or_else(|| {
                    format!("generated_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"))
                });
                let filename = commands::files::resolve_filename(&raw_filename, Some(language));
                let response = match commands::files::write_text_to_file(&filename, content) {
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
            })
        }),
    });

    reg.register(ToolDef {
        name: "switch_mode",
        description: "Switch between online and offline mode",
        args: &[ToolArg { name: "mode", description: "online or offline", required: true }],
        category: "System",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let mode = args.get("mode").map(|s| s.as_str()).unwrap_or("offline");
                let msg = if mode == "online" {
                    online::enable_online_mode();
                    "Switched to online mode."
                } else {
                    online::disable_online_mode();
                    "Switched to offline mode."
                };
                let _ = core::tts::speak(msg);
                msg.to_string()
            })
        }),
    });

    reg.register(ToolDef {
        name: "general_chat",
        description: "Greetings, farewells, casual chat, and general conversation",
        args: &[ToolArg { name: "response", description: "Friendly response text", required: true }],
        category: "Communication",
        handler: Arc::new(|args: HashMap<String, String>, _cmd: String| {
            Box::pin(async move {
                let response = args
                    .get("response")
                    .cloned()
                    .unwrap_or_else(|| "I'm not sure how to respond to that.".to_string());
                add_log(&response, LogLevel::Success);
                let _ = core::tts::speak(&response);
                response
            })
        }),
    });

    reg
}

pub async fn execute_registered_tool(name: &str, args: &str, command: &str) -> Option<String> {
    let registry = GLOBAL_TOOL_REGISTRY.lock().unwrap();
    registry.execute(name, args, command).await
}

pub fn describe_capabilities() -> String {
    let registry = GLOBAL_TOOL_REGISTRY.lock().unwrap();
    registry.describe_all()
}

pub fn list_tool_names() -> Vec<String> {
    let registry = GLOBAL_TOOL_REGISTRY.lock().unwrap();
    registry.tool_names()
}
