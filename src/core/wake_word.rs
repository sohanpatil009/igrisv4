// Wake word detection - minimal output
use crate::core::audio_capture::{capture_audio_vad, CaptureConfig, CaptureMode};
use crate::core::stt::{transcribe_audio, SttEngine};
use std::sync::atomic::Ordering;

/// Listen for wake word with minimal output
pub fn listen_for_wake_word(
    stt: &SttEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let wake_word = "hello";
    
    loop {
        // Check for reset signal from hotkey
        if crate::RESET_FLAG.swap(false, Ordering::Relaxed) {
            return Err("Reset by hotkey".into());
        }
        
        let capture_config = CaptureConfig {
            mode: CaptureMode::WakeWord,
            max_wait_ms: 10000,
            debug: false,
        };

        let result = capture_audio_vad(capture_config)?;

        if !result.speech_detected || result.samples.is_empty() {
            continue;
        }

        let transcription = match transcribe_audio(&result.samples, stt) {
            Ok(text) => text,
            Err(_) => continue,
        };

        if transcription.trim().is_empty() {
            continue;
        }

        let transcription_lower = transcription.to_lowercase();
        
        if contains_wake_word(&transcription_lower, wake_word) {
            println!("✅ Wake word detected: \"{}\"", transcription);
            return Ok(());
        }

        if transcription_lower.len() > 2 {
            println!("❌ Wake word not detected. Heard: \"{}\"", transcription_lower);
        }
    }
}

fn contains_wake_word(transcription: &str, wake_word: &str) -> bool {
    let transcription_clean = transcription.trim().to_lowercase();
    
    if transcription_clean == wake_word || transcription_clean.contains(wake_word) {
        return true;
    }

    let variations = vec![
        "hello", "hallo", "hullo", "halo", "helo",
        "hello!", "hello.", "helloo", "hellooo", "hellow",
        "hllo", "helo", "ello", "hlo",
        "ok hello", "hello there", "computer hello", "assistant hello",
    ];

    for variant in variations {
        if transcription_clean.contains(variant) {
            return true;
        }
    }
    
    levenshtein_distance(&transcription_clean, wake_word) <= 2
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    
    if len1 == 0 { return len2; }
    if len2 == 0 { return len1; }
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 { matrix[i][0] = i; }
    for j in 0..=len2 { matrix[0][j] = j; }
    
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost
            );
        }
    }
    
    matrix[len1][len2]
}
