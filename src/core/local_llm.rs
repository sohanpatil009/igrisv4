#![cfg(feature = "llm")]
use anyhow::{Context, Result};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Mutex;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

/// Lightweight local LLM wrapper around a GGUF model for smart reasoning.
/// Used as a fallback when the existing NLU pipeline has low confidence.
pub struct LocalLlm {
    _backend: LlamaBackend,
    model: LlamaModel,
    ctx: llama_cpp_2::context::LlamaContext<'static>,
    eos_token: LlamaToken,
}

// SAFETY: LlamaModel + LlamaContext are already Send + Sync in the upstream crate.
unsafe impl Send for LocalLlm {}
unsafe impl Sync for LocalLlm {}

impl LocalLlm {
    /// Load a GGUF model from disk.
    pub fn new(model_path: &str) -> Result<Self> {
        if !Path::new(model_path).exists() {
            anyhow::bail!("GGUF model not found at: {}", model_path);
        }

        let backend = LlamaBackend::init().context("Failed to init llama backend")?;

        let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())
            .context("Failed to load GGUF model")?;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048))
            .with_n_threads(num_cpus::get() as i32)
            .with_n_batch(512);

        let ctx: llama_cpp_2::context::LlamaContext<'_> = model
            .new_context(&backend, ctx_params)
            .context("Failed to create llama context")?;

        // SAFETY: model outlives ctx; both are owned by Self.
        let ctx: llama_cpp_2::context::LlamaContext<'static> =
            unsafe { std::mem::transmute(ctx) };

        let eos_token = model.token_eos();

        tracing::info!(
            "[LocalLLM] Loaded GGUF model ({:.2} MB)",
            Path::new(model_path)
                .metadata()
                .map(|m| m.len() as f64 / 1_048_576.0)
                .unwrap_or(0.0)
        );

        Ok(Self {
            _backend: backend,
            model,
            ctx,
            eos_token,
        })
    }

    /// Run inference with a system prompt + user query using ChatML format.
    pub fn generate(&mut self, system_prompt: &str, user_query: &str, max_tokens: u32) -> Result<String> {
        let prompt = format!(
            "<|im_start|>system\n{}\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
            system_prompt, user_query
        );

        let input_tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .context("Failed to tokenize prompt")?;

        let n_input = input_tokens.len();
        let n_ctx = self.ctx.n_ctx() as usize;
        if n_input >= n_ctx {
            anyhow::bail!(
                "Prompt too long ({} tokens, context window {})",
                n_input,
                n_ctx
            );
        }

        let mut output_tokens: Vec<LlamaToken> = Vec::new();
        let max_gen = max_tokens as usize;

        // Feed all input tokens into the KV cache
        let batch_size = 512.min(n_input).max(1);
        let mut batch = LlamaBatch::new(batch_size, 1);

        for i in 0..n_input {
            let is_last = i == n_input - 1;
            batch
                .add(input_tokens[i], i as i32, &[0], is_last)
                .context("Failed to add token to batch")?;

            if batch.n_tokens() as usize >= batch_size || is_last {
                self.ctx
                    .decode(&mut batch)
                    .context("Failed to decode input batch")?;
                batch.clear();
            }
        }

        // Generate new tokens one at a time
        let mut pos = n_input as i32;
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::temp(0.7),
        ]);

        for _ in 0..max_gen {
            let token = sampler.sample(&self.ctx, 0);

            if token == self.eos_token {
                break;
            }

            output_tokens.push(token);

            let mut gen_batch = LlamaBatch::new(1, 1);
            gen_batch
                .add(token, pos, &[0], true)
                .context("Failed to add gen token")?;
            self.ctx
                .decode(&mut gen_batch)
                .context("Failed to decode gen batch")?;

            pos += 1;
        }

        let output = self
            .model
            .tokens_to_str(&output_tokens, Special::Tokenize)
            .context("Failed to decode tokens")?;

        Ok(output.trim().to_string())
    }

    /// Call the LLM for structured tool reasoning.
    pub fn reason(&mut self, system_prompt: &str, user_query: &str) -> Result<String> {
        self.generate(system_prompt, user_query, 256)
    }
}

// ---- Global lazy singleton ----

static LOCAL_LLM: once_cell::sync::Lazy<Mutex<Option<LocalLlm>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// Initialize the global LLM from a model path. Safe to call multiple times.
pub fn init_local_llm(model_path: &str) -> Result<()> {
    let mut guard = LOCAL_LLM.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }
    let llm = LocalLlm::new(model_path)?;
    *guard = Some(llm);
    Ok(())
}

/// Returns true if the LLM has been loaded.
pub fn is_local_llm_ready() -> bool {
    LOCAL_LLM.lock().unwrap().is_some()
}

/// Run reasoning through the global LLM. Returns the raw generated text.
pub fn global_reason(system_prompt: &str, user_query: &str) -> Option<String> {
    let mut guard = LOCAL_LLM.lock().ok()?;
    let llm = guard.as_mut()?;
    llm.reason(system_prompt, user_query).ok()
}

/// Default system prompt describing all IGRIS tools.
pub fn default_tool_system_prompt() -> String {
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

Output ONLY valid JSON with no markdown formatting. Examples:
User: "open chrome"
Assistant: {"tool": "open_app", "arguments": {"app": "chrome"}}

User: "what's the weather like"
Assistant: {"tool": "get_weather", "arguments": {}}

User: "set an alarm for 7am"
Assistant: {"tool": "set_alarm", "arguments": {"time": "7:00"}}

User: "who are you"
Assistant: {"tool": "general_chat", "arguments": {"response": "I am IGRIS, your AI voice assistant powered by a local reasoning model."}}"#.to_string()
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

    let args_start = json_str.find("\"arguments\"")?;
    let args_brace = json_str[args_start..].find('{')? + args_start;
    let args_end = json_str[args_start..].rfind('}')? + args_start;
    let args = json_str[args_brace..=args_end].to_string();

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

    #[test]
    fn test_parse_tool_call_open_app() {
        let output = r#"{"tool": "open_app", "arguments": {"app": "chrome"}}"#;
        let (tool, args) = parse_tool_call(output).unwrap();
        assert_eq!(tool, "open_app");
        assert!(args.contains("chrome"));
    }

    #[test]
    fn test_parse_tool_call_general_chat() {
        let output = r#"{"tool": "general_chat", "arguments": {"response": "Hello!"}}"#;
        let (tool, args) = parse_tool_call(output).unwrap();
        assert_eq!(tool, "general_chat");
        assert!(args.contains("Hello"));
    }

    #[test]
    fn test_parse_tool_call_with_noise() {
        let output = "Let me think... \n\n{\"tool\": \"get_weather\", \"arguments\": {}}\n\nDone!";
        let (tool, args) = parse_tool_call(output).unwrap();
        assert_eq!(tool, "get_weather");
    }

    #[test]
    fn test_parse_tool_call_invalid() {
        assert!(parse_tool_call("I don't know what you mean").is_none());
        assert!(parse_tool_call("").is_none());
    }

    #[test]
    fn test_extract_json_string_field() {
        let json = r#"{"tool": "open_app"}"#;
        assert_eq!(
            extract_json_string_field(json, "tool").as_deref(),
            Some("open_app")
        );
    }
}
