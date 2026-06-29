// src/nlu/nlu_processor.rs - Unified NLU processing with NER and SBERT integration
// Provides high-level interface combining NER, SBERT, and Intent recognition

use crate::nlu::engine::GLOBAL_NLU;
use crate::nlu::context::add_to_context_with_details;
use anyhow::Result;
use std::collections::HashMap;

/// Complete NLU processing result combining all components
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProcessedInput {
    /// The original user input
    pub original_input: String,
    /// Normalized/cleaned input
    pub normalized_input: String,
    /// Detected intent
    pub intent: String,
    /// Intent confidence (from SBERT)
    pub confidence: f32,
    /// Extracted entities (simple list for backward compatibility)
    pub entities: Vec<String>,
    /// Detailed entity map from NER (entity_type -> value)
    pub entity_map: HashMap<String, String>,
    /// Whether processing was successful
    pub is_valid: bool,
}

impl ProcessedInput {
    pub fn unknown() -> Self {
        Self {
            original_input: String::new(),
            normalized_input: String::new(),
            intent: "UnknownIntent".to_string(),
            confidence: 0.0,
            entities: Vec::new(),
            entity_map: HashMap::new(),
            is_valid: false,
        }
    }
    
    /// Check if a specific entity type is present
    pub fn has_entity(&self, entity_type: &str) -> bool {
        self.entity_map.contains_key(entity_type)
    }
    
    /// Get entity value by type
    pub fn get_entity(&self, entity_type: &str) -> Option<String> {
        self.entity_map.get(entity_type).cloned()
    }
    
    /// Get all values for a specific entity type (for multiple occurrences)
    pub fn get_all_entities(&self, entity_type: &str) -> Vec<String> {
        self.entity_map
            .iter()
            .filter(|(k, _)| k.starts_with(entity_type))
            .map(|(_, v)| v.clone())
            .collect()
    }
}

/// Process user input with full NLU pipeline
/// Integrates NER, SBERT, and intent matching
#[allow(dead_code)]
pub fn process_user_input(input: &str) -> Result<ProcessedInput> {
    // Process through the main NLU engine
    let intent_result = GLOBAL_NLU.process_input(input)?;
    
    // Convert to our unified format
    let processed = ProcessedInput {
        original_input: input.to_string(),
        normalized_input: intent_result.normalized_input.clone(),
        intent: intent_result.intent_name.clone(),
        confidence: intent_result.confidence,
        entities: intent_result
            .entities
            .values()
            .cloned()
            .collect(),
        entity_map: intent_result.entities,
        is_valid: intent_result.intent_name != "UnknownIntent",
    };
    
    Ok(processed)
}

/// Process input and add to context in one call
#[allow(dead_code)]
pub fn process_and_remember(
    input: &str,
    response: &str,
) -> Result<ProcessedInput> {
    let processed = process_user_input(input)?;
    
    // Add to context with full details
    add_to_context_with_details(
        input.to_string(),
        response.to_string(),
        processed.intent.clone(),
        processed.entities.clone(),
        processed.entity_map.clone(),
        processed.confidence,
        processed.normalized_input.clone(),
    );
    
    Ok(processed)
}

/// Get detailed entity information from processed input
#[allow(dead_code)]
pub fn extract_entity_details(processed: &ProcessedInput) -> HashMap<String, Vec<String>> {
    let mut details: HashMap<String, Vec<String>> = HashMap::new();
    
    // Group entities by their base type
    for (key, value) in &processed.entity_map {
        // Extract the base type (before any suffixes)
        let base_type = if let Some(idx) = key.find('_') {
            &key[..idx]
        } else {
            key.as_str()
        };
        
        details.entry(base_type.to_string())
            .or_insert_with(Vec::new)
            .push(value.clone());
    }
    
    details
}

/// Semantic similarity check between two inputs
#[allow(dead_code)]
pub fn check_semantic_similarity(text1: &str, text2: &str) -> f32 {
    GLOBAL_NLU.get_similarity(text1, text2)
}

/// Suggest alternative intents if confidence is low
#[allow(dead_code)]
pub fn suggest_intents(_input: &str, _threshold: f32) -> Vec<(String, f32)> {
    // This would be implemented in engine.rs with SBERT's find_top_intents
    // For now, return empty - enhance engine.rs to expose this
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processed_input_unknown() {
        let unknown = ProcessedInput::unknown();
        assert_eq!(unknown.intent, "UnknownIntent");
        assert!(!unknown.is_valid);
    }

    #[test]
    fn test_entity_operations() {
        let mut processed = ProcessedInput::unknown();
        processed.entity_map.insert("app".to_string(), "chrome".to_string());
        processed.entity_map.insert("action".to_string(), "open".to_string());
        
        assert!(processed.has_entity("app"));
        assert_eq!(processed.get_entity("app"), Some("chrome".to_string()));
        assert!(!processed.has_entity("file"));
    }
}
