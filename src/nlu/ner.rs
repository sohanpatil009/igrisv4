// src/ner.rs - Named Entity Recognition
// Extracts structured entities from natural language commands

use regex::Regex;
use std::collections::HashMap;

/// Entity types that can be recognized
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// Application name (e.g., "chrome", "firefox")
    Application,
    /// File name or path
    File,
    /// Folder/directory name or path
    Folder,
    /// Number/quantity
    Number,
    /// Percentage value
    Percentage,
    /// Time duration
    Duration,
    /// System action (shutdown, restart, etc.)
    SystemAction,
    /// Volume level
    VolumeLevel,
    /// Brightness level
    BrightnessLevel,
    /// Network interface (wifi, bluetooth)
    NetworkInterface,
    /// Generic entity
    Generic,
}

/// Recognized entity with type and value
#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub confidence: f32,
}

impl Entity {
    pub fn new(entity_type: EntityType, value: String, start_pos: usize, end_pos: usize) -> Self {
        Self {
            entity_type,
            value,
            start_pos,
            end_pos,
            confidence: 1.0,
        }
    }
    
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

/// NER engine for extracting entities from text
pub struct NerEngine {
    patterns: HashMap<EntityType, Vec<Regex>>,
    application_names: Vec<String>,
}

impl NerEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            patterns: HashMap::new(),
            application_names: Vec::new(),
        };
        
        engine.initialize_patterns();
        engine.initialize_application_names();
        engine
    }
    
    /// Initialize regex patterns for entity extraction
    fn initialize_patterns(&mut self) {
        // Number patterns
        self.add_pattern(
            EntityType::Number,
            vec![
                r"\b(\d+)\b",
                r"\b(one|two|three|four|five|six|seven|eight|nine|ten)\b",
                r"\b(twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety|hundred)\b",
            ],
        );
        
        // Percentage patterns — full match span prevents overlap with Number
        self.add_pattern(
            EntityType::Percentage,
            vec![
                r"\b(\d+)\s*(?:percent|%)\b",
                r"\b(half|quarter|full)\b",
            ],
        );
        
        // Duration patterns — capture group 1 includes both number and unit
        self.add_pattern(
            EntityType::Duration,
            vec![
                r"\b(\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?))\b",
                r"\bin\s+(\d+\s*(?:seconds?|minutes?|hours?))\b",
                r"\bafter\s+(\d+\s*(?:seconds?|minutes?|hours?))\b",
            ],
        );
        
        // File patterns
        self.add_pattern(
            EntityType::File,
            vec![
                r"\bfile\s+([a-zA-Z0-9._\-/\\]+\.[a-zA-Z0-9]+)\b",
                r"\b([a-zA-Z0-9._\-]+\.(?:txt|pdf|doc|docx|jpg|png|mp3|mp4|zip|exe))\b",
            ],
        );
        
        // Folder patterns
        self.add_pattern(
            EntityType::Folder,
            vec![
                r"\bfolder\s+([a-zA-Z0-9._\-/\\]+)\b",
                r"\bdirectory\s+([a-zA-Z0-9._\-/\\]+)\b",
                r"\bpath\s+([a-zA-Z0-9._\-/\\:]+)\b",
            ],
        );
        
        // System action patterns
        self.add_pattern(
            EntityType::SystemAction,
            vec![
                r"\b(shutdown|restart|reboot|sleep|hibernate|lock)\b",
                r"\b(turn\s+off|power\s+off|shut\s+down)\b",
            ],
        );
        
        // Volume level patterns
        self.add_pattern(
            EntityType::VolumeLevel,
            vec![
                r"\bvolume\s+(?:to\s+)?(\d+)\b",
                r"\bvolume\s+(?:to\s+)?(\d+)%\b",
                r"\b(increase|decrease|raise|lower)\s+volume\s+(?:by\s+)?(\d+)\b",
                r"\b(mute|unmute|max|maximum|min|minimum)\b",
            ],
        );
        
        // Brightness level patterns
        self.add_pattern(
            EntityType::BrightnessLevel,
            vec![
                r"\bbrightness\s+(?:to\s+)?(\d+)\b",
                r"\bbrightness\s+(?:to\s+)?(\d+)%\b",
                r"\b(increase|decrease|raise|lower)\s+brightness\s+(?:by\s+)?(\d+)\b",
            ],
        );
        
        // Network interface patterns
        self.add_pattern(
            EntityType::NetworkInterface,
            vec![
                r"\b(wifi|wi-fi|wireless)\b",
                r"\b(bluetooth|bt)\b",
                r"\b(ethernet|lan)\b",
            ],
        );
    }
    
    /// Add patterns for an entity type
    fn add_pattern(&mut self, entity_type: EntityType, patterns: Vec<&str>) {
        let compiled_patterns: Vec<Regex> = patterns
            .iter()
            .filter_map(|p| Regex::new(&format!("(?i){}", p)).ok())
            .collect();
        
        self.patterns.insert(entity_type, compiled_patterns);
    }
    
    /// Initialize application names from plugins
    /// All applications are now managed via the plugin system
    fn initialize_application_names(&mut self) {
        let mut app_names = Vec::new();

        // Load all applications from plugin manager
        let plugins = crate::plugins::get_all_plugins();
        for plugin in plugins {
            // Add plugin name as searchable entity
            app_names.push(plugin.metadata.name.clone());
            
            // Add all commands from this plugin
            for command in &plugin.commands {
                // Add full trigger
                app_names.push(command.trigger.clone());
                
                // Add individual keywords from trigger for better matching
                for word in command.trigger.split_whitespace() {
                    if word.len() > 2 && !word.eq("the") && !word.eq("and") && !word.eq("open") {
                        app_names.push(word.to_string());
                    }
                }
                
                // Add examples as alternative keywords
                for example in &command.examples {
                    app_names.push(example.clone());
                }
            }
            
            // Add plugin keywords
            for keyword in &plugin.metadata.keywords {
                if keyword.len() > 2 {
                    app_names.push(keyword.clone());
                }
            }
        }

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        app_names.retain(|x| seen.insert(x.clone()));

        self.application_names = app_names;
    }
    
    /// Extract all entities from text
    pub fn extract_entities(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();
        
        // Extract application names first (highest priority)
        entities.extend(self.extract_applications(&text_lower));
        
        // Extract non-Number entity types (more specific types take priority)
        for (entity_type, patterns) in &self.patterns {
            if *entity_type == EntityType::Number {
                continue;
            }
            for pattern in patterns {
                self.match_and_add(pattern, &text_lower, entity_type, &mut entities);
            }
        }
        
        // Extract Number entities last (generic, only where nothing more specific matched)
        if let Some(patterns) = self.patterns.get(&EntityType::Number) {
            for pattern in patterns {
                self.match_and_add(pattern, &text_lower, &EntityType::Number, &mut entities);
            }
        }
        
        // Sort by position in text
        entities.sort_by_key(|e| e.start_pos);
        
        entities
    }
    
    fn match_and_add(
        &self,
        pattern: &Regex,
        text: &str,
        entity_type: &EntityType,
        entities: &mut Vec<Entity>,
    ) {
        if let Some(captures) = pattern.captures(text) {
            if let Some(matched) = captures.get(1) {
                let full_match = captures.get(0).unwrap();
                let entity = Entity::new(
                    entity_type.clone(),
                    matched.as_str().to_string(),
                    full_match.start(),
                    full_match.end(),
                );
                if !self.overlaps_with_existing(&entity, entities) {
                    entities.push(entity);
                }
            }
        }
    }
    
    /// Extract application names from text
    fn extract_applications(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        for app_name in &self.application_names {
            if let Some(pos) = text.find(app_name) {
                entities.push(Entity::new(
                    EntityType::Application,
                    app_name.clone(),
                    pos,
                    pos + app_name.len(),
                ).with_confidence(0.9));
            }
        }
        
        entities
    }
    
    /// Check if entity overlaps with existing entities.
    /// Only blocks same-type overlaps (prevents duplicate Number or duplicate Application).
    /// Different entity types at the same span are both valuable (e.g. "20" is both a Number and a Percentage).
    fn overlaps_with_existing(&self, entity: &Entity, existing: &[Entity]) -> bool {
        existing.iter().any(|e| {
            e.entity_type == entity.entity_type
                && ((entity.start_pos >= e.start_pos && entity.start_pos < e.end_pos)
                    || (entity.end_pos > e.start_pos && entity.end_pos <= e.end_pos)
                    || (entity.start_pos <= e.start_pos && entity.end_pos >= e.end_pos))
        })
    }
    
    /// Extract specific entity type from text
    pub fn extract_entity_type(&self, text: &str, entity_type: EntityType) -> Option<Entity> {
        self.extract_entities(text)
            .into_iter()
            .find(|e| e.entity_type == entity_type)
    }
    
    /// Extract all entities of a specific type
    pub fn extract_all_of_type(&self, text: &str, entity_type: EntityType) -> Vec<Entity> {
        self.extract_entities(text)
            .into_iter()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }
    
    /// Convert word numbers to digits
    pub fn word_to_number(&self, word: &str) -> Option<u32> {
        match word.to_lowercase().as_str() {
            "zero" => Some(0),
            "one" => Some(1),
            "two" => Some(2),
            "three" => Some(3),
            "four" => Some(4),
            "five" => Some(5),
            "six" => Some(6),
            "seven" => Some(7),
            "eight" => Some(8),
            "nine" => Some(9),
            "ten" => Some(10),
            "twenty" => Some(20),
            "thirty" => Some(30),
            "forty" => Some(40),
            "fifty" => Some(50),
            "sixty" => Some(60),
            "seventy" => Some(70),
            "eighty" => Some(80),
            "ninety" => Some(90),
            "hundred" => Some(100),
            "half" => Some(50),
            "quarter" => Some(25),
            "full" => Some(100),
            _ => word.parse::<u32>().ok(),
        }
    }
    
    /// Extract numeric value from entity
    pub fn extract_number(&self, entity: &Entity) -> Option<u32> {
        self.word_to_number(&entity.value)
    }
    
    /// Parse percentage from text
    pub fn parse_percentage(&self, text: &str) -> Option<u8> {
        if let Some(entity) = self.extract_entity_type(text, EntityType::Percentage) {
            return self.extract_number(&entity).map(|n| n.min(100) as u8);
        }
        
        // Try direct number extraction
        if let Some(entity) = self.extract_entity_type(text, EntityType::Number) {
            return self.extract_number(&entity).map(|n| n.min(100) as u8);
        }
        
        None
    }
    
    /// Parse duration in seconds
    pub fn parse_duration_seconds(&self, text: &str) -> Option<u64> {
        if let Some(entity) = self.extract_entity_type(text, EntityType::Duration) {
            let value_str = &entity.value;
            
            // Extract number and unit
            let re = Regex::new(r"(\d+)\s*(seconds?|secs?|minutes?|mins?|hours?|hrs?)").ok()?;
            if let Some(captures) = re.captures(value_str) {
                let number = captures.get(1)?.as_str().parse::<u64>().ok()?;
                let unit = captures.get(2)?.as_str().to_lowercase();
                
                let multiplier = match unit.as_str() {
                    "second" | "seconds" | "sec" | "secs" => 1,
                    "minute" | "minutes" | "min" | "mins" => 60,
                    "hour" | "hours" | "hr" | "hrs" => 3600,
                    _ => 1,
                };
                
                return Some(number * multiplier);
            }
        }
        
        None
    }
}

