/// Intent Router — lightweight LLM call for intent classification.
///
/// Uses NVIDIA NIM with a compact prompt (no personality, no conversation history)
/// to classify the user's intent into one or more tool calls. This replaces the
/// hash-based NLU when online mode is active, providing real semantic understanding
/// at a fraction of the cost of full LLM reasoning.
///
/// Cost: ~500 tokens vs ~5000+ tokens for full reasoning.
/// Timeout: 15s (matches full reasoning timeout).

use crate::online::task_planner::{parse_llm_response, TaskPlan};

/// Attempt to route a command through the lightweight intent router.
/// Uses the fast model (Gemma 9B) for quick intent classification.
/// Returns a TaskPlan if the LLM responds with valid tool calls, None otherwise.
pub async fn route_intent(command: &str) -> Option<TaskPlan> {
    let system_prompt = intent_router_prompt();
    let timeout = std::time::Duration::from_secs(15);

    let fut = crate::online::reason_online_fast(&system_prompt, command);
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(output)) => {
            let output = output.trim();
            if output.is_empty() {
                println!("[IntentRouter] Empty response");
                return None;
            }
            println!("[IntentRouter] Raw: {}", &output[..output.len().min(300)]);
            match parse_llm_response(output) {
                Ok(plan) => {
                    println!("[IntentRouter] Parsed: {} step(s)", plan.steps.len());
                    Some(plan)
                }
                Err(e) => {
                    println!("[IntentRouter] Parse error: {}", e);
                    None
                }
            }
        }
        Ok(Err(e)) => {
            println!("[IntentRouter] Error: {}", e);
            None
        }
        Err(_) => {
            println!("[IntentRouter] Timed out after 15s");
            None
        }
    }
}

/// Build the compact intent routing system prompt.
/// Much shorter than the full reasoning prompt — no personality, no conversation history.
fn intent_router_prompt() -> String {
    let prompt = r#"You are an intent router. Classify the user's command and output the corresponding tool call(s) as JSON ONLY.

Available tools:
1. open_app {"app": "name"} — Open any application (chrome, firefox, vscode, spotify, discord, etc.)
2. close_app {"app": "name"} — Close a named application by name
3. close_all_apps {} — Close all windows/applications launched by IGRIS (use for "close everything", "close all windows", "close all my apps")
4. close_current_window {} — Close the currently focused window (use for "close this window", "close this", "close the current window")
5. close_current_tab {} — Close the currently focused tab (use for "close this tab", "close this file", "close the current tab", "close tab")
5. search_web {"query": "..."} — Search the web and read results aloud (use for facts, questions, news only)
6. browser_search {"site": "amazon|youtube|github|wikipedia|reddit|...", "query": "..."} — Search for something on a specific site. PREFERRED for "search for X on Y" patterns (amazon, youtube, github, etc.)
7. open_website {"url": "...", "browser": "chrome|firefox|safari|edge|brave"} — Open a URL in the specified browser (omit browser to use default). Include the browser name if the user mentions one. Works with http://, https://, and mailto: URIs. PREFERRED for all browser + search combos.
8. system_command {"command": "shutdown|restart|sleep|lock|volume_up|volume_down|mute"} — System control
9. camera_action {"action": "photo|video_start|video_stop"} — Camera operations
10. file_operation {"action": "create|delete|open|list|read|write", "path": "..."} — File operations
11. set_alarm {"time": "..."} — Set an alarm
12. set_reminder {"text": "..."} — Set a reminder
13. get_weather {"location": "city name"} — Get weather for any city
14. tell_fact {} — Tell an interesting fact
15. tell_joke {} — Tell a joke
16. take_screenshot {} — Take a screenshot
17. get_system_info {"info": "os|memory|cpu|ip|uptime|all"} — System information
18. clipboard_action {"action": "read|write", "text": "..."} — Clipboard operations
19. read_file {"path": "..."} — Read a text file
20. write_file {"path": "...", "content": "..."} — Write a text file
21. compose_email {"to": "...", "subject": "...", "body": "..."} — Compose an email and open the default email client
22. generate_code {"language": "...", "code": "...", "filename": "..."} — Generate code and open in IDE (extension auto-detected from language)
23. general_chat {"response": "..."} — Greetings, farewells, casual chat
24. switch_mode {"mode": "online|offline"} — Switch between online and offline
25. switch_previous_tab {} — Switch to the previous tab (use for "switch tab", "go back a tab", "previous tab", "switch to previous tab")
26. switch_previous_window {} — Switch to the previous window of the same app (use for "switch window", "previous window")

CRITICAL RULES:
- NEVER use open_app + search_web as separate steps for browser searches
- "search for headphones on amazon" → {"tool": "browser_search", "args": {"site": "amazon", "query": "headphones"}}
- "search youtube for rust tutorials" → {"tool": "browser_search", "args": {"site": "youtube", "query": "rust tutorials"}}
- "search for cats" → {"tool": "open_website", "args": {"url": "https://www.google.com/search?q=cats"}}
- "open chrome and search for cats" → {"tool": "open_website", "args": {"url": "https://www.google.com/search?q=cats", "browser": "chrome"}}
- "search for weather in London" → {"tool": "open_website", "args": {"url": "https://www.google.com/search?q=weather+in+london"}}
- "open safari and go to youtube" → {"tool": "open_website", "args": {"url": "https://youtube.com", "browser": "safari"}}
- "compose an email to John about the meeting" → {"tool": "open_website", "args": {"url": "mailto:john@example.com?subject=Meeting&body=Hi%20John"}}
- For weather queries → get_weather (NOT search_web)
- For jokes → tell_joke (NOT general_chat)
- For facts → tell_fact (NOT general_chat)
- For greetings/goodbyes/thanks → general_chat
- NEVER ask clarifying questions. Use your best judgment.
- Output ONLY valid JSON. No markdown, no explanation, no extra text.
- Single action: {"tool": "tool_name", "args": {...}}
- Multiple actions: [{"tool": "tool_name", "args": {...}}, ...]"#;

    prompt.to_string()
}
