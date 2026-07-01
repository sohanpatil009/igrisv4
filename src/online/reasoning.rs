// src/online/reasoning.rs - Online reasoning using GLM 5.1 via NVIDIA NIM

use crate::config::CONFIG;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone)]
pub struct OnlineReasoning {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    top_p: f32,
    max_tokens: u32,
    seed: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_template_kwargs: Option<ChatTemplateKwargs>,
}

#[derive(Serialize)]
struct ChatTemplateKwargs {
    enable_thinking: bool,
    clear_thinking: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl OnlineReasoning {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("NVIDIA_API_KEY")
            .map_err(|_| "NVIDIA_API_KEY not set in .env")?;

        let base_url = env::var("NVIDIA_NIM_BASE_URL")
            .or_else(|_| env::var("NVIDIA_NIM_GLM_BASE_URL"))
            .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string());

        let model = env::var("NVIDIA_NIM_MODEL")
            .unwrap_or_else(|_| "meta/llama-3.1-8b-instruct".to_string());

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
            api_key,
            base_url,
            model,
        })
    }

    /// Run reasoning through NVIDIA NIM (OpenAI-compatible chat completions)
    pub async fn reason(&self, system_prompt: &str, user_query: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/chat/completions", self.base_url);
        println!("[NIM] POST {}", url);
        println!("[NIM] Model: {} | User query: \"{}\"", self.model, user_query);
        println!("[NIM] System prompt length: {} chars", system_prompt.len());

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_query.to_string(),
            },
        ];

        let chat_template_kwargs = if self.model.contains("glm") {
            Some(ChatTemplateKwargs {
                enable_thinking: true,
                clear_thinking: false,
            })
        } else {
            None
        };

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 1.0,
            top_p: 1.0,
            max_tokens: 16384,
            seed: 42,
            stream: false,
            chat_template_kwargs,
        };

        println!("[NIM] Request: model={}, max_tokens={}, temp={}, top_p={}, seed={}",
            request.model, request.max_tokens, request.temperature, request.top_p, request.seed);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        println!("[NIM] Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("[NIM] ERROR: {}", error_text);
            return Err(format!("NIM API error ({}): {}", status, error_text).into());
        }

        let result: ChatResponse = response.json().await?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        if let Some(usage) = result.usage {
            println!("[NIM] Tokens: {} prompt + {} completion = {} total",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
        }
        println!("[NIM] Response: \"{}\"", &content[..content.len().min(200)]);
        println!("[NIM] Response length: {} chars", content.len());

        Ok(content.trim().to_string())
    }
}

impl Default for OnlineReasoning {
    fn default() -> Self {
        Self::new().expect("Failed to create OnlineReasoning - check NVIDIA_API_KEY in .env")
    }
}

