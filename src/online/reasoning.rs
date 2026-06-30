// src/online/reasoning.rs - Online reasoning using GLM 5.1 via NVIDIA NIM

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

/// Default system prompt for online reasoning
pub fn online_tool_system_prompt() -> String {
    r#"You are a command routing assistant for IGRIS voice assistant.
Your job is to analyse the user's request and output a single JSON object
choosing the most appropriate tool and its arguments.

Available tools:

1. open_app {"app": "name"} — Open an application or browser
2. close_app {"app": "name"} — Close an application
3. close_all_apps {} — Close all running applications
4. search_web {"query": "..."} — Search the web
5. open_website {"url": "..."} — Open a specific website
6. system_command {"command": "shutdown|restart|sleep|lock|volume_up|volume_down|mute"} — System control
7. camera_action {"action": "photo|video_start|video_stop|switch"} — Camera control
8. file_operation {"action": "create|delete|open|list", "path": "..."} — File operations
9. set_alarm {"time": "..."} — Set an alarm
10. set_reminder {"text": "..."} — Set a reminder
11. get_weather {} — Get current weather
12. tell_fact {} — Get an interesting fact
13. tell_joke {} — Tell a joke
14. general_chat {"response": "..."} — General conversation / anything else
15. switch_mode {"mode": "online|offline"} — Switch between online and offline mode

Output ONLY valid JSON with no markdown formatting."#.to_string()
}

/// Transcribe audio using online Parakeet ASR (NVIDIA NIM)
pub async fn reason_online(system_prompt: &str, user_query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let reasoning = OnlineReasoning::new()?;
    reasoning.reason(system_prompt, user_query).await
}

/// Parse a JSON tool call from the LLM output.
/// Returns (tool_name, arguments_json_string) with leaked `'static` lifetimes
/// — safe because these are small strings used ephemerally per-request.
pub fn parse_tool_call(output: &str) -> Option<(&'static str, &'static str)> {
    let output = output.trim();

    let start = output.find('{')?;
    let end = output.rfind('}')?;
    let json_str = &output[start..=end];

    let tool = extract_json_string_field(json_str, "tool")?;

    // Try "args" (GLM 5.1) then "arguments" (local LLM)
    let args = if let Some(args_field) = json_str.find("\"args\"")
        .or_else(|| json_str.find("\"arguments\""))
    {
        let args_brace = json_str[args_field..].find('{')? + args_field;
        let args_end = json_str[args_field..].rfind('}')? + args_field;
        json_str[args_brace..=args_end].to_string()
    } else {
        "{}".to_string()
    };

    Some((Box::leak(tool.into_boxed_str()), Box::leak(args.into_boxed_str())))
}

fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
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