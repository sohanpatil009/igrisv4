# IGRIS v4 — Offline AI Voice Assistant

A fully offline, voice-activated AI assistant built with **Rust** and **Dioxus 0.7**. IGRIS provides hands-free desktop control with natural language understanding, camera management, file sharing, alarms, reminders, and an extensible plugin system — all running locally with no cloud dependency.

![IGRIS](icons/igris_icon.svg)

---

## Features

### Voice Processing Pipeline
- **Wake Word Detection** — Say "hello" (or "alita" in Alita mode) to activate
- **Speech Recognition** — Offline STT via Whisper (base-q8_0 quantized model)
- **Voice Activity Detection** — FFT-based spectral VAD with configurable thresholds, noise gating, and state machine
- **Natural Language Understanding** — SBERT sentence embeddings + NER + fuzzy keyword matching with context memory
- **Text-to-Speech** — Piper TTS with personality-based voice selection and audio caching

### System Control
- **App Launcher** — Open/close apps by voice (50+ app aliases: Chrome, Firefox, VSCode, Spotify, Discord, etc.)
- **System Commands** — Volume, brightness, WiFi, Bluetooth, lock screen, sleep, shutdown, restart
- **File Operations** — Create, delete, search files with multi-threaded recursive search
- **Camera Control** — FFmpeg-based photo capture & video recording with live preview UI
- **Web Search** — Voice-triggered web search with spoken results
- **Alarms & Reminders** — Set time-based alarms and reminders with background scheduler

### FastSwap File Transfer
- **Cross-Platform Sharing** — LocalSend v2.0 protocol compatible
- **Network Discovery** — Automatic subnet scanning (1–254) for nearby devices
- **Approval Flow** — Full-screen popup for receiver to accept/deny transfers
- **Real-Time Progress** — Per-file progress bars with speed and ETA
- **Multi-File/Folder** — Send entire directories with recursive scanning
- **Transfer Control** — Cancel in-progress transfers; 60-second approval timeout

### Plugin System
- **13 Built-in Plugins** — Browsers, utilities, media, office, communication, creative, gaming, editors, camera, files, reminders, system control, presentation
- **Smart App Aliases** — "open chrome" and "launch google" both resolve to Chrome
- **Extensible** — Custom JSON plugins supported, loaded from `plugins/` directory
- **Unified Routing** — All commands pass through the plugin system before falling back to NLU

### Personalities
- **IGRIS** (default) — Deep male voice, purple/cyan UI theme, wake word "hello"
- **Alita** — Female voice, lavender/pink UI theme, wake word "alita"
- **Custom** — Configurable name, speaker, and wake word