/// Build the system prompt for online reasoning.
/// Includes JARVIS-like personality and conversation context if provided.
pub fn online_tool_system_prompt(conversation_context: &str) -> String {
    let name = CONFIG.assistant_name();
    let personality_desc = match CONFIG.get().personality {
        crate::config::Personality::Igris =>
            "Calm, collected, professional. You serve your master with unwavering loyalty. You are serious but not cold, and you can be witty when appropriate.",
        crate::config::Personality::Alita =>
            "Energetic, friendly, enthusiastic. You are the user's best friend. You are upbeat, encouraging, and always excited to help.",
        _ =>
            "Helpful, knowledgeable, and precise. You assist the user with whatever they need.",
    };

    let mut prompt = format!(
        "You are an AI assistant named {name}. You are highly intelligent, confident, and always ready to help. You speak naturally and conversationally.\n\
        \n\
        Your personality: {personality_desc}\n\
        \n\
        CAPABILITIES:\n\
        You have full control over this computer. You can open/close apps, search the web, control system settings,\n\
        take screenshots, manage files, set reminders, check weather, access the camera, control clipboard, and more.\n\
        \n\
        RULES:\n\
        - NEVER ask clarifying questions. If a request is ambiguous, use your best judgment and pick the most likely tool.\n\
        - Be concise in conversation. Short, natural responses.\n\
        - Use general_chat ONLY for pure greetings, farewells, compliments, thank yous, and chit-chat that has no actionable intent.\n\
        - For weather queries, ALWAYS use get_weather — never search_web and never general_chat.\n\
        - For jokes, ALWAYS use tell_joke — never general_chat or search_web.\n\
        - For facts, ALWAYS use tell_fact — never general_chat or search_web.\n\
        - For questions about facts, news, or information, use search_web.\n\
        - Output ONLY valid JSON. No markdown, no explanation, no extra text.\n\
        \n\
        Your response MUST be valid JSON. For a single action, use:\n\
        {{\"tool\": \"tool_name\", \"args\": {{...}}}}\n\
        For multiple actions (command chaining), use a JSON array:\n\
        [{{\"tool\": \"tool_name\", \"args\": {{...}}}}, {{\"tool\": \"tool_name\", \"args\": {{...}}}}]\n\
        Examples of when to chain:\n\
         - \"search for cats\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"https://www.google.com/search?q=cats\"}}}}\n\
        - \"open chrome and search for cats\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"https://www.google.com/search?q=cats\", \"browser\": \"chrome\"}}}}\n\
        - \"search for weather in London\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"https://www.google.com/search?q=weather+in+london\"}}}}\n\
        - \"open browser and go to youtube\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"https://youtube.com\"}}}}\n\
        - \"open safari and go to youtube\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"https://youtube.com\", \"browser\": \"safari\"}}}}\n\
        - \"write code for a sorting algorithm in python and open it\" → {{\"tool\": \"generate_code\", \"args\": {{\"language\": \"python\", \"code\": \"def bubble_sort(arr):\\n    n = len(arr)\\n    for i in range(n):\\n        for j in range(0, n-i-1):\\n            if arr[j] > arr[j+1]:\\n                arr[j], arr[j+1] = arr[j+1], arr[j]\\n    return arr\", \"filename\": \"sorting.py\"}}}}\n\
        CRITICAL RULE for search+browser requests: NEVER use open_app then search_web as separate steps. ALWAYS use a single open_website with a Google search URL. The browser opening + search is ONE action.\n\
        - \"tell me a joke and take a screenshot\" → [{{\"tool\": \"tell_joke\", \"args\": {{}}}}, {{\"tool\": \"take_screenshot\", \"args\": {{}}}}]\n\
        - \"write a Python script to sort a list and open it\" → [{{\"tool\": \"generate_code\", \"args\": {{\"language\": \"python\", \"code\": \"def sort_list(arr):\\n    return sorted(arr)\\n\\nif __name__ == '__main__':\\n    print(sort_list([3, 1, 2]))\", \"filename\": \"sort_list.py\"}}}}, {{\"tool\": \"open_app\", \"args\": {{\"app\": \"code\"}}}}]\n\
        - \"compose an email to John about the meeting\" → {{\"tool\": \"open_website\", \"args\": {{\"url\": \"mailto:john@example.com?subject=Meeting%20Agenda&body=Hi%20John%2C%0A%0ALet%27s%20discuss%20the%20project%20tomorrow.%0A%0ABest%2C%0AYour%20Name\"}}}}\n\
        \n\
        Available tools:\n\
        \n\
        1. open_app {{\"app\": \"name\"}} — Open any application (chrome, firefox, vscode, spotify, etc.)\n\
        2. close_app {{\"app\": \"name\"}} — Close a running application\n\
        3. close_all_apps {{}} — Close all running applications\n\
         4. search_web {{\"query\": \"...\"}} — Search the web and read results aloud (use for facts, questions, news). Do NOT use for browser searches — use open_website with a Google search URL instead.\n\
          5. open_website {{\"url\": \"...\", \"browser\": \"chrome|firefox|safari|edge|brave\"}} — Open a URL in the specified browser (omit browser to use default). Include browser name if user mentions one. Works with http://, https://, and mailto: URIs. PREFERRED for search queries + browser combos.\n\
        6. system_command {{\"command\": \"shutdown|restart|sleep|lock|volume_up|volume_down|mute\"}} — System control\n\
        7. camera_action {{\"action\": \"photo|video_start|video_stop\"}} — Take a photo or record video\n\
         8. file_operation {{\"action\": \"create|delete|open|list|read|write\", \"path\": \"...\"}} — File and folder operations\n\
        9. set_alarm {{\"time\": \"...\"}} — Set an alarm (e.g. \"7:00 am\")\n\
        10. set_reminder {{\"text\": \"...\"}} — Set a reminder\n\
        11. get_weather {{\"location\": \"city name\"}} — Get current weather for any city. BEST tool for all weather, climate, temperature queries.\n\
        12. tell_fact {{}} — Tell an interesting fact\n\
        13. tell_joke {{}} — Tell a joke\n\
        14. take_screenshot {{}} — Take a screenshot\n\
        15. get_system_info {{\"info\": \"os|memory|cpu|ip|uptime|all\"}} — Get system information\n\
        16. clipboard_action {{\"action\": \"read|write\", \"text\": \"...\"}} — Read or write clipboard\n\
         17. read_file {{\"path\": \"...\"}} — Read the contents of a text file\n\
         18. write_file {{\"path\": \"...\", \"content\": \"...\"}} — Write content to a text file\n\
          19. compose_email {{\"to\": \"...\", \"subject\": \"...\", \"body\": \"...\"}} — Compose an email and open the default email client with fields pre-filled. The body must be URL-encoded. ALWAYS use this for email requests.\n\
          20. generate_code {{\"language\": \"python|rust|javascript|typescript|java|go|cpp|html|css|...\", \"code\": \"...\", \"filename\": \"...\"}} — Generate code in any language and save to a file, then open it in the available IDE (VS Code if installed) or default text editor. The LLM generates the complete code. ALWAYS use this for code generation requests.\n\
         21. general_chat {{\"response\": \"...\"}} — Casual conversation, greetings, farewells\n\
         22. switch_mode {{\"mode\": \"online|offline\"}} — Switch between online and offline mode",
        name = name, personality_desc = personality_desc,
    );

    if !conversation_context.is_empty() {
        prompt.push_str("\n\n---\nRecent conversation context:\n");
        prompt.push_str(conversation_context);
        prompt.push_str("\n---");
    }

    prompt
}

