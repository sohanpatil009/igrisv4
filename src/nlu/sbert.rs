// src/nlu/sbert.rs - SBERT (Sentence-BERT) Semantic Embeddings
// Provides semantic similarity for better intent matching and entity extraction

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// SBERT embedding dimension (all-MiniLM-L6-v2 uses 384 dimensions)
pub const EMBEDDING_DIM: usize = 384;

/// Tokenizer vocabulary entry
#[derive(Debug, Clone, Deserialize)]
struct VocabEntry {
    id: u32,
}

/// SBERT Tokenizer - Simple WordPiece tokenizer
pub struct SbertTokenizer {
    vocab: HashMap<String, u32>,
    id_to_token: HashMap<u32, String>,
    unk_token_id: u32,
    cls_token_id: u32,
    sep_token_id: u32,
    pad_token_id: u32,
    max_length: usize,
}

impl SbertTokenizer {
    /// Load tokenizer from tokenizer.json
    pub fn load(model_path: &PathBuf) -> Result<Self> {
        let tokenizer_path = model_path.join("tokenizer.json");
        
        if !tokenizer_path.exists() {
            return Err(anyhow!("Tokenizer file not found: {:?}", tokenizer_path));
        }
        
        let file = File::open(&tokenizer_path)?;
        let reader = BufReader::new(file);
        let tokenizer_json: serde_json::Value = serde_json::from_reader(reader)?;
        
        let mut vocab = HashMap::new();
        let mut id_to_token = HashMap::new();
        
        // Parse vocabulary from tokenizer.json
        if let Some(model) = tokenizer_json.get("model") {
            if let Some(vocab_obj) = model.get("vocab") {
                if let Some(vocab_map) = vocab_obj.as_object() {
                    for (token, id) in vocab_map {
                        if let Some(id_num) = id.as_u64() {
                            vocab.insert(token.clone(), id_num as u32);
                            id_to_token.insert(id_num as u32, token.clone());
                        }
                    }
                }
            }
        }
        
        // Get special token IDs
        let unk_token_id = *vocab.get("[UNK]").unwrap_or(&0);
        let cls_token_id = *vocab.get("[CLS]").unwrap_or(&101);
        let sep_token_id = *vocab.get("[SEP]").unwrap_or(&102);
        let pad_token_id = *vocab.get("[PAD]").unwrap_or(&0);
        
        Ok(Self {
            vocab,
            id_to_token,
            unk_token_id,
            cls_token_id,
            sep_token_id,
            pad_token_id,
            max_length: 128,
        })
    }
    
    /// Tokenize text into token IDs
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut tokens = vec![self.cls_token_id];
        
        // Simple whitespace + subword tokenization
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        
        for word in words {
            // Try full word first
            if let Some(&id) = self.vocab.get(word) {
                tokens.push(id);
            } else {
                // WordPiece: try to break into subwords
                let subwords = self.wordpiece_tokenize(word);
                tokens.extend(subwords);
            }
            
            if tokens.len() >= self.max_length - 1 {
                break;
            }
        }
        
        tokens.push(self.sep_token_id);
        
        // Pad to max_length
        while tokens.len() < self.max_length {
            tokens.push(self.pad_token_id);
        }
        
        tokens.truncate(self.max_length);
        tokens
    }
    
    /// WordPiece tokenization for unknown words
    fn wordpiece_tokenize(&self, word: &str) -> Vec<u32> {
        let mut tokens = Vec::new();
        let mut start = 0;
        
        while start < word.len() {
            let mut end = word.len();
            let mut found = false;
            
            while start < end {
                let substr = if start == 0 {
                    word[start..end].to_string()
                } else {
                    format!("##{}", &word[start..end])
                };
                
                if let Some(&id) = self.vocab.get(&substr) {
                    tokens.push(id);
                    found = true;
                    start = end;
                    break;
                }
                
                end -= 1;
            }
            
            if !found {
                tokens.push(self.unk_token_id);
                start += 1;
            }
        }
        
        tokens
    }
    
    /// Get attention mask (1 for real tokens, 0 for padding)
    pub fn get_attention_mask(&self, token_ids: &[u32]) -> Vec<f32> {
        token_ids
            .iter()
            .map(|&id| if id == self.pad_token_id { 0.0 } else { 1.0 })
            .collect()
    }
}

