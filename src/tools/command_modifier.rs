use std::collections::HashMap;

use igrisv3::nlu::context;
use igrisv3::online::reasoning::extract_json_string_field;

/// Detect if the user is modifying/redirecting a previous command.
/// Returns a `Modification` describing what to change.
#[derive(Debug, Clone)]
pub enum Modification {
    /// Replace entire last command with new text
    Replace(String),
    /// Change a specific parameter of the last command
    ChangeParam { param: String, value: String },
    /// Undo the last action
    Undo,
    /// Retry the last action
    Retry,
    /// No modification detected
    None,
}

/// Check if the user input is a modification of a previous command.
pub fn detect_modification(input: &str) -> Modification {
    let lower = input.to_lowercase().trim().to_string();

    // === Undo patterns ===
    if matches_undo(&lower) {
        return Modification::Undo;
    }

    // === Retry patterns ===
    if matches_retry(&lower) {
        return Modification::Retry;
    }

    // === "actually" replacement patterns ===
    if let Some(rest) = strip_prefix_any(&lower, &[
        "actually ",
        "actually, ",
        "actually do ",
        "actually, do ",
    ]) {
        if !rest.is_empty() {
            return Modification::Replace(rest.to_string());
        }
    }

    // === "no/nope" replacement patterns ===
    if let Some(rest) = strip_prefix_any(&lower, &[
        "no i meant ",
        "no, i meant ",
        "nope i meant ",
        "no i mean ",
        "no, i mean ",
        "no i said ",
        "no, i said ",
        "no ",
        "nope ",
    ]) {
        if !rest.is_empty() {
            return Modification::Replace(rest.to_string());
        }
    }

    // === "instead" replacement ===
    if let Some(rest) = strip_prefix_any(&lower, &[
        "instead ",
        "instead, ",
        "instead do ",
        "instead, do ",
    ]) {
        if !rest.is_empty() {
            return Modification::Replace(format!("{} instead", rest));
        }
    }

    // === "change" patterns ===
    if let Some(rest) = strip_prefix_any(&lower, &[
        "change that to ",
        "change that ",
        "change it to ",
        "change it ",
        "change ",
    ]) {
        if !rest.is_empty() {
            return Modification::Replace(rest.to_string());
        }
    }

    // === Parameter change: "make it louder", "set it to 50%" ===
    if lower.starts_with("make it ")
        || lower.starts_with("set it to ")
        || lower.starts_with("set it ")
    {
        return Modification::Replace(input.to_string());
    }

    // === "try" patterns: "try opening chrome" → replace last command with "open chrome" ===
    if let Some(rest) = strip_prefix_any(&lower, &["try ", "try to ", "try and "]) {
        if !rest.is_empty() {
            return Modification::Replace(rest.to_string());
        }
    }

    Modification::None
}

/// Build a modified command by applying a `Modification` on top of
/// the last user input from context memory.
pub fn apply_modification(modification: &Modification, original: &str) -> Option<String> {
    match modification {
        Modification::Replace(new_cmd) => {
            // If user just says "open chrome", use it as-is
            Some(new_cmd.clone())
        }
        Modification::ChangeParam { param, value } => {
            // Modify a specific param in the last command
            let turns = context::get_recent_context(1);
            if let Some(last) = turns.first() {
                let modified = replace_param(&last.user_input, param, value);
                Some(modified)
            } else {
                Some(format!("set {} to {}", param, value))
            }
        }
        Modification::Undo => {
            // Get the last task step and try to reverse it
            let tasks = context::get_recent_task_steps(1);
            if let Some(last) = tasks.first() {
                Some(format!("undo last: {} ({})", last.tool, last.command))
            } else {
                None
            }
        }
        Modification::Retry => {
            // Retry the last command from context
            let turns = context::get_recent_context(1);
            turns.first().map(|t| t.user_input.clone())
        }
        Modification::None => None,
    }
}

/// Build an enhanced prompt for the LLM that includes the last command context
/// so it can understand modification/redirection queries.
pub fn build_modification_context(command: &str) -> String {
    let context_str = context::build_llm_context();
    if context_str.is_empty() {
        return command.to_string();
    }

    // Check if this looks like a modification
    match detect_modification(command) {
        Modification::Replace(new_cmd) => {
            format!(
                "Previous context:\n{}\n\nThe user originally asked for something else but now says: {}\n\
                 Interpret this as a replacement/redirection of their intent.",
                context_str, new_cmd
            )
        }
        Modification::Undo => {
            format!(
                "Previous context:\n{}\n\nThe user wants to UNDO the last action.\n\
                 Figure out what the last action was and how to reverse it.",
                context_str
            )
        }
        Modification::Retry => {
            format!(
                "Previous context:\n{}\n\nThe user wants to RETRY the last command.\
                 Try the same operation again but handle any errors differently.",
                context_str
            )
        }
        Modification::ChangeParam { param, value } => {
            format!(
                "Previous context:\n{}\n\nThe user wants to change {} to {} in the last command.",
                context_str, param, value
            )
        }
        Modification::None => command.to_string(),
    }
}

fn matches_undo(lower: &str) -> bool {
    matches!(
        lower.trim(),
        "undo" | "undo that" | "undo it" | "nevermind" | "never mind"
            | "go back" | "back" | "that's wrong" | "thats wrong"
            | "not what i wanted" | "that's not what i wanted"
            | "revert" | "revert that"
    )
}

fn matches_retry(lower: &str) -> bool {
    matches!(
        lower.trim(),
        "try again" | "retry" | "do it again" | "again" | "one more time"
            | "try that again"
    ) || lower.starts_with("try again ")
}

fn strip_prefix_any<'a>(input: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for p in prefixes {
        if let Some(rest) = input.strip_prefix(p) {
            return Some(rest);
        }
    }
    None
}

fn replace_param(input: &str, param: &str, value: &str) -> String {
    // Simple replacement: try to find the param in the input and replace it
    let lower = input.to_lowercase();
    let param_lower = param.to_lowercase();

    if let Some(pos) = lower.find(&param_lower) {
        let before = &input[..pos];
        let after = &input[pos + param_lower.len()..];
        let after_words: Vec<&str> = after.split_whitespace().collect();
        if !after_words.is_empty() {
            let rest = after_words[1..].join(" ");
            format!("{} {} {} {}", before.trim(), param, value, rest).trim().to_string()
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    }
}
