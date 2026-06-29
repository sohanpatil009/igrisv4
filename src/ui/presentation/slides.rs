// src/ui/presentation/slides.rs
// Slide content and narration for IGRIS presentation

/// Slide content type
#[derive(Clone, PartialEq)]
pub enum SlideContent {
    Title,
    Problem,
    Solution,
    Architecture,
    VoiceFlow,
    NluModule,
    PluginSystem,
    FileShare,
    SetupManager,
    TechStack,
    VoiceCommands,
    SupportedApps,
    ModuleSummary,
    FutureScope,
    ThankYou,
}

/// Single slide with content and narration
#[derive(Clone)]
pub struct Slide {
    pub id: usize,
    pub title: &'static str,
    pub icon: &'static str,
    pub content: SlideContent,
    pub narration: &'static str,
    pub bullet_points: &'static [&'static str],
}

/// All presentation slides
pub const SLIDES: &[Slide] = &[
    // Slide 1: Title
    Slide {
        id: 0,
        title: "IGRIS",
        icon: "🗡️",
        content: SlideContent::Title,
        narration: "Hello! I am IGRIS, your AI-powered voice assistant for desktop. \
                   I am named after the shadow knight from Solo Leveling. \
                   My wake word is 'Arise'. I was developed by Sohan Patil using Rust and Dioxus.",
        bullet_points: &[
            "AI-Powered Voice Assistant",
            "100% Offline & Private",
            "Built with Rust + Dioxus",
            "Wake word: \"Arise\"",
        ],
    },
    // Slide 2: Problem
    Slide {
        id: 1,
        title: "Problem Statement",
        icon: "🎯",
        content: SlideContent::Problem,
        narration: "Let me explain the problems I solve. \
                   Traditional desktop interaction requires keyboard and mouse for everything. \
                   Repetitive tasks like opening apps waste your time. \
                   There's no unified voice control across Windows, Linux, and macOS. \
                   Cloud-based assistants raise privacy concerns. \
                   And file sharing between devices is unnecessarily complex.",
        bullet_points: &[
            "⌨️ Traditional interaction needs keyboard & mouse",
            "🔄 Repetitive tasks waste time",
            "🌐 No unified voice control across platforms",
            "🔒 Privacy concerns with cloud assistants",
            "📁 Complex file sharing between devices",
        ],
    },
    // Slide 3: Solution
    Slide {
        id: 2,
        title: "Solution: IGRIS",
        icon: "💡",
        content: SlideContent::Solution,
        narration: "I am the solution to these problems. \
                   I run fully offline, so your data never leaves your computer. \
                   I'm privacy-focused with no cloud dependency. \
                   I'm fast and native because I'm built with Rust. \
                   And I work on Windows, Linux, and macOS.",
        bullet_points: &[
            "🎤 Natural voice control",
            "🔒 100% Offline - No cloud needed",
            "🚀 Fast & Native with Rust",
            "🌍 Cross-Platform support",
        ],
    },
    // Slide 4: Architecture
    Slide {
        id: 3,
        title: "System Architecture",
        icon: "🏗️",
        content: SlideContent::Architecture,
        narration: "Here's my main system architecture. \
                   At the input layer, I receive audio from your microphone and hotkey triggers with Control Shift Space. \
                   The core processing includes Voice Activity Detection, Wake Word recognition, Whisper speech-to-text, NLU engine, and Command Router. \
                   My plugin system manages over 50 applications. \
                   Output goes through Piper text-to-speech, Dioxus UI, and action executors. \
                   Supporting modules include Setup Manager, Config, and File Share.",
        bullet_points: &[
            "🎤 Input: Microphone + Hotkey (Ctrl+Shift+Space)",
            "⚙️ Core: VAD → Wake Word → Whisper → NLU → Router",
            "🔌 Plugins: 50+ Apps, Camera, File Share, Web",
            "🔊 Output: Piper TTS + Dioxus UI + Actions",
        ],
    },
    // Slide 5: Voice Flow
    Slide {
        id: 4,
        title: "Voice Processing Flow",
        icon: "🎤",
        content: SlideContent::VoiceFlow,
        narration: "Let me walk you through my voice processing pipeline. \
                   Step 1: Audio capture using CPAL at 16 kilohertz mono. \
                   Step 2: Voice Activity Detection analyzes energy and zero-crossing rate. \
                   Step 3: Wake word detection listens for 'Arise' using fuzzy matching. \
                   Step 4: Whisper base model converts speech to text. \
                   Step 5: NLU with SBERT extracts intent and entities. \
                   Step 6: Action execution through the plugin system. \
                   Total response time is about 1.5 seconds, completely offline.",
        bullet_points: &[
            "🎤 Audio (CPAL 16kHz)",
            "🔊 VAD (Energy + ZCR)",
            "👂 Wake Word (\"Arise\")",
            "📝 Whisper STT",
            "🧠 NLU (SBERT)",
            "⚡ Action Execute",
        ],
    },
    // Slide 6: NLU Module
    Slide {
        id: 5,
        title: "NLU Engine",
        icon: "🧠",
        content: SlideContent::NluModule,
        narration: "My Natural Language Understanding engine is the brain of the operation. \
                   SBERT generates 384-dimensional semantic embeddings to understand meaning. \
                   Named Entity Recognition extracts apps, files, numbers, and actions. \
                   Context Manager maintains conversation state for follow-ups. \
                   Intent Classifier determines what you want: open app, camera, search, and more. \
                   This lets me understand natural phrases like 'can you please open chrome for me'.",
        bullet_points: &[
            "🔮 SBERT: 384-dim semantic embeddings",
            "🏷️ NER: Apps, Files, Numbers, Actions",
            "💭 Context: Conversation memory",
            "🎯 Intent: open_app, camera, search...",
        ],
    },
    // Slide 7: Plugin System
    Slide {
        id: 6,
        title: "Plugin System",
        icon: "🔌",
        content: SlideContent::PluginSystem,
        narration: "My plugin system architecture supports over 50 applications. \
                   Browsers: Chrome, Firefox, Edge, Brave, Opera. \
                   Office: Word, Excel, PowerPoint, Outlook. \
                   Media: Spotify, VLC, Netflix, YouTube. \
                   Communication: Discord, Zoom, Teams, Slack. \
                   Creative: Photoshop, Figma, Blender, Canva. \
                   Gaming: Steam, Epic Games, GOG. \
                   Special plugins include Camera with FFmpeg, File Share with TLS, and Web Search. \
                   All plugins are compiled into the binary.",
        bullet_points: &[
            "🌐 Browsers: Chrome, Firefox, Edge, Brave",
            "📄 Office: Word, Excel, PowerPoint",
            "🎵 Media: Spotify, VLC, YouTube",
            "💬 Comms: Discord, Zoom, Teams, Slack",
            "🎨 Creative: Photoshop, Figma, Blender",
            "🎮 Gaming: Steam, Epic Games",
        ],
    },
    // Slide 8: File Share
    Slide {
        id: 7,
        title: "File Share Module",
        icon: "📁",
        content: SlideContent::FileShare,
        narration: "My file sharing module enables secure device-to-device transfers. \
                   Discovery Service broadcasts UDP on port 5354 to find other IGRIS devices. \
                   Trust Manager handles verification with 6-digit codes. \
                   Transfer Manager sends files over TCP port 5355 with TLS encryption. \
                   Each device generates its own certificate for end-to-end encryption. \
                   Works across Windows, Linux, and macOS on the same network.",
        bullet_points: &[
            "🔍 Discovery: UDP broadcast (port 5354)",
            "🤝 Trust: 6-digit verification codes",
            "📤 Transfer: TCP + TLS (port 5355)",
            "🔐 Security: Per-device certificates",
        ],
    },
    // Slide 9: Setup Manager
    Slide {
        id: 8,
        title: "Setup Manager",
        icon: "📦",
        content: SlideContent::SetupManager,
        narration: "My Setup Manager handles first-time initialization automatically. \
                   On first launch, it checks for required components. \
                   The Downloader fetches Whisper model at 81 megabytes, Piper TTS, SBERT, and FFmpeg. \
                   Downloads are platform-specific: Windows gets zips, Linux gets tar archives. \
                   The Extractor unpacks everything to correct locations. \
                   After extraction, downloads folder is deleted to save space. \
                   Total size is about 200 megabytes.",
        bullet_points: &[
            "🚀 First Launch: Check pkg/ folder",
            "📥 Download: Whisper, Piper, SBERT, FFmpeg",
            "📂 Extract: Platform-specific archives",
            "🗑️ Cleanup: Delete downloads after extraction",
        ],
    },
    // Slide 10: Tech Stack
    Slide {
        id: 9,
        title: "Technology Stack",
        icon: "🛠️",
        content: SlideContent::TechStack,
        narration: "My technology stack is optimized for performance. \
                   Rust for memory safety and speed. \
                   Dioxus 0.7 for cross-platform UI. \
                   Whisper dot cpp for speech recognition. \
                   Piper for text-to-speech. \
                   SBERT MiniLM for semantic understanding. \
                   FFmpeg for media capture. \
                   Rustls for TLS encryption. \
                   CPAL for audio capture. \
                   Tokio for async operations. \
                   Single binary of about 15 megabytes, models add 200 megabytes.",
        bullet_points: &[
            "🦀 Rust (Performance)",
            "⚛️ Dioxus 0.7 (UI)",
            "🎙️ Whisper.cpp (STT)",
            "🔊 Piper (TTS)",
            "🧠 SBERT (NLU)",
            "🎬 FFmpeg (Media)",
        ],
    },
    // Slide 11: Voice Commands
    Slide {
        id: 10,
        title: "Voice Commands",
        icon: "🗣️",
        content: SlideContent::VoiceCommands,
        narration: "Here are some example voice commands you can use. \
                   Say 'Arise' to activate me. \
                   'Open Chrome' launches the browser. \
                   'Take photo' captures from camera. \
                   'Start recording' records video with audio. \
                   'Search Rust tutorials' opens web search. \
                   'Open downloads' opens the folder. \
                   You can also use Control Shift Space hotkey to activate me anytime.",
        bullet_points: &[
            "\"Arise\" → Activate IGRIS",
            "\"Open Chrome\" → Launch browser",
            "\"Take photo\" → Camera capture",
            "\"Start recording\" → Video + Audio",
            "\"Search tutorials\" → Web search",
            "\"Open downloads\" → File explorer",
        ],
    },
    // Slide 12: Supported Apps
    Slide {
        id: 11,
        title: "50+ Applications",
        icon: "📱",
        content: SlideContent::SupportedApps,
        narration: "I support over 50 applications across categories. \
                   Browsers: Chrome, Firefox, Edge, Brave, Opera, Arc. \
                   Office: Word, Excel, PowerPoint, Outlook, OneNote. \
                   Communication: Discord, Telegram, WhatsApp, Zoom, Teams, Slack. \
                   Creative: Photoshop, Premiere, Figma, Blender, Canva. \
                   Gaming: Steam, Epic Games, GOG, Battle dot net. \
                   Editors: VS Code, Sublime, Notepad plus plus, IntelliJ.",
        bullet_points: &[
            "🌐 Browsers: Chrome, Firefox, Edge, Brave",
            "📄 Office: Word, Excel, PowerPoint",
            "💬 Comms: Discord, Zoom, Teams, Slack",
            "🎨 Creative: Photoshop, Figma, Blender",
            "🎮 Gaming: Steam, Epic, GOG",
            "💻 Editors: VS Code, Sublime, Notepad++",
        ],
    },
    // Slide 13: Module Summary
    Slide {
        id: 12,
        title: "Module Summary",
        icon: "📊",
        content: SlideContent::ModuleSummary,
        narration: "Here's a summary of my modules. \
                   Audio module handles VAD, STT, and TTS with 5 files. \
                   NLU module has SBERT, NER, and Context with 4 files. \
                   Plugins module manages 50 plus apps with 12 files. \
                   File Share handles discovery and transfer with 8 files. \
                   Setup module manages downloads with 7 files. \
                   UI module has Dioxus components with 10 files. \
                   Utils has hotkey and config with 6 files. \
                   Media module has FFmpeg camera with 2 files. \
                   Total: about 15,000 lines of Rust across 50 plus source files.",
        bullet_points: &[
            "🎤 Audio: VAD, STT, TTS (5 files)",
            "🧠 NLU: SBERT, NER, Context (4 files)",
            "🔌 Plugins: 50+ Apps (12 files)",
            "📁 File Share: Discovery, Transfer (8 files)",
            "📦 Setup: Download, Extract (7 files)",
            "🖥️ UI: Dioxus Components (10 files)",
        ],
    },
    // Slide 14: Future Scope
    Slide {
        id: 13,
        title: "Future Scope",
        icon: "🚀",
        content: SlideContent::FutureScope,
        narration: "Here's what's planned for the future. \
                   Local LLM integration with Llama and Mistral for conversations. \
                   Mobile companion app to control PC from phone. \
                   Smart home control for IoT devices. \
                   Plugin marketplace for community plugins. \
                   Multi-language support including Hindi and Spanish. \
                   Custom themes for UI personalization.",
        bullet_points: &[
            "🤖 Local LLM: Llama, Mistral",
            "📱 Mobile Companion App",
            "🏠 Smart Home Control",
            "🔌 Plugin Marketplace",
            "🌐 Multi-language Support",
            "🎨 Custom Themes",
        ],
    },
    // Slide 15: Thank You
    Slide {
        id: 14,
        title: "Thank You!",
        icon: "🗡️",
        content: SlideContent::ThankYou,
        narration: "Thank you for learning about me! \
                   I am IGRIS, your AI Shadow Monarch. \
                   Remember, you can activate me anytime by saying 'Arise' or pressing Control Shift Space. \
                   I was developed by Sohan Patil with love, using Rust and Dioxus. \
                   If you have any questions, just ask!",
        bullet_points: &[
            "🗡️ IGRIS - Your AI Shadow Monarch",
            "🎤 Say \"Arise\" to activate",
            "⌨️ Or press Ctrl+Shift+Space",
            "👨‍💻 Developed by: Sohan Patil",
            "❤️ Built with Rust + Dioxus",
        ],
    },
];