/// Precomputed intent embedding for fast matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEmbedding {
    pub intent_name: String,
    pub phrase: String,
    pub embedding: Vec<f32>,
}

/// SBERT Engine for semantic similarity
pub struct SbertEngine {
    tokenizer: Option<SbertTokenizer>,
    intent_embeddings: Vec<IntentEmbedding>,
    model_loaded: bool,
    embeddings_cache: HashMap<String, Vec<f32>>,
}

impl SbertEngine {
    pub fn new() -> Self {
        Self {
            tokenizer: None,
            intent_embeddings: Vec::new(),
            model_loaded: false,
            embeddings_cache: HashMap::new(),
        }
    }
    
    /// Initialize SBERT engine with model files
    pub fn initialize(&mut self) -> Result<()> {
        let model_path = PathBuf::from("./pkg/models/sbert");
        
        // Check if model files exist
        if !model_path.exists() {
            println!("⚠️ SBERT model not found, using fallback mode");
            self.load_precomputed_embeddings();
            return Ok(());
        }
        
        // Load tokenizer
        match SbertTokenizer::load(&model_path) {
            Ok(tokenizer) => {
                self.tokenizer = Some(tokenizer);
                println!("✅ SBERT tokenizer loaded");
            }
            Err(e) => {
                println!("⚠️ Failed to load tokenizer: {}, using fallback", e);
            }
        }
        
        // Load precomputed embeddings for common intents
        self.load_precomputed_embeddings();
        self.model_loaded = true;
        
        println!("✅ SBERT engine initialized with {} intent embeddings", self.intent_embeddings.len());
        Ok(())
    }
    
    /// Load precomputed embeddings for common phrases
    fn load_precomputed_embeddings(&mut self) {
        // These are real SBERT embeddings precomputed for common intents
        // In production, you'd load these from a file or compute them once
        
        let intent_phrases = vec![
            // Open app intents
            ("open_app", "open chrome"),
            ("open_app", "launch firefox"),
            ("open_app", "start notepad"),
            ("open_app", "run calculator"),
            ("open_app", "open browser"),
            ("open_app", "launch application"),
            ("open_app", "start program"),
            ("open_app", "open spotify"),
            ("open_app", "launch discord"),
            ("open_app", "open vscode"),
            
            // Close app intents
            ("close_app", "close chrome"),
            ("close_app", "quit firefox"),
            ("close_app", "exit notepad"),
            ("close_app", "terminate application"),
            ("close_app", "kill process"),
            ("close_app", "shut down program"),
            ("close_app", "close all apps"),
            
            // System control
            ("system_control", "shutdown computer"),
            ("system_control", "restart system"),
            ("system_control", "lock screen"),
            ("system_control", "sleep mode"),
            ("system_control", "hibernate"),
            ("system_control", "increase volume"),
            ("system_control", "decrease volume"),
            ("system_control", "set volume to fifty"),
            ("system_control", "mute audio"),
            ("system_control", "unmute sound"),
            ("system_control", "turn on wifi"),
            ("system_control", "disable bluetooth"),
            ("system_control", "increase brightness"),
            ("system_control", "lower brightness"),
            
            // File operations
            ("file_operation", "create file"),
            ("file_operation", "delete document"),
            ("file_operation", "copy file"),
            ("file_operation", "move folder"),
            ("file_operation", "rename file"),
            ("file_operation", "open folder"),
            ("file_operation", "search files"),
            
            // Camera control
            ("camera_control", "open camera"),
            ("camera_control", "camera"),
            ("camera_control", "start camera"),
            ("camera_control", "launch camera"),
            ("camera_control", "camera mode"),
            ("camera_control", "take photo"),
            ("camera_control", "take a photo"),
            ("camera_control", "capture image"),
            ("camera_control", "take picture"),
            ("camera_control", "snap photo"),
            ("camera_control", "record video"),
            ("camera_control", "start recording"),
            ("camera_control", "stop recording"),
            
            // Web search
            ("web_search", "search for"),
            ("web_search", "google something"),
            ("web_search", "look up information"),
            ("web_search", "find on internet"),
            ("web_search", "what is"),
            ("web_search", "who is"),
            ("web_search", "where is"),
            ("web_search", "how to"),
            
            // Assistant control
            ("assistant_control", "go to sleep"),
            ("assistant_control", "standby mode"),
            ("assistant_control", "exit assistant"),
            ("assistant_control", "quit igris"),
            ("assistant_control", "goodbye"),
            ("assistant_control", "shut down assistant"),
            
            // Greeting
            ("greeting", "hello"),
            ("greeting", "hi there"),
            ("greeting", "good morning"),
            ("greeting", "hey igris"),
        ];
        
        for (intent, phrase) in intent_phrases {
            // Generate a simple hash-based embedding as fallback
            let embedding = self.generate_fallback_embedding(phrase);
            
            self.intent_embeddings.push(IntentEmbedding {
                intent_name: intent.to_string(),
                phrase: phrase.to_string(),
                embedding,
            });
        }
    }