impl Default for NerEngine {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    /// Global NER engine instance
    pub static ref GLOBAL_NER: NerEngine = NerEngine::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_application() {
        let ner = NerEngine::new();
        let entities = ner.extract_entities("open chrome browser");
        assert!(entities.iter().any(|e| e.entity_type == EntityType::Application));
    }
    
    #[test]
    fn test_extract_number() {
        let ner = NerEngine::new();
        let entities = ner.extract_entities("set volume to 50");
        assert!(entities.iter().any(|e| e.entity_type == EntityType::Number));
    }
    
    #[test]
    fn test_parse_percentage() {
        let ner = NerEngine::new();
        assert_eq!(ner.parse_percentage("increase volume by 20 percent"), Some(20));
        assert_eq!(ner.parse_percentage("set to 75%"), Some(75));
    }
    
    #[test]
    fn test_parse_duration() {
        let ner = NerEngine::new();
        assert_eq!(ner.parse_duration_seconds("in 5 minutes"), Some(300));
        assert_eq!(ner.parse_duration_seconds("after 2 hours"), Some(7200));
    }
    
    #[test]
    fn test_word_to_number() {
        let ner = NerEngine::new();
        assert_eq!(ner.word_to_number("fifty"), Some(50));
        assert_eq!(ner.word_to_number("half"), Some(50));
    }
}