### UI
- **Dark Theme** — Gradient background (#0a0a0a → #1a1a2e) with glassmorphism panels
- **Dynamic Colors** — Personality-based accent colors (cyan standby → purple/pink awake)
- **Animated Orb** — Glowing core with personality-based colors and pulse animations
- **Responsive** — Scales smoothly with window size using `clamp()`
- **Settings Panel** — Configure personality, sensitivity, volume, speed, theme, logs visibility
- **System Logs** — Real-time log panel with color-coded entries (info/success/warning/error)
- **Active Apps Panel** — Shows applications launched by IGRIS with live process tracking

---

## Architecture

```
src/
├── main.rs                   # Dioxus 0.7 UI + voice loop (1811 lines)
├── lib.rs                    # Library exports & global state
├── config.rs                 # JSON config (personality, recognition, TTS, hotkey, UI)
├── core/
│   ├── stt.rs                # Whisper STT (offline transcription)
│   ├── tts.rs                # Piper TTS with audio caching
│   ├── vad.rs                # FFT-based Voice Activity Detection
│   ├── wake_word.rs          # Wake word detection with Levenshtein fallback
│   ├── audio_capture.rs      # CPAL audio capture with VAD integration
│   ├── about.rs              # Self-presentation data
│   └── local_llm.rs          # Candle ML local LLM inference (feature-gated)
├── nlu/
│   ├── engine.rs             # Intent engine (SBERT + keyword + Jaccard)
│   ├── sbert.rs              # SBERT sentence embeddings (all-MiniLM-L6-v2)
│   ├── ner.rs                # Named Entity Recognition
│   ├── context.rs            # Conversation context & reference resolution
│   └── nlu_processor.rs      # End-to-end NLU processor
├── commands/
│   ├── system.rs             # Volume, brightness, WiFi, Bluetooth, power
│   ├── files.rs              # Create, delete, search files
│   ├── web.rs                # Web search & browser integration
│   ├── ffmpeg_camera.rs      # FFmpeg camera control
│   ├── about.rs              # Self-presentation command
│   ├── reminders.rs          # Alarms & reminders scheduler
│   └── app_utils.rs          # Running app utilities
├── plugins/
│   ├── system.rs             # Plugin manager (load, match, execute)
│   └── builtin/              # 13 builtin Rust plugins
│       ├── browsers.rs       # Chrome, Firefox, Edge, Brave, Opera, Safari
│       ├── utilities.rs      # Calculator, Notepad, Explorer, Terminal, Settings
│       ├── media.rs          # Spotify, VLC, YouTube
│       ├── office.rs         # Word, Excel, PowerPoint, Outlook
│       ├── communication.rs  # Discord, Slack, Teams, Zoom, Skype, Telegram, WhatsApp
│       ├── creative.rs       # Photoshop, Premiere, Blender, etc.
│       ├── gaming.rs         # Steam, Epic, etc.
│       ├── editors.rs        # VSCode, Sublime, Atom, Vim, Notepad++
│       ├── camera.rs         # Camera open/close/photo/recording
│       ├── files.rs          # File create/delete/open
│       ├── reminders.rs      # Alarm/reminder set/show/cancel
│       └── system_control.rs # Volume, brightness, wifi, bt, power
├── ui/
│   ├── mod.rs                # UI module exports
│   ├── settings.rs           # Settings modal (personality, sensitivity, volume, speed, theme)
│   ├── camera_panel.rs       # Camera preview + photo/video controls
│   ├── fastswap_panel.rs     # File sharing panel with device discovery
│   ├── incoming_transfer_popup.rs # Full-screen approval dialog
│   ├── file_picker.rs        # File/folder selector (native dialog)
│   ├── search_results.rs     # File search results panel
│   ├── menu_button.rs        # Top-right hamburger menu
│   └── presentation/         # Self-presentation slides
│       ├── mod.rs            # Presentation state management
│       ├── panel.rs          # Full-screen slide viewer
│       └── slides.rs         # Slide content & TTS narration
├── fastswap/
│   ├── mod.rs                # FastSwap manager, approval, cancellation, progress
│   ├── models/
│   │   ├── device.rs         # Device, RegisterRequest, Announce
│   │   ├── transfer.rs       # FileInfo, PrepareUpload, ConfirmUpload
│   │   └── progress.rs       # FileProgress, TransferProgress with speed/ETA
│   └── network/
│       ├── discovery.rs      # UDP-based subnet scanning (LocalSend protocol)
│       ├── server.rs         # Axum HTTP server (receive files, handle approval)
│       └── client.rs         # HTTP client (send files, poll server)
├── platform/
│   ├── app_launcher.rs       # OS-specific app launch/close (macOS `open -a`, Windows `start`, Linux)
│   ├── file_system.rs        # File operations abstraction
│   ├── process_builder.rs    # Cross-platform command builder
│   └── system_control.rs     # Volume, brightness, WiFi, Bluetooth, power
├── setup_manager/
│   ├── mod.rs                # Setup orchestrator, uninstall, verification
│   ├── downloader.rs         # Concurrent model downloads with progress
│   ├── extractor.rs          # Zip extraction
│   ├── validator.rs          # File integrity checks (SHA256, size)
│   ├── permissions.rs        # Module-level permission system
│   ├── permissions_ui.rs     # Permission grant UI
│   ├── gui.rs                # Setup progress GUI
│   └── platforms/            # Platform-specific setup
│       ├── macos.rs          # macOS: Homebrew deps, model downloads
│       ├── windows.rs        # Windows: FFmpeg, Scoop/Choco
│       └── linux.rs          # Linux: apt/pacman deps, model downloads
├── utils/
│   ├── hotkey.rs             # Global hotkey (Ctrl+Shift+Space)
│   ├── greetings.rs          # Voice greetings & wake word detection
│   ├── shared_memory.rs      # Thread pool for faster processing
│   └── process_tracker.rs    # Track opened/closed processes
└── media/
    └── ffmpeg_camera/        # FFmpeg-based camera module
```

### Data Flow

```
Audio → VAD (FFT spectral) → Whisper STT → NLU (SBERT + NER) → Plugin System → Command Handler → TTS (Piper)
                                                                        ↓
                                                                  Fallback: Keyword/NLU/LocalLLM
```

---

## Quick Start

### Prerequisites
- Rust 1.70+
- macOS 10.13+, Windows 10+, or Linux (x86_64)
- 4 GB RAM (8 GB recommended)
- Microphone access
- Network access for FastSwap (port 53317)

### Build & Run

```bash
git clone <repo-url> && cd igrisv4
cargo run --release
```

First launch automatically downloads:
- Whisper STT model (~81 MB)
- Piper TTS + voice model (~50 MB)
- SBERT NLU model (~80 MB)
- FFmpeg (Windows only, ~100 MB)

### First Use
1. Wait for setup to complete (permissions + model downloads)
2. Say **"hello"** to wake IGRIS
3. Give a command: *"Open Chrome"*, *"What time is it"*, *"Tell me about yourself"*

---

## Voice Commands

### Application Control
```
"Open Chrome"           "Close Firefox"
"Launch Discord"        "Quit Spotify"
"Open Visual Studio"    "Close all applications"
```

### System Control
```
"Increase volume by 20"    "Set brightness to 80"
"Mute"                      "Unmute"
"Lock screen"               "Shutdown"
"Restart"                   "Sleep"
"Enable WiFi"               "Disable Bluetooth"
```

### Alarms & Reminders
```
"Set alarm for 7 am"               "Wake me up at 6:30 pm"
"Remind me in 30 minutes"          "Remind me to call mom in 2 hours"
"Show alarms"                      "Cancel all reminders"
```

### Files
```
"Search for *.pdf files"   "Create file notes.txt"
"Open downloads folder"    "Delete file temp.txt"
```

### Camera
```
"Take a photo"      "Start recording"
"Stop recording"    "Open camera"
"Close camera"
```

### Web
```
"Search for Rust programming"   "What is the weather today"
"Look up latest news"           "Search the web for..."
```

### FastSwap File Sharing
```
"Open FastSwap"   "Share files"   "Fast swap"
```
- Scan for nearby devices on the local network
- Select files/folders via native file dialog
- Receiver gets a full-screen approval popup
- Real-time progress tracking for both sender and receiver

### Assistant Control
```
"Tell me about yourself"  → Starts self-presentation mode
"Sleep"                   → Return to wake word mode
"Exit"                    → Shutdown IGRIS
```

---

## Configuration

Settings saved to `pkg/config.json`:

```json
{
  "personality": "Igris",
  "recognition": {
    "sensitivity": 0.45,
    "max_listen_sec": 15,
    "silence_timeout_ms": 600
  },
  "tts": {
    "speed": 1.0,
    "volume": 0.8,
    "use_cache": true
  },
  "hotkey": {
    "modifier": "Ctrl+Shift",
    "key": "Space",
    "enabled": true
  },
  "ui": {
    "theme": "Dark",
    "show_logs": true,
    "show_apps": true,
    "window_width": 800,
    "window_height": 600
  }
}
```

Accessible via the **Settings Panel** (☰ menu button, top-right).

---

## Models

| Model | Size | Purpose |
|-------|------|---------|
| `ggml-base.bin` / `ggml-base-q8_0.bin` | 81 MB | Whisper STT |
| `en_US-libritts_r-medium.onnx` | 50 MB | Piper TTS voice |
| `all-MiniLM-L6-v2` | ~80 MB | SBERT sentence embeddings |
| `espeak-ng-data` | — | Phoneme data for Piper |

Optional (feature-gated with `candle` feature):
| `qwen2.5-1.5b-instruct-q4_k_m.gguf` | ~1 GB | Local LLM for smart reasoning fallback |

---

## Development

```bash
# Debug build
cargo build

# Release build (recommended for daily use)
cargo build --release

# Run with local LLM support
cargo run --release --features candle

# Run tests
cargo test

# Run with logging
RUST_LOG=igrisv3=debug cargo run
```

### Project Structure Notes
- `main.rs` (~1811 lines) contains the Dioxus UI component tree and voice loop orchestrator
- The voice loop runs on a dedicated Tokio runtime thread
- Plugin system uses 5-pass command matching (exact → example → contains → fuzzy → word-overlap)
- VAD uses FFT-based spectral analysis with configurable noise floor estimation

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Mic not working | Check system permissions, try different input device |
| STT slow | Use release build (`cargo run --release`) |
| Models missing | Delete `pkg/` folder and restart for fresh download |
| Camera error | Ensure no other app is using the camera |
| Volume/Brightness not working | macOS: System Settings → Privacy → Automation |
| Alarm not triggering | Background scheduler checks every 10 seconds |
| FastSwap not finding devices | Ensure devices are on the same network subnet, allow port 53317 in firewall |
| espeak-ng-data not found | Install espeak-ng: `brew install espeak-ng` (macOS) |

---

## License

MIT

---

## Roadmap

- [x] SBERT semantic NLU with context memory
- [x] Self-presentation mode with animated slides
- [x] FFmpeg-based camera (photo + video with audio)
- [x] Multi-threaded file search
- [x] Alarms & Reminders with background scheduler
- [x] Fully plugin-based architecture (13 built-in plugins)
- [x] Dynamic camera/mic detection
- [x] Smart command validation & fallback
- [x] FastSwap file transfer (LocalSend v2.0 compatible)
- [x] Incoming transfer approval popup
- [x] Receiver-side real-time progress tracking
- [x] Folder selection with recursive scanning
- [x] Server-side polling for transfer approval (60s timeout)
- [ ] Voice-activated file sharing
- [ ] Transfer history persistence
- [ ] Multi-language support
- [ ] Custom wake word training
- [ ] Voice command history & analytics
- [ ] Enhanced camera features (filters, zoom)

---

*Say **"hello"** to begin.*