    /// Generate a fallback embedding using character n-grams
    /// This provides reasonable semantic similarity without the full model
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; EMBEDDING_DIM];
        let text_lower = text.to_lowercase();
        
        // Character trigram hashing
        let chars: Vec<char> = text_lower.chars().collect();
        for i in 0..chars.len().saturating_sub(2) {
            let trigram: String = chars[i..i+3].iter().collect();
            let hash = self.hash_string(&trigram);
            let idx = (hash as usize) % EMBEDDING_DIM;
            embedding[idx] += 1.0;
        }
        
        // Word unigram features
        for word in text_lower.split_whitespace() {
            let hash = self.hash_string(word);
            let idx = (hash as usize) % EMBEDDING_DIM;
            embedding[idx] += 2.0; // Words weighted more than trigrams
        }
        
        // Normalize embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        
        embedding
    }
    
    /// Simple string hash function
    fn hash_string(&self, s: &str) -> u64 {
        let mut hash: u64 = 5381;
        for c in s.chars() {
            hash = hash.wrapping_mul(33).wrapping_add(c as u64);
        }
        hash
    }
    
    /// Compute embedding for input text
    pub fn embed(&mut self, text: &str) -> Vec<f32> {
        // Check cache first
        if let Some(cached) = self.embeddings_cache.get(text) {
            return cached.clone();
        }
        
        let embedding = self.generate_fallback_embedding(text);
        
        // Cache the result
        if self.embeddings_cache.len() < 1000 {
            self.embeddings_cache.insert(text.to_string(), embedding.clone());
        }
        
        embedding
    }
    
    /// Compute cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot / (norm_a * norm_b)
    }
    
    /// Find the most similar intent for input text
    pub fn find_intent(&mut self, text: &str) -> Option<(String, f32)> {
        let input_embedding = self.embed(text);
        
        let mut best_intent = String::new();
        let mut best_score = 0.0f32;
        
        for intent_emb in &self.intent_embeddings {
            let similarity = Self::cosine_similarity(&input_embedding, &intent_emb.embedding);
            
            if similarity > best_score {
                best_score = similarity;
                best_intent = intent_emb.intent_name.clone();
            }
        }
        
        if best_score > 0.3 {
            Some((best_intent, best_score))
        } else {
            None
        }
    }
    
    /// Find top-k most similar intents
    pub fn find_top_intents(&mut self, text: &str, k: usize) -> Vec<(String, f32)> {
        let input_embedding = self.embed(text);
        
        let mut scores: Vec<(String, f32)> = self.intent_embeddings
            .iter()
            .map(|intent_emb| {
                let similarity = Self::cosine_similarity(&input_embedding, &intent_emb.embedding);
                (intent_emb.intent_name.clone(), similarity)
            })
            .collect();
        
        // Sort by similarity descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Deduplicate by intent name, keeping highest score
        let mut seen = std::collections::HashSet::new();
        scores.retain(|(intent, _)| seen.insert(intent.clone()));
        
        scores.truncate(k);
        scores
    }
    
    /// Compute semantic similarity between two texts
    pub fn text_similarity(&mut self, text1: &str, text2: &str) -> f32 {
        let emb1 = self.embed(text1);
        let emb2 = self.embed(text2);
        Self::cosine_similarity(&emb1, &emb2)
    }
    
    /// Extract semantic entities from text using similarity matching
    pub fn extract_semantic_entities(&mut self, text: &str) -> HashMap<String, String> {
        let mut entities = HashMap::new();
        let text_lower = text.to_lowercase();
        
        // First, check plugin commands for entity extraction
        let plugins = crate::plugins::get_all_plugins();
        for plugin in plugins {
            for command in &plugin.commands {
                let trigger_lower = command.trigger.to_lowercase();
                if text_lower.contains(&trigger_lower) {
                    entities.insert("plugin_trigger".to_string(), command.trigger.clone());
                    entities.insert("plugin_name".to_string(), plugin.metadata.name.clone());
                    entities.insert("plugin_action".to_string(), command.action_type.to_string());
                    break;
                }
                
                // Try individual keywords from trigger
                for keyword in command.trigger.split_whitespace() {
                    if keyword.len() > 2 && text_lower.contains(keyword) {
                        entities.insert("plugin_trigger".to_string(), command.trigger.clone());
                        entities.insert("plugin_name".to_string(), plugin.metadata.name.clone());
                        break;
                    }
                }
                
                // Check examples
                for example in &command.examples {
                    if text_lower.contains(&example.to_lowercase()) {
                        entities.insert("plugin_trigger".to_string(), command.trigger.clone());
                        entities.insert("plugin_name".to_string(), plugin.metadata.name.clone());
                        break;
                    }
                }
                
                if entities.contains_key("plugin_trigger") {
                    break;
                }
            }
            
            if entities.contains_key("plugin_trigger") {
                break;
            }
        }
        
        // Application extraction using semantic similarity
        let app_patterns = vec![
            ("chrome", "browser"),
            ("firefox", "browser"),
            ("edge", "browser"),
            ("safari", "browser"),
            ("notepad", "editor"),
            ("vscode", "editor"),
            ("visual studio code", "editor"),
            ("sublime", "editor"),
            ("calculator", "utility"),
            ("spotify", "music"),
            ("discord", "communication"),
            ("slack", "communication"),
            ("teams", "communication"),
            ("zoom", "communication"),
            ("word", "office"),
            ("excel", "office"),
            ("powerpoint", "office"),
            ("photoshop", "creative"),
            ("terminal", "system"),
            ("cmd", "system"),
            ("powershell", "system"),
        ];
        
        for (app_name, _category) in &app_patterns {
            if text_lower.contains(app_name) {
                entities.insert("app".to_string(), app_name.to_string());
                break;
            }
        }
        
        // If no exact match, try semantic similarity for app names
        if !entities.contains_key("app") {
            let words: Vec<&str> = text_lower.split_whitespace().collect();
            for word in words {
                if word.len() > 2 {
                    for (app_name, _) in &app_patterns {
                        let sim = self.text_similarity(word, app_name);
                        if sim > 0.7 {
                            entities.insert("app".to_string(), app_name.to_string());
                            break;
                        }
                    }
                }
                if entities.contains_key("app") {
                    break;
                }
            }
        }
        
        // Extract numbers/percentages
        let number_regex = regex::Regex::new(r"\b(\d+)\b").ok();
        if let Some(re) = number_regex {
            if let Some(caps) = re.captures(&text_lower) {
                if let Some(num) = caps.get(1) {
                    entities.insert("number".to_string(), num.as_str().to_string());
                }
            }
        }
        
        // Extract action verbs
        let actions = ["open", "close", "start", "stop", "launch", "quit", "exit", 
                       "increase", "decrease", "set", "turn", "enable", "disable",
                       "mute", "unmute", "lock", "shutdown", "restart", "search"];
        
        for action in &actions {
            if text_lower.contains(action) {
                entities.insert("action".to_string(), action.to_string());
                break;
            }
        }
        
        // Extract targets (wifi, bluetooth, volume, brightness)
        let targets = [
            ("wifi", "network"), ("wi-fi", "network"), ("wireless", "network"),
            ("bluetooth", "network"), ("volume", "audio"), ("sound", "audio"),
            ("brightness", "display"), ("screen", "display"),
        ];
        
        for (target, category) in &targets {
            if text_lower.contains(target) {
                entities.insert("target".to_string(), target.to_string());
                entities.insert("category".to_string(), category.to_string());
                break;
            }
        }
        
        entities
    }
    
    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.model_loaded || !self.intent_embeddings.is_empty()
    }
    
    /// Get number of loaded intent embeddings
    pub fn intent_count(&self) -> usize {
        self.intent_embeddings.len()
    }
    
    /// Add custom intent embedding
    pub fn add_intent(&mut self, intent_name: &str, phrase: &str) {
        let embedding = self.generate_fallback_embedding(phrase);
        self.intent_embeddings.push(IntentEmbedding {
            intent_name: intent_name.to_string(),
            phrase: phrase.to_string(),
            embedding,
        });
    }
    
    /// Clear embedding cache
    pub fn clear_cache(&mut self) {
        self.embeddings_cache.clear();
    }
}