/// Transcribe audio using online Parakeet ASR (NVIDIA NIM)
pub async fn reason_online(system_prompt: &str, user_query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let reasoning = OnlineReasoning::new()?;
    reasoning.reason(system_prompt, user_query).await
}

/// Parse a JSON tool call from the LLM output (single tool).
/// Returns (tool_name, arguments_json_string) with leaked `'static` lifetimes
/// — safe because these are small strings used ephemerally per-request.
pub fn parse_tool_call(output: &str) -> Option<(&'static str, &'static str)> {
    let output = output.trim();

    let start = output.find('{')?;
    let end = output.rfind('}')?;
    let json_str = &output[start..=end];

    let tool = extract_json_string_field(json_str, "tool")?;

    // Try "args" then "arguments" field
    let args = if let Some(args_field) = json_str.find("\"args\"")
        .or_else(|| json_str.find("\"arguments\""))
    {
        let after_field = &json_str[args_field..];
        let args_brace = after_field.find('{')? + args_field;
        let after_brace = &json_str[args_brace + 1..];
        let args_end = after_brace.find('}')? + args_brace + 1;
        json_str[args_brace..=args_end].to_string()
    } else {
        "{}".to_string()
    };

    Some((Box::leak(tool.into_boxed_str()), Box::leak(args.into_boxed_str())))
}

/// Parse one or more tool calls from the LLM output.
/// Supports both single `{"tool":"...","args":{...}}` and
/// multi-tool `[{"tool":"...","args":{...}}, ...]` JSON formats.
pub fn parse_tool_calls(output: &str) -> Vec<(String, String)> {
    let output = output.trim();

    // Multi-tool: JSON array
    if output.starts_with('[') {
        let end = output.rfind(']').unwrap_or(output.len());
        let array_str = &output[..=end];
        let mut results = Vec::new();
        let mut depth = 0;
        let mut obj_start = None;

        for (i, ch) in array_str.char_indices() {
            match ch {
                '{' => {
                    if depth == 0 {
                        obj_start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            let obj = &array_str[start..=i];
                            if let Some((t, a)) = extract_single_call(obj) {
                                results.push((t.to_string(), a.to_string()));
                            }
                        }
                        obj_start = None;
                    }
                }
                _ => {}
            }
        }
        return results;
    }

    // Single tool: try as-is
    if let Some((tool, args)) = parse_tool_call(output) {
        return vec![(tool.to_string(), args.to_string())];
    }

    Vec::new()
}

fn extract_single_call(json: &str) -> Option<(&str, &str)> {
    let tool_start = json.find("\"tool\"")?;
    let after_tool = &json[tool_start..];
    let colon = after_tool.find(':')? + tool_start + 1;
    let val_start = json[colon..].find('"')? + colon + 1;
    let val_end = json[val_start..].find('"')? + val_start;
    let tool = &json[val_start..val_end];

    let args_brace = json.find("\"args\"")
        .or_else(|| json.find("\"arguments\""))?;
    let after_field = &json[args_brace..];
    let brace = after_field.find('{')? + args_brace;
    let rest = &json[brace + 1..];
    let end = rest.find('}')? + brace + 1;
    let args = &json[brace..=end];

    Some((tool, args))
}

pub fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!(r#""{field}"\s*:\s*"(?P<val>[^"]+)""#);
    let re = regex::Regex::new(&pattern).ok()?;
    let caps = re.captures(json)?;
    caps.name("val").map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_online_reasoning() {
        let reasoning = OnlineReasoning::new().unwrap();
        let result = reasoning
            .reason("You are a helpful assistant.", "What is the capital of France?")
            .await;
        println!("Result: {:?}", result);
    }
}