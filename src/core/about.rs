// src/core/about.rs
// IGRIS self-introduction module
// When user asks "tell me about yourself", IGRIS explains its capabilities
// Includes architecture and flowchart explanations

/// IGRIS introduction sections
pub struct IgrisAbout;

impl IgrisAbout {
    /// Short introduction (for quick response)
    pub fn short_intro() -> &'static str {
        "I am IGRIS, your AI-powered voice assistant for desktop. \
         I run completely offline, ensuring your privacy. \
         I can open apps, take photos, record videos, and much more. \
         Just say 'Arise' or press Control Shift Space to activate me anytime."
    }

    /// Full introduction (detailed explanation)
    pub fn full_intro() -> String {
        format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
            Self::intro_section(),
            Self::architecture_section(),
            Self::voice_flow_section(),
            Self::nlu_section(),
            Self::apps_section(),
            Self::camera_section(),
            Self::tech_section()
        )
    }

    /// Introduction section
    fn intro_section() -> &'static str {
        "Hello! I am IGRIS, your personal AI Shadow Monarch. \
         I am a fully offline, privacy-focused voice assistant built with Rust. \
         Unlike cloud-based assistants, everything I do runs locally on your machine. \
         No internet required, no data leaves your computer."
    }

    /// Main system architecture explanation
    pub fn architecture_section() -> &'static str {
        "Let me explain my architecture. \
         I have a modular design with several key components. \
         At the input layer, I receive audio from your microphone and hotkey triggers. \
         The core processing layer handles voice activity detection, wake word recognition, speech-to-text with Whisper, and natural language understanding. \
         My plugin system manages over 50 applications across categories like browsers, office, media, communication, creative tools, and gaming. \
         The output layer includes Piper text-to-speech for voice responses, Dioxus UI for visual feedback, and action executors for system commands. \
         Supporting modules include the Setup Manager for auto-downloading models and Config for personality and permissions."
    }

    /// Voice processing flow explanation
    pub fn voice_flow_section() -> &'static str {
        "Here's how my voice processing flow works, step by step. \
         Step 1: Audio capture using CPAL at 16 kilohertz mono. \
         Step 2: Voice Activity Detection analyzes energy levels and zero-crossing rate to detect speech. \
         Step 3: Wake word detection listens for 'Arise' using fuzzy matching. \
         Step 4: Once activated, Whisper base model converts your speech to text. \
         Step 5: My NLU engine with SBERT extracts intent and entities from the text. \
         Step 6: The command router executes the appropriate action through the plugin system. \
         The entire pipeline runs in about 1.5 seconds, completely offline."
    }

    /// NLU module explanation
    pub fn nlu_section() -> &'static str {
        "My Natural Language Understanding module is the brain of the operation. \
         It has four main components. \
         First, SBERT generates 384-dimensional semantic embeddings to understand meaning, not just keywords. \
         Second, Named Entity Recognition extracts applications, files, folders, numbers, and system actions from your commands. \
         Third, the Context Manager maintains conversation state for follow-up commands. \
         Fourth, the Intent Classifier determines what you want to do: open app, close app, camera control, system control, web search, and more. \
         All these work together to understand natural language like 'can you please open chrome for me'."
    }

    /// Apps section
    fn apps_section() -> &'static str {
        "My plugin system architecture supports over 50 applications. \
         Plugins are organized into categories: \
         Browsers including Chrome, Firefox, Edge, Brave, and Opera. \
         Office apps like Word, Excel, PowerPoint, and Outlook. \
         Media players including Spotify, VLC, Netflix, and YouTube. \
         Communication tools like Discord, Zoom, Teams, Slack, and Telegram. \
         Creative software including Photoshop, Premiere, Figma, Blender, and Canva. \
         Gaming platforms like Steam, Epic Games, and GOG. \
         Code editors including VS Code, Sublime Text, and Notepad plus plus. \
         All plugins are compiled into the binary, no external configuration needed."
    }

    /// Camera section
    fn camera_section() -> &'static str {
        "My camera module uses FFmpeg for reliable photo and video capture. \
         The flow is simple: say 'open camera' to activate, 'take photo' to capture a picture, \
         'start recording' for video with audio, and 'stop recording' to save. \
         On Windows I use DirectShow, on Linux V4L2, and on macOS AVFoundation. \
         Videos are encoded with H.264 and AAC audio at 128 kilobits per second. \
         All media saves to your Pictures and Videos folders under an IGRIS subfolder."
    }

    /// Setup manager flow
    pub fn setup_flow_section() -> &'static str {
        "My Setup Manager handles first-time initialization. \
         On first launch, it checks the package folder for required components. \
         The Downloader fetches Whisper model at 81 megabytes, Piper TTS with voice model, SBERT for NLU, and FFmpeg for media. \
         Downloads are platform-specific: Windows gets zip files, Linux gets tar archives. \
         The Extractor unpacks everything to the correct locations. \
         The Validator verifies all files are present and working. \
         After extraction, the downloads folder is automatically deleted to save space. \
         Total size is about 200 megabytes for all models."
    }

    /// Tech stack section
    fn tech_section() -> &'static str {
        "My technology stack is optimized for performance and privacy. \
         Core language is Rust for memory safety and speed. \
         UI framework is Dioxus 0.7 for cross-platform desktop apps. \
         Speech recognition uses Whisper dot cpp with the base quantized model. \
         Text-to-speech is Piper with the LibriTTS voice model. \
         NLU uses SBERT MiniLM for semantic understanding. \
         Media capture uses FFmpeg for video and audio. \
         Networking uses Rustls for TLS encryption. \
         Audio capture uses CPAL for cross-platform microphone access. \
         Async runtime is Tokio for concurrent operations. \
         The entire application compiles to a single binary of about 15 megabytes."
    }

    /// Get response based on query type
    pub fn get_response(detailed: bool) -> String {
        if detailed {
            Self::full_intro()
        } else {
            Self::short_intro().to_string()
        }
    }

    /// Get specific section
    pub fn get_section(section: AboutSection) -> &'static str {
        match section {
            AboutSection::Intro => Self::intro_section(),
            AboutSection::Architecture => Self::architecture_section(),
            AboutSection::VoiceFlow => Self::voice_flow_section(),
            AboutSection::Nlu => Self::nlu_section(),
            AboutSection::Apps => Self::apps_section(),
            AboutSection::Camera => Self::camera_section(),
            AboutSection::SetupFlow => Self::setup_flow_section(),
            AboutSection::Tech => Self::tech_section(),
        }
    }
}

/// Sections of IGRIS introduction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AboutSection {
    Intro,
    Architecture,
    VoiceFlow,
    Nlu,
    Apps,
    Camera,
    SetupFlow,
    Tech,
}

impl AboutSection {
    /// Parse section from user query
    pub fn from_query(query: &str) -> Option<Self> {
        let query_lower = query.to_lowercase();
        
        // Architecture queries
        if query_lower.contains("architecture") || query_lower.contains("design") 
            || query_lower.contains("structure") || query_lower.contains("how are you built")
            || query_lower.contains("system design") {
            Some(Self::Architecture)
        }
        // Voice flow queries
        else if query_lower.contains("voice flow") || query_lower.contains("voice processing")
            || query_lower.contains("how do you listen") || query_lower.contains("how do you hear")
            || query_lower.contains("speech pipeline") || query_lower.contains("audio flow") {
            Some(Self::VoiceFlow)
        }
        // NLU queries
        else if query_lower.contains("nlu") || query_lower.contains("understand")
            || query_lower.contains("language") || query_lower.contains("intent")
            || query_lower.contains("how do you process") {
            Some(Self::Nlu)
        }
        // App queries
        else if query_lower.contains("app") || query_lower.contains("plugin")
            || query_lower.contains("launch") || query_lower.contains("open") {
            Some(Self::Apps)
        }
        // Camera queries
        else if query_lower.contains("camera") || query_lower.contains("photo") 
            || query_lower.contains("video") || query_lower.contains("record") {
            Some(Self::Camera)
        }
        // Setup queries
        else if query_lower.contains("setup") || query_lower.contains("install")
            || query_lower.contains("download") || query_lower.contains("first time") {
            Some(Self::SetupFlow)
        }
        // Tech stack queries
        else if query_lower.contains("tech") || query_lower.contains("built with")
            || query_lower.contains("stack") || query_lower.contains("technology") {
            Some(Self::Tech)
        }
        else {
            None
        }
    }
    
    /// Get all sections for full explanation
    pub fn all_sections() -> Vec<Self> {
        vec![
            Self::Intro,
            Self::Architecture,
            Self::VoiceFlow,
            Self::Nlu,
            Self::Apps,
            Self::Camera,
            Self::SetupFlow,
            Self::Tech,
        ]
    }
}

/// Check if query is asking about IGRIS
pub fn is_about_query(query: &str) -> bool {
    let query_lower = query.to_lowercase();
    
    // Check for "tell me about yourself" patterns
    let about_patterns = [
        "tell me about yourself",
        "tell me about you",
        "about yourself",
        "about you",
        "who are you",
        "what are you",
        "what can you do",
        "what do you do",
        "introduce yourself",
        "your capabilities",
        "your features",
        "igris tell me",
        "tell me about igris",
        "what is igris",
    ];
    
    about_patterns.iter().any(|p| query_lower.contains(p))
}

/// Check if query is asking for detailed info
pub fn wants_detailed_info(query: &str) -> bool {
    let query_lower = query.to_lowercase();
    
    let detailed_patterns = [
        "detail",
        "everything",
        "all",
        "full",
        "complete",
        "explain",
    ];
    
    detailed_patterns.iter().any(|p| query_lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_about_query_detection() {
        assert!(is_about_query("tell me about yourself"));
        assert!(is_about_query("who are you"));
        assert!(is_about_query("what can you do"));
        assert!(is_about_query("igris tell me about yourself"));
        assert!(!is_about_query("open chrome"));
    }

    #[test]
    fn test_section_detection() {
        assert_eq!(AboutSection::from_query("explain your architecture"), Some(AboutSection::Architecture));
        assert_eq!(AboutSection::from_query("how does voice processing work"), Some(AboutSection::VoiceFlow));
        assert_eq!(AboutSection::from_query("how do you understand me"), Some(AboutSection::Nlu));
        assert_eq!(AboutSection::from_query("what apps can you open"), Some(AboutSection::Apps));
        assert_eq!(AboutSection::from_query("camera features"), Some(AboutSection::Camera));
        assert_eq!(AboutSection::from_query("setup process"), Some(AboutSection::SetupFlow));
        assert_eq!(AboutSection::from_query("tech stack"), Some(AboutSection::Tech));
    }
    
    #[test]
    fn test_all_sections() {
        let sections = AboutSection::all_sections();
        assert_eq!(sections.len(), 8);
    }
}
