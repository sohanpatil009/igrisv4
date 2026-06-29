// src/context_memory.rs
// Context memory for multi-turn conversations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};

/// Maximum number of conversation turns to remember
const MAX_CONTEXT_SIZE: usize = 10;

/// A single conversation turn with enhanced NER and SBERT support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub assistant_response: String,
    pub intent: String,
    pub entities: Vec<String>,
    /// Detailed entity map from NER (entity_type -> entity_value)
    pub entity_map: HashMap<String, String>,
    /// Intent confidence score from SBERT
    pub intent_confidence: f32,
    /// Original normalized input
    pub normalized_input: String,
}

impl ConversationTurn {
    pub fn new(
        user_input: String,
        assistant_response: String,
        intent: String,
        entities: Vec<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            user_input,
            assistant_response,
            intent,
            entities,
            entity_map: HashMap::new(),
            intent_confidence: 0.0,
            normalized_input: String::new(),
        }
    }
    
    /// Create a new turn with full NER/SBERT details
    pub fn with_details(
        user_input: String,
        assistant_response: String,
        intent: String,
        entities: Vec<String>,
        entity_map: HashMap<String, String>,
        intent_confidence: f32,
        normalized_input: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            user_input,
            assistant_response,
            intent,
            entities,
            entity_map,
            intent_confidence,
            normalized_input,
        }
    }
}

/// Context memory manager
pub struct ContextMemory {
    history: VecDeque<ConversationTurn>,
    max_size: usize,
    current_topic: Option<String>,
}

impl ContextMemory {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_CONTEXT_SIZE),
            max_size: MAX_CONTEXT_SIZE,
            current_topic: None,
        }
    }

    /// Add a new conversation turn
    pub fn add_turn(&mut self, turn: ConversationTurn) {
        if self.history.len() >= self.max_size {
            self.history.pop_front();
        }
        
        // Update current topic based on intent
        self.current_topic = Some(turn.intent.clone());
        
        self.history.push_back(turn);
    }

    /// Get the last N turns
    pub fn get_recent_turns(&self, n: usize) -> Vec<ConversationTurn> {
        self.history
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get the last user input
    pub fn get_last_user_input(&self) -> Option<String> {
        self.history.back().map(|turn| turn.user_input.clone())
    }

    /// Get the last assistant response
    pub fn get_last_response(&self) -> Option<String> {
        self.history.back().map(|turn| turn.assistant_response.clone())
    }

    /// Get the current topic
    pub fn get_current_topic(&self) -> Option<String> {
        self.current_topic.clone()
    }

    /// Check if a topic was recently discussed
    pub fn was_topic_discussed(&self, topic: &str, within_last_n: usize) -> bool {
        self.get_recent_turns(within_last_n)
            .iter()
            .any(|turn| turn.intent.contains(topic))
    }

    /// Get all entities mentioned in recent conversation
    pub fn get_recent_entities(&self, within_last_n: usize) -> Vec<String> {
        let mut entities = Vec::new();
        for turn in self.get_recent_turns(within_last_n) {
            entities.extend(turn.entities);
        }
        entities.dedup();
        entities
    }

    /// Resolve pronoun references (it, that, this, etc.)
    pub fn resolve_reference(&self, input: &str) -> String {
        let input_lower = input.to_lowercase();
        
        // Check for pronouns
        if input_lower.contains("it") || input_lower.contains("that") || input_lower.contains("this") {
            // Get the last mentioned entity
            if let Some(last_turn) = self.history.back() {
                if !last_turn.entities.is_empty() {
                    let entity = &last_turn.entities[0];
                    return input
                        .replace("it", entity)
                        .replace("It", entity)
                        .replace("that", entity)
                        .replace("That", entity)
                        .replace("this", entity)
                        .replace("This", entity);
                }
            }
        }
        
        input.to_string()
    }

    /// Get conversation summary
    pub fn get_summary(&self) -> String {
        if self.history.is_empty() {
            return "No conversation history".to_string();
        }

        let turn_count = self.history.len();
        let topics: Vec<String> = self.history
            .iter()
            .map(|turn| turn.intent.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        format!(
            "Conversation: {} turns, Topics: {}",
            turn_count,
            topics.join(", ")
        )
    }

    /// Clear conversation history
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_topic = None;
    }

    /// Get full conversation history
    pub fn get_all_turns(&self) -> Vec<ConversationTurn> {
        self.history.iter().cloned().collect()
    }

    /// Save conversation to file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load conversation from file
    pub fn load_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        self.history = serde_json::from_str(&json)?;
        Ok(())
    }
}

impl Default for ContextMemory {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    /// Global context memory instance
    pub static ref GLOBAL_CONTEXT: Arc<Mutex<ContextMemory>> = 
        Arc::new(Mutex::new(ContextMemory::new()));
}

/// Add a conversation turn to global context
pub fn add_to_context(
    user_input: String,
    assistant_response: String,
    intent: String,
    entities: Vec<String>,
) {
    let turn = ConversationTurn::new(user_input, assistant_response, intent, entities);
    GLOBAL_CONTEXT.lock().unwrap().add_turn(turn);
}

/// Add a full conversation turn with NER and SBERT details
pub fn add_to_context_with_details(
    user_input: String,
    assistant_response: String,
    intent: String,
    entities: Vec<String>,
    entity_map: HashMap<String, String>,
    intent_confidence: f32,
    normalized_input: String,
) {
    let turn = ConversationTurn::with_details(
        user_input,
        assistant_response,
        intent,
        entities,
        entity_map,
        intent_confidence,
        normalized_input,
    );
    GLOBAL_CONTEXT.lock().unwrap().add_turn(turn);
}

/// Get recent conversation context
pub fn get_recent_context(n: usize) -> Vec<ConversationTurn> {
    GLOBAL_CONTEXT.lock().unwrap().get_recent_turns(n)
}

/// Resolve references in user input
pub fn resolve_references(input: &str) -> String {
    GLOBAL_CONTEXT.lock().unwrap().resolve_reference(input)
}

/// Get conversation summary
pub fn get_context_summary() -> String {
    GLOBAL_CONTEXT.lock().unwrap().get_summary()
}

/// Clear conversation history
pub fn clear_context() {
    GLOBAL_CONTEXT.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_turn() {
        let mut memory = ContextMemory::new();
        let turn = ConversationTurn::new(
            "open chrome".to_string(),
            "Opening Chrome".to_string(),
            "open_app".to_string(),
            vec!["chrome".to_string()],
        );
        memory.add_turn(turn);
        assert_eq!(memory.history.len(), 1);
    }

    #[test]
    fn test_resolve_reference() {
        let mut memory = ContextMemory::new();
        let turn = ConversationTurn::new(
            "open chrome".to_string(),
            "Opening Chrome".to_string(),
            "open_app".to_string(),
            vec!["chrome".to_string()],
        );
        memory.add_turn(turn);
        
        let resolved = memory.resolve_reference("close it");
        assert!(resolved.contains("chrome"));
    }

    #[test]
    fn test_max_size() {
        let mut memory = ContextMemory::new();
        for i in 0..15 {
            let turn = ConversationTurn::new(
                format!("command {}", i),
                format!("response {}", i),
                "test".to_string(),
                vec![],
            );
            memory.add_turn(turn);
        }
        assert_eq!(memory.history.len(), MAX_CONTEXT_SIZE);
    }
}
