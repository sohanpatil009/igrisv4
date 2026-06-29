// src/nlu/engine.rs - Conversational NLU with SBERT semantic understanding
// Enhanced for natural language processing with filler word removal and alias support

use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::nlu::sbert::GLOBAL_SBERT;
use crate::nlu::ner::{GLOBAL_NER, EntityType};

// ═══════════════════════════════════════════════════════════════════════════════
// FILLER WORDS & NORMALIZATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Common filler words to remove for better intent matching
const FILLER_WORDS: &[&str] = &[
    "okay", "ok", "now", "please", "can", "you", "could", "would", "just",
    "actually", "basically", "literally", "really", "very", "so", "well",
    "um", "uh", "like", "i", "want", "to", "need", "the", "a", "an",
    "hey", "hi", "hello", "yo", "alright", "right", "sure", "yeah",
    "gonna", "wanna", "gotta", "lemme", "let", "me", "my", "for", "me",
    "kindly", "if", "possible", "maybe", "perhaps", "think", "guess",
];

/// Normalize input by removing filler words and cleaning
fn normalize_input(input: &str) -> String {
    let filler_set: HashSet<&str> = FILLER_WORDS.iter().copied().collect();
    
    input
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == '_' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .filter(|word| !filler_set.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Light normalization (keeps more context)
fn light_normalize(input: &str) -> String {
    input
        .to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == '_' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ═══════════════════════════════════════════════════════════════════════════════
// APP ALIASES
// ═══════════════════════════════════════════════════════════════════════════════

/// Application name aliases for better recognition
fn get_app_aliases() -> HashMap<&'static str, Vec<&'static str>> {
    let mut aliases = HashMap::new();
    
    // Browsers
    aliases.insert("chrome", vec!["chrome", "google chrome", "google", "browser"]);
    aliases.insert("firefox", vec!["firefox", "mozilla", "mozilla firefox"]);
    aliases.insert("edge", vec!["edge", "microsoft edge", "msedge"]);
    aliases.insert("brave", vec!["brave", "brave browser"]);
    aliases.insert("opera", vec!["opera", "opera browser"]);
    aliases.insert("safari", vec!["safari"]);
    
    // Editors & IDEs
    aliases.insert("notepad", vec!["notepad", "note pad", "text editor"]);
    aliases.insert("vscode", vec!["vscode", "vs code", "visual studio code", "code"]);
    aliases.insert("sublime", vec!["sublime", "sublime text"]);
    aliases.insert("atom", vec!["atom", "atom editor"]);
    aliases.insert("vim", vec!["vim", "vi"]);
    aliases.insert("notepad++", vec!["notepad++", "notepad plus plus", "npp"]);
    
    // Communication
    aliases.insert("discord", vec!["discord"]);
    aliases.insert("slack", vec!["slack"]);
    aliases.insert("teams", vec!["teams", "microsoft teams", "ms teams"]);
    aliases.insert("zoom", vec!["zoom", "zoom meeting"]);
    aliases.insert("skype", vec!["skype"]);
    aliases.insert("telegram", vec!["telegram"]);
    aliases.insert("whatsapp", vec!["whatsapp", "whats app"]);
    
    // Media
    aliases.insert("spotify", vec!["spotify", "music"]);
    aliases.insert("vlc", vec!["vlc", "vlc player", "media player"]);
    aliases.insert("youtube", vec!["youtube", "yt"]);
    
    // Utilities
    aliases.insert("calculator", vec!["calculator", "calc"]);
    aliases.insert("explorer", vec!["explorer", "file explorer", "files", "my computer", "this pc"]);
    aliases.insert("terminal", vec!["terminal", "cmd", "command prompt", "powershell", "shell", "console"]);
    aliases.insert("settings", vec!["settings", "control panel", "preferences"]);
    aliases.insert("task manager", vec!["task manager", "taskmanager", "processes"]);
    
    // Office
    aliases.insert("word", vec!["word", "microsoft word", "ms word"]);
    aliases.insert("excel", vec!["excel", "microsoft excel", "ms excel", "spreadsheet"]);
    aliases.insert("powerpoint", vec!["powerpoint", "ppt", "microsoft powerpoint", "presentation"]);
    aliases.insert("outlook", vec!["outlook", "email", "mail"]);
    
    // Games & Entertainment
    aliases.insert("steam", vec!["steam"]);
    aliases.insert("epic", vec!["epic", "epic games", "epic launcher"]);
    
    // Development
    aliases.insert("git", vec!["git", "git bash"]);
    aliases.insert("docker", vec!["docker"]);
    aliases.insert("postman", vec!["postman", "api tester"]);
    
    aliases
}

/// Resolve app name from aliases
fn resolve_app_name(input: &str) -> Option<String> {
    let input_lower = input.to_lowercase();
    let aliases = get_app_aliases();
    
    // Direct match first
    for (canonical, alias_list) in &aliases {
        for alias in alias_list {
            if input_lower == *alias || input_lower.contains(alias) {
                return Some(canonical.to_string());
            }
        }
    }
    
    // Fuzzy match - check if input contains any alias
    for (canonical, alias_list) in &aliases {
        for alias in alias_list {
            if input_lower.split_whitespace().any(|w| w == *alias) {
                return Some(canonical.to_string());
            }
        }
    }
    
    // Return original if no alias found
    if !input_lower.is_empty() {
        Some(input_lower)
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// INTENT & ENTITY TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,
    pub example_phrases: Vec<String>,
    pub precomputed_embeddings: Vec<Vec<f32>>,
}

impl Intent {
    pub fn new(name: String, example_phrases: Vec<String>) -> Self {
        Self {
            name,
            example_phrases,
            precomputed_embeddings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntentResult {
    pub intent_name: String,
    pub confidence: f32,
    pub entities: HashMap<String, String>,
    pub normalized_input: String,
}

impl IntentResult {
    pub fn unknown() -> Self {
        Self {
            intent_name: "UnknownIntent".to_string(),
            confidence: 0.0,
            entities: HashMap::new(),
            normalized_input: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct EntityPattern {
    name: String,
    regex: Regex,
}

impl EntityPattern {
    fn new(name: &str, pattern: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            regex: Regex::new(pattern)?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NLU CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct NluConfig {
    pub similarity_threshold: f32,
    pub use_sbert: bool,
    pub use_filler_removal: bool,
}

impl Default for NluConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.35, // Lower threshold for more natural matching
            use_sbert: true,
            use_filler_removal: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NLU ENGINE
// ═══════════════════════════════════════════════════════════════════════════════

pub struct NluEngine {
    intents: Vec<Intent>,
    entity_patterns: Vec<EntityPattern>,
    config: NluConfig,
    sbert_initialized: bool,
}

impl NluEngine {
    pub fn new() -> Self {
        Self {
            intents: Vec::new(),
            entity_patterns: Vec::new(),
            config: NluConfig::default(),
            sbert_initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        println!("🧠 Initializing Conversational NLU engine...");
        
        // Initialize NER engine
        println!("📍 Initializing Named Entity Recognition engine...");
        let _ = &GLOBAL_NER; // Initialize global NER
        println!("✅ NER engine ready");
        
        if self.config.use_sbert {
            match GLOBAL_SBERT.initialize() {
                Ok(_) => {
                    self.sbert_initialized = true;
                    println!("✅ SBERT semantic engine initialized");
                }
                Err(e) => {
                    println!("⚠️ SBERT init failed: {}, using fallback", e);
                    self.sbert_initialized = false;
                }
            }
        }
        
        self.setup_default_intents();
        self.setup_entity_patterns()?;
        
        println!("✅ Conversational NLU ready with {} intents", self.intents.len());
        println!("✅ NER patterns configured for entity extraction");
        Ok(())
    }

    fn setup_default_intents(&mut self) {
        // More natural, conversational example phrases
        let intents = vec![
            Intent::new(
                "open_app".to_string(),
                vec![
                    "open chrome".into(),
                    "launch firefox".into(),
                    "start notepad".into(),
                    "run calculator".into(),
                    "open the browser".into(),
                    "can you open chrome".into(),
                    "please launch spotify".into(),
                    "i want to open discord".into(),
                    "fire up vscode".into(),
                    "bring up terminal".into(),
                    "show me file explorer".into(),
                    "get chrome running".into(),
                ],
            ),
            Intent::new(
                "close_app".to_string(),
                vec![
                    "close chrome".into(),
                    "shut down firefox".into(),
                    "exit notepad".into(),
                    "quit calculator".into(),
                    "kill the browser".into(),
                    "stop spotify".into(),
                    "terminate discord".into(),
                    "close that app".into(),
                    "shut it down".into(),
                ],
            ),
            Intent::new(
                "camera_control".to_string(),
                vec![
                    "camera".into(),
                    "open camera".into(),
                    "start camera".into(),
                    "launch camera".into(),
                    "camera mode".into(),
                    "take a photo".into(),
                    "take photo".into(),
                    "capture image".into(),
                    "take picture".into(),
                    "snap photo".into(),
                    "record video".into(),
                    "start recording".into(),
                    "snap a picture".into(),
                    "take a selfie".into(),
                ],
            ),
            Intent::new(
                "system_control".to_string(),
                vec![
                    "shutdown".into(),
                    "restart".into(),
                    "lock screen".into(),
                    "sleep mode".into(),
                    "increase volume".into(),
                    "decrease volume".into(),
                    "mute".into(),
                    "unmute".into(),
                    "turn up the volume".into(),
                    "make it louder".into(),
                    "turn on wifi".into(),
                    "enable bluetooth".into(),
                    "brightness up".into(),
                    "dim the screen".into(),
                ],
            ),
            Intent::new(
                "assistant_control".to_string(),
                vec![
                    "go to sleep".into(),
                    "standby".into(),
                    "exit".into(),
                    "quit".into(),
                    "shutdown assistant".into(),
                    "goodbye".into(),
                    "see you later".into(),
                    "that's all".into(),
                    "i'm done".into(),
                ],
            ),
            Intent::new(
                "greeting".to_string(),
                vec![
                    "hello".into(),
                    "hi".into(),
                    "hey".into(),
                    "good morning".into(),
                    "good afternoon".into(),
                    "good evening".into(),
                    "what's up".into(),
                    "howdy".into(),
                ],
            ),
            Intent::new(
                "web_search".to_string(),
                vec![
                    "search for".into(),
                    "google".into(),
                    "look up".into(),
                    "find information".into(),
                    "what is".into(),
                    "who is".into(),
                    "where is".into(),
                    "how to".into(),
                    "search the web".into(),
                ],
            ),
            Intent::new(
                "about_igris".to_string(),
                vec![
                    "tell me about yourself".into(),
                    "tell me about you".into(),
                    "who are you".into(),
                    "what are you".into(),
                    "what can you do".into(),
                    "introduce yourself".into(),
                    "your capabilities".into(),
                    "your features".into(),
                    "igris tell me about yourself".into(),
                    "tell me about igris".into(),
                    "what is igris".into(),
                    "explain yourself".into(),
                    "describe yourself".into(),
                    "what do you do".into(),
                    "how do you work".into(),
                ],
            ),
        ];

        self.intents = intents;
    }

    fn setup_entity_patterns(&mut self) -> Result<()> {
        let patterns = vec![
            // App extraction - more flexible patterns
            EntityPattern::new("app", r"(?:open|launch|start|run|close|quit|exit|kill|stop|fire\s+up|bring\s+up)\s+(?:the\s+)?([a-zA-Z0-9\s]+?)(?:\s+app|\s+application|\s+please|\s+now|$)")?,
            EntityPattern::new("app_simple", r"(?:open|launch|start|close|quit)\s+([a-zA-Z0-9]+)")?,
            
            // File patterns
            EntityPattern::new("file", r"(?:file|document)\s+(?:called\s+|named\s+)?([a-zA-Z0-9._\-/\\]+)")?,
            EntityPattern::new("folder", r"(?:folder|directory)\s+(?:called\s+|named\s+)?([a-zA-Z0-9._\-/\\]+)")?,
            
            // System actions
            EntityPattern::new("action", r"(shutdown|restart|sleep|hibernate|lock|mute|unmute)")?,
            
            // Volume/brightness levels
            EntityPattern::new("level", r"(?:to\s+)?(\d+)\s*(?:percent|%)?")?,
            
            // Time expressions
            EntityPattern::new("time", r"(?:in|after)\s+(\d+)\s+(minutes?|hours?|seconds?)")?,
        ];

        self.entity_patterns = patterns;
        Ok(())
    }

    /// Main processing function - handles natural language
    pub fn process_input(&self, input: &str) -> Result<IntentResult> {
        let light_cleaned = light_normalize(input);
        let normalized = if self.config.use_filler_removal {
            normalize_input(input)
        } else {
            light_cleaned.clone()
        };
        
        // Try SBERT semantic matching
        if self.sbert_initialized && self.config.use_sbert {
            // Try with normalized input first
            if let Some((intent_name, confidence)) = GLOBAL_SBERT.find_intent(&normalized) {
                if confidence >= self.config.similarity_threshold {
                    let entities = self.extract_entities_smart(&light_cleaned, &intent_name)?;
                    
                    return Ok(IntentResult {
                        intent_name,
                        confidence,
                        entities,
                        normalized_input: normalized,
                    });
                }
            }
            
            // Try with light-cleaned input (more context)
            if let Some((intent_name, confidence)) = GLOBAL_SBERT.find_intent(&light_cleaned) {
                if confidence >= self.config.similarity_threshold {
                    let entities = self.extract_entities_smart(&light_cleaned, &intent_name)?;
                    
                    return Ok(IntentResult {
                        intent_name,
                        confidence,
                        entities,
                        normalized_input: light_cleaned,
                    });
                }
            }
        }
        
        // Fallback to keyword matching with both versions
        let (best_intent, best_confidence) = self.keyword_match(&normalized, &light_cleaned);

        if best_confidence < self.config.similarity_threshold {
            return Ok(IntentResult::unknown());
        }

        let entities = self.extract_entities_smart(&light_cleaned, &best_intent)?;

        Ok(IntentResult {
            intent_name: best_intent,
            confidence: best_confidence,
            entities,
            normalized_input: normalized,
        })
    }

    /// Keyword-based matching fallback
    fn keyword_match(&self, normalized: &str, original: &str) -> (String, f32) {
        let mut best_intent = String::new();
        let mut best_confidence = 0.0;

        for intent in &self.intents {
            for phrase in &intent.example_phrases {
                // Try normalized
                let sim1 = self.jaccard_similarity(normalized, phrase);
                // Try original
                let sim2 = self.jaccard_similarity(original, phrase);
                
                let similarity = sim1.max(sim2);
                
                if similarity > best_confidence {
                    best_confidence = similarity;
                    best_intent = intent.name.clone();
                }
            }
        }

        (best_intent, best_confidence)
    }

    fn jaccard_similarity(&self, input: &str, phrase: &str) -> f32 {
        let input_words: HashSet<_> = input.split_whitespace().collect();
        let phrase_words: HashSet<_> = phrase.split_whitespace().collect();
        
        let intersection = input_words.intersection(&phrase_words).count();
        let union = input_words.union(&phrase_words).count();
        
        if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
    }

    /// Smart entity extraction based on intent type
    fn extract_entities_smart(&self, input: &str, intent: &str) -> Result<HashMap<String, String>> {
        let mut entities = HashMap::new();

        // First, use NER for structured entity extraction
        let ner_entities = GLOBAL_NER.extract_entities(input);
        
        // Map NER entities to standard keys
        for ner_entity in ner_entities {
            match ner_entity.entity_type {
                EntityType::Application => {
                    entities.insert("app".to_string(), ner_entity.value);
                }
                EntityType::File => {
                    entities.insert("file".to_string(), ner_entity.value);
                }
                EntityType::Folder => {
                    entities.insert("folder".to_string(), ner_entity.value);
                }
                EntityType::Number => {
                    entities.insert("number".to_string(), ner_entity.value);
                }
                EntityType::Percentage => {
                    entities.insert("percentage".to_string(), ner_entity.value);
                }
                EntityType::Duration => {
                    entities.insert("duration".to_string(), ner_entity.value);
                }
                EntityType::SystemAction => {
                    entities.insert("action".to_string(), ner_entity.value);
                }
                EntityType::VolumeLevel => {
                    entities.insert("volume".to_string(), ner_entity.value);
                }
                EntityType::BrightnessLevel => {
                    entities.insert("brightness".to_string(), ner_entity.value);
                }
                EntityType::NetworkInterface => {
                    entities.insert("network".to_string(), ner_entity.value);
                }
                EntityType::Generic => {
                    entities.insert("generic".to_string(), ner_entity.value);
                }
            }
        }

        // Intent-specific entity extraction (fallback/additional)
        match intent {
            "open_app" | "close_app" => {
                // Use NER-extracted app if available, otherwise use regex
                if !entities.contains_key("app") {
                    if let Some(app) = self.extract_app_entity(input) {
                        entities.insert("app".to_string(), app);
                    }
                }
            }

            "system_control" => {
                // Extract action and level if not already found by NER
                if !entities.contains_key("action") {
                    for pattern in &self.entity_patterns {
                        if pattern.name == "action" {
                            if let Some(captures) = pattern.regex.captures(input) {
                                if let Some(matched) = captures.get(1) {
                                    entities.insert("action".to_string(), matched.as_str().trim().to_string());
                                }
                            }
                        }
                    }
                }
                
                // Extract volume/brightness level if available
                if !entities.contains_key("volume") && !entities.contains_key("level") {
                    for pattern in &self.entity_patterns {
                        if pattern.name == "level" {
                            if let Some(captures) = pattern.regex.captures(input) {
                                if let Some(matched) = captures.get(1) {
                                    entities.insert("level".to_string(), matched.as_str().trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
            "web_search" => {
                // Extract search query
                if !entities.contains_key("query") {
                    let search_terms = vec!["search for", "google", "look up", "find", "what is", "who is"];
                    for term in search_terms {
                        if let Some(pos) = input.find(term) {
                            let query = &input[pos + term.len()..].trim();
                            if !query.is_empty() {
                                entities.insert("query".to_string(), query.to_string());
                                break;
                            }
                        }
                    }
                }
            }
            _ => {
                // Generic extraction - use regex patterns as fallback
                for pattern in &self.entity_patterns {
                    if !entities.contains_key(&pattern.name) {
                        if let Some(captures) = pattern.regex.captures(input) {
                            if let Some(matched) = captures.get(1) {
                                entities.insert(pattern.name.clone(), matched.as_str().trim().to_string());
                            }
                        }
                    }
                }
            }
        }

        // Merge with SBERT semantic entities if available and not already found
        if GLOBAL_NLU.is_sbert_enabled() {
            let sbert_entities = GLOBAL_SBERT.extract_semantic_entities(input);
            for (key, value) in sbert_entities {
                // Only add if not already extracted by NER
                if !entities.contains_key(&key) || entities[&key].is_empty() {
                    entities.insert(key, value);
                }
            }
        }

        Ok(entities)
    }

    /// Extract app name with alias support
    fn extract_app_entity(&self, input: &str) -> Option<String> {
        // Try regex patterns first
        for pattern in &self.entity_patterns {
            if pattern.name == "app" || pattern.name == "app_simple" {
                if let Some(captures) = pattern.regex.captures(input) {
                    if let Some(matched) = captures.get(1) {
                        let raw_app = matched.as_str().trim();
                        // Resolve through aliases
                        if let Some(resolved) = resolve_app_name(raw_app) {
                            return Some(resolved);
                        }
                    }
                }
            }
        }
        
        // Fallback: find known app names in input
        let aliases = get_app_aliases();
        for (canonical, alias_list) in &aliases {
            for alias in alias_list {
                if input.contains(alias) {
                    return Some(canonical.to_string());
                }
            }
        }
        
        // Last resort: extract last word after action verb
        let action_words = ["open", "launch", "start", "run", "close", "quit", "exit", "kill", "stop"];
        for action in action_words {
            if let Some(pos) = input.find(action) {
                let after = &input[pos + action.len()..].trim();
                let words: Vec<&str> = after.split_whitespace().collect();
                if !words.is_empty() {
                    let app_candidate = words.join(" ");
                    if let Some(resolved) = resolve_app_name(&app_candidate) {
                        return Some(resolved);
                    }
                }
            }
        }
        
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// THREAD-SAFE WRAPPER
// ═══════════════════════════════════════════════════════════════════════════════

pub struct SharedNluEngine {
    engine: std::sync::Arc<std::sync::Mutex<NluEngine>>,
}

impl SharedNluEngine {
    pub fn new() -> Self {
        Self {
            engine: std::sync::Arc::new(std::sync::Mutex::new(NluEngine::new())),
        }
    }

    pub fn initialize(&self) -> Result<()> {
        let mut engine = self.engine.lock().map_err(|_| anyhow!("Lock failed"))?;
        engine.initialize()
    }

    pub fn process_input(&self, input: &str) -> Result<IntentResult> {
        let engine = self.engine.lock().map_err(|_| anyhow!("Lock failed"))?;
        engine.process_input(input)
    }

    pub fn is_sbert_enabled(&self) -> bool {
        self.engine.lock().map(|e| e.sbert_initialized).unwrap_or(false)
    }
    
    pub fn get_similarity(&self, text1: &str, text2: &str) -> f32 {
        GLOBAL_SBERT.text_similarity(text1, text2)
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_NLU: SharedNluEngine = SharedNluEngine::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_input() {
        let input = "okay now can you please open the chrome";
        let normalized = normalize_input(input);
        assert!(normalized.contains("open"));
        assert!(normalized.contains("chrome"));
        assert!(!normalized.contains("okay"));
        assert!(!normalized.contains("please"));
    }

    #[test]
    fn test_app_alias_resolution() {
        assert_eq!(resolve_app_name("google chrome"), Some("chrome".to_string()));
        assert_eq!(resolve_app_name("vs code"), Some("vscode".to_string()));
        assert_eq!(resolve_app_name("file explorer"), Some("explorer".to_string()));
    }

    #[test]
    fn test_unknown_intent() {
        let result = IntentResult::unknown();
        assert_eq!(result.intent_name, "UnknownIntent");
    }
}
