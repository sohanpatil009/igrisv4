// src/commands/about.rs
// Handle "tell me about yourself" and similar queries
// IGRIS explains its capabilities using TTS with animated presentation UI

use crate::core::about::{IgrisAbout, AboutSection, is_about_query, wants_detailed_info};
use crate::core::tts::speak;
use crate::ui::presentation::{start_presentation, stop_presentation};

/// Handle about/introduction queries
pub fn handle_about_command(query: &str) -> Result<String, String> {
    println!("[ABOUT] Processing query: {}", query);
    
    // Check if wants full presentation
    let wants_presentation = wants_detailed_info(query) 
        || query.to_lowercase().contains("yourself")
        || query.to_lowercase().contains("about you")
        || query.to_lowercase().contains("who are you")
        || query.to_lowercase().contains("introduce");
    
    if wants_presentation {
        // Start animated presentation with TTS
        println!("[ABOUT] Starting presentation mode");
        start_presentation();
        return Ok("Starting presentation...".to_string());
    }
    
    // Check if asking about specific section
    if let Some(section) = AboutSection::from_query(query) {
        let response = IgrisAbout::get_section(section);
        
        // Speak the response
        speak_response(response);
        
        return Ok(response.to_string());
    }
    
    // Default: short intro without presentation
    let short = IgrisAbout::short_intro();
    speak_response(short);
    Ok(short.to_string())
}

/// Speak a response using TTS
fn speak_response(text: &str) {
    // Run TTS in background thread to not block
    let text_owned = text.to_string();
    std::thread::spawn(move || {
        if let Err(e) = speak(&text_owned) {
            eprintln!("[ABOUT] TTS error: {}", e);
        }
    });
}

/// Check if the query is an about/introduction query
pub fn is_about_command(query: &str) -> bool {
    is_about_query(query)
}

/// Stop the presentation
pub fn stop_about_presentation() {
    stop_presentation();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_about_detection() {
        assert!(is_about_command("tell me about yourself"));
        assert!(is_about_command("who are you"));
        assert!(is_about_command("what can you do"));
        assert!(!is_about_command("open chrome"));
    }
}