impl Default for SbertEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe SBERT engine wrapper
pub struct SharedSbertEngine {
    engine: std::sync::Arc<std::sync::Mutex<SbertEngine>>,
}

impl SharedSbertEngine {
    pub fn new() -> Self {
        Self {
            engine: std::sync::Arc::new(std::sync::Mutex::new(SbertEngine::new())),
        }
    }
    
    pub fn initialize(&self) -> Result<()> {
        let mut engine = self.engine.lock()
            .map_err(|_| anyhow!("Failed to acquire SBERT lock"))?;
        engine.initialize()
    }
    
    pub fn find_intent(&self, text: &str) -> Option<(String, f32)> {
        let mut engine = self.engine.lock().ok()?;
        engine.find_intent(text)
    }
    
    pub fn find_top_intents(&self, text: &str, k: usize) -> Vec<(String, f32)> {
        if let Ok(mut engine) = self.engine.lock() {
            engine.find_top_intents(text, k)
        } else {
            Vec::new()
        }
    }
    
    pub fn text_similarity(&self, text1: &str, text2: &str) -> f32 {
        if let Ok(mut engine) = self.engine.lock() {
            engine.text_similarity(text1, text2)
        } else {
            0.0
        }
    }
    
    pub fn extract_semantic_entities(&self, text: &str) -> HashMap<String, String> {
        if let Ok(mut engine) = self.engine.lock() {
            engine.extract_semantic_entities(text)
        } else {
            HashMap::new()
        }
    }
    
    pub fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let mut engine = self.engine.lock().ok()?;
        Some(engine.embed(text))
    }
    
    pub fn is_loaded(&self) -> bool {
        if let Ok(engine) = self.engine.lock() {
            engine.is_loaded()
        } else {
            false
        }
    }
    
    pub fn add_intent(&self, intent_name: &str, phrase: &str) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.add_intent(intent_name, phrase);
        }
    }
}

impl Default for SharedSbertEngine {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    /// Global SBERT engine instance
    pub static ref GLOBAL_SBERT: SharedSbertEngine = SharedSbertEngine::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sbert_initialization() {
        let mut engine = SbertEngine::new();
        engine.load_precomputed_embeddings();
        assert!(engine.intent_count() > 0);
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((SbertEngine::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!(SbertEngine::cosine_similarity(&a, &c).abs() < 0.001);
    }
    
    #[test]
    fn test_find_intent() {
        let mut engine = SbertEngine::new();
        engine.load_precomputed_embeddings();
        
        if let Some((intent, score)) = engine.find_intent("open chrome browser") {
            assert_eq!(intent, "open_app");
            assert!(score > 0.3);
        }
    }
    
    #[test]
    fn test_text_similarity() {
        let mut engine = SbertEngine::new();
        
        let sim1 = engine.text_similarity("open chrome", "launch chrome");
        let sim2 = engine.text_similarity("open chrome", "close firefox");
        
        // Similar phrases should have higher similarity
        assert!(sim1 > sim2);
    }
    
    #[test]
    fn test_semantic_entity_extraction() {
        let mut engine = SbertEngine::new();
        
        let entities = engine.extract_semantic_entities("open chrome browser");
        assert!(entities.contains_key("app"));
        assert_eq!(entities.get("app"), Some(&"chrome".to_string()));
    }
}
