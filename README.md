# IGRIS v4 — Hybrid Offline/Online AI Voice Assistant

A voice-activated AI assistant built with **Rust** and **Dioxus 0.7**. IGRIS provides hands-free desktop control with natural language understanding, camera management, file sharing, alarms, reminders, and an extensible plugin system. It runs **fully offline** using local models by default, or switches to **online mode** using NVIDIA NIM cloud APIs (Parakeet STT + LLM reasoning with tool calling) for smarter, more capable conversations.

![IGRIS](icons/igris_icon.svg)

---

## Features

### Voice Processing Pipeline
- **Wake Word Detection** — Say "hello" (or "alita" in Alita mode) to activate
- **Speech Recognition** — Offline STT via Whisper (base-q8_0 quantized model); or **Online Parakeet ASR** via NVIDIA NIM gRPC
- **Voice Activity Detection** — FFT-based spectral VAD with configurable thresholds, noise gating, and state machine
- **Natural Language Understanding** — SBERT sentence embeddings + NER + fuzzy keyword matching with context memory
- **LLM Reasoning** — **Offline**: candle ML (Qwen 2.5 1.5B GGUF, feature-gated); **Online**: any NVIDIA NIM model (default `meta/llama-3.1-8b-instruct`) with 18-tool function calling, personality, and conversation context
- **Text-to-Speech** — Piper TTS with personality-based voice selection and audio caching

### Smart Auto-Mode Selection
- **Internet Check on Startup** — First thing after launch: pings `1.1.1.1`, `8.8.8.8`, and `google.com` to detect connectivity
- **Auto-Enables Online Mode** — If internet + `NVIDIA_API_KEY` present, online mode activates immediately; skips loading heavy offline models (SBERT NLU ~80MB, local LLM ~1GB)
- **Auto-Fallback to Offline** — If online LLM times out (15s) or fails 3 consecutive times, switches to offline mode automatically
- **Connectivity Monitor** — Background task checks internet every 15 seconds; switches modes seamlessly as connectivity changes
- **On-Demand NLU Loading** — SBERT NLU engine loads only when switching from online to offline mode at runtime

### Online Mode (NVIDIA NIM APIs)
- **Parakeet ASR** — High-accuracy speech recognition via gRPC (`grpc.nvcf.nvidia.com:443`)
- **NIM Chat Completions** — LLM reasoning with tool calling (OpenAI-compatible API at `integrate.api.nvidia.com/v1`)
- **Personality-Driven System Prompt** — Dynamic personality descriptions (IGRIS: calm/professional, Alita: energetic/friendly) tailored per query
- **18 Tool Functions** — The LLM can open/close apps, search web, control system, manage clipboard, take screenshots, get weather, tell jokes/facts, set alarms/reminders, switch modes, and more
- **Conversation Context** — Last 3 user–assistant turns passed to the LLM for follow-up understanding
- **Configurable Model** — Any NIM model via `NVIDIA_NIM_MODEL` env var (GLM, Llama, Nemotron, etc.)
- **Automatic Fallback** — If online STT or reasoning fails, falls back gracefully to local models

### System Control
- **App Launcher** — Open/close apps by voice (50+ app aliases: Chrome, Firefox, VSCode, Spotify, Discord, etc.)
- **System Commands** — Volume, brightness, WiFi, Bluetooth, lock screen, sleep, shutdown, restart
- **File Operations** — Create, delete, search files with multi-threaded recursive search
- **Camera Control** — FFmpeg-based photo capture & video recording with live preview UI
- **Web Search** — Voice-triggered web search with spoken results (Google, Bing, DuckDuckGo, Yahoo)
- **Alarms & Reminders** — Set time-based alarms and reminders with background scheduler
- **Screenshot** — Take screenshots via platform tools (`screencapture`, `import`, PowerShell)
- **System Info** — Query OS version, memory, CPU, public IP, and uptime
- **Clipboard** — Read and write clipboard contents

### FastSwap File Transfer
- **TLS-Encrypted** — End-to-end encryption via self-signed TLS (port 53318)
- **Cross-Platform Sharing** — LocalSend v2.0 protocol compatible
- **Network Discovery** — Automatic subnet scanning (1–254) for nearby devices (HTTPS-only)
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
- **IGRIS** (default) — Calm, collected, professional; deep male voice; purple/cyan UI theme; wake word "hello"
- **Alita** — Energetic, friendly, enthusiastic; female voice; lavender/pink UI theme; wake word "alita"
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
├── main.rs                   # Dioxus 0.7 UI + voice loop + LLM tool routing (1946 lines)
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
├── online/
│   ├── mod.rs                # Online mode state toggle + init_from_env()
│   ├── reasoning.rs          # NVIDIA NIM chat completions + tool calling + personality prompt
│   └── stt.rs                # Parakeet ASR via tonic gRPC (rustls CryptoProvider)
├── nlu/
│   ├── engine.rs             # Intent engine (SBERT + keyword + Jaccard)
│   ├── sbert.rs              # SBERT sentence embeddings (all-MiniLM-L6-v2)
│   ├── ner.rs                # Named Entity Recognition
│   ├── context.rs            # Conversation context & reference resolution
│   └── nlu_processor.rs      # End-to-end NLU processor
├── commands/
│   ├── system.rs             # Volume, brightness, WiFi, Bluetooth, power, screenshot, system info, clipboard
│   ├── files.rs              # Create, delete, search files
│   ├── web.rs                # Web search, weather (wttr.in), jokes (JokeAPI), facts (Useless Facts)
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
│   ├── tls.rs                # Self-signed TLS cert generation (rcgen + ring)
│   └── network/
│       ├── discovery.rs      # HTTPS subnet scanning (LocalSend protocol)
│       ├── server.rs         # Axum HTTP server + TLS proxy (receive files, handle approval)
│       └── client.rs         # HTTPS client (send files, poll server)
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
Startup: ─→ Check Internet ─→ Online? + API key? ─YES─→ Online Mode (skip SBERT/local LLM)
                              │                       └──→ Parakeet ASR → NIM LLM → Tool Router → TTS
                              └NO─→ Offline Mode
                                    └──→ Whisper STT → NLU (SBERT) → Plugin System → TTS (Piper)
                                                           ↓
                                                     Keyword/LocalLLM

Runtime: ─→ Connectivity Monitor (every 15s) ─→ Internet lost? → Auto-switch to offline
                                                → Internet back? → Auto-switch to online

                                  ┌─────────────────────────────┐
                                  │      OFFLINE MODE           │
Audio → VAD (FFT spectral) ─────→┤  Whisper STT → NLU (SBERT) ├─→ Plugin System → TTS (Piper)
                                  │              ↓              │
                                  │        Keyword/LocalLLM     │
                                  └─────────────────────────────┘

                                  ┌───────────────────────────────────────────┐
                                  │         ONLINE MODE (NVIDIA NIM)         │
Audio → VAD (FFT spectral) ─────→┤  Parakeet ASR (gRPC) → NIM LLM (REST)   ├─→ Tool Router → TTS (Piper)
                                  │              ↓                           │
                                  │    18 tools: open/close apps, web,       │
                                  │    weather, screenshot, system info,     │
                                  │    clipboard, alarms, jokes, facts, etc. │
                                  └───────────────────────────────────────────┘
```

---

## Quick Start

### Prerequisites
- Rust 1.70+
- macOS 10.13+, Windows 10+, or Linux (x86_64)
- 4 GB RAM (8 GB recommended)
- Microphone access
- Network access for FastSwap (port 53318 for TLS, port 53317 for HTTP fallback)
- *(Optional)* NVIDIA API key (nvapi-...) for online mode

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

### Online Mode Setup

```bash
# Create .env file in project root
echo "NVIDIA_API_KEY=nvapi-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" > .env

# Optional: set model (default: meta/llama-3.1-8b-instruct)
echo "NVIDIA_NIM_MODEL=meta/llama-3.1-8b-instruct" >> .env

# Optional: auto-enable online mode on launch
export IGRIS_ONLINE_MODE=true
```

### First Use
1. Wait for setup to complete (permissions + model downloads)
2. On launch, IGRIS checks internet connectivity:
   - **Internet + API key** → auto-enables online mode (skips offline model loading)
   - **No internet / no key** → loads offline models and runs fully local
3. Say **"hello"** to wake IGRIS
4. Give a command: *"Open Chrome"*, *"What's the weather in London"*, *"Tell me a joke"*
5. If internet drops during use, auto-switches to offline mode within ~15 seconds

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

### Web & Information
```
"Search for Rust programming"   "What is the weather today"
"Look up latest news"           "What's the weather in London"
"Tell me a joke"                "Tell me an interesting fact"
```

### Desktop Tools
```
"Take a screenshot"             "What's my IP address"
"Show system information"       "How much RAM do I have"
"Read clipboard"                "Copy 'hello world' to clipboard"
```

### Mode Switching
```
"Switch to online mode"         "Go online"
"Switch to offline mode"        "Go offline"
```

*(Mode switching is automatic — IGRIS detects internet connectivity and switches on its own. These commands are for manual override.)*

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

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NVIDIA_API_KEY` | For online mode | — | NVIDIA API key (nvapi-... format) for Parakeet STT (gRPC) and NIM chat (REST) |
| `NVIDIA_NIM_MODEL` | No | `meta/llama-3.1-8b-instruct` | Model for chat completions (any NIM model) |
| `NVIDIA_NIM_BASE_URL` | No | `https://integrate.api.nvidia.com/v1` | Base URL for chat completions API |
| `NVIDIA_NIM_GLM_BASE_URL` | No (fallback) | — | Fallback base URL for GLM models |
| `IGRIS_ONLINE_MODE` | No | `false` | Set to `true` or `1` to auto-enable online mode on launch |

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
| `model.int8.onnx` + `tokens.txt` | ~60 MB | SenseVoice STT (sherpa-onnx) |
| `en_US-libritts_r-medium.onnx` | 50 MB | Piper TTS voice |
| `all-MiniLM-L6-v2` | ~80 MB | SBERT sentence embeddings |
| `espeak-ng-data` | — | Phoneme data for Piper |

Optional (feature-gated with `candle` feature):
| `qwen2.5-1.5b-instruct-q4_k_m.gguf` | ~1 GB | Local LLM for smart reasoning fallback |

Online mode models (cloud, no download):
| NVIDIA Parakeet ASR | — | Cloud STT via gRPC |
| `meta/llama-3.1-8b-instruct` (default) | — | LLM reasoning with tool calling |
| Any NIM model | — | Configurable via `NVIDIA_NIM_MODEL` |

---

## Development

```bash
# Debug build
cargo build

# Release build (recommended for daily use)
cargo build --release

# Run with local LLM support
cargo run --release --features candle

# Run with online mode
IGRIS_ONLINE_MODE=true cargo run --release

# Run tests
cargo test

# Run with logging
RUST_LOG=igrisv3=debug cargo run
```

### Project Structure Notes
- `main.rs` (~1946 lines) contains the Dioxus UI component tree, voice loop orchestrator, and LLM tool routing
- The voice loop runs on a dedicated Tokio runtime thread
- Plugin system uses 5-pass command matching (exact → example → contains → fuzzy → word-overlap)
- VAD uses FFT-based spectral analysis with configurable noise floor estimation
- Online reasoning system prompt includes personality descriptions, 18 tools, and last 3 conversation turns for context
- Online mode uses `tonic` gRPC for Parakeet ASR and `reqwest` REST for NIM chat completions
- Two `CryptoProvider` backends used: `aws-lc-rs` (tonic gRPC/TLS) and `ring` (rustls 0.23 for FastSwap)

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Mic not working | Check system permissions, try different input device |
| STT slow | Use release build (`cargo run --release`) or try online mode |
| Models missing | Delete `pkg/` folder and restart for fresh download |
| Camera error | Ensure no other app is using the camera |
| Volume/Brightness not working | macOS: System Settings → Privacy → Automation |
| Alarm not triggering | Background scheduler checks every 10 seconds |
| FastSwap not finding devices | Ensure devices are on the same network subnet, allow port 53318 (TLS) in firewall |
| espeak-ng-data not found | Install espeak-ng: `brew install espeak-ng` (macOS) |
| Online mode not working | Check `NVIDIA_API_KEY` is set in `.env`; verify key is valid for Parakeet + NIM |
| Online mode not auto-enabling | IGRIS checks internet at startup — if no connectivity or missing API key, stays offline. Check `.env` and internet connection |
| Auto-switch to offline too aggressive | 3 consecutive online failures (timeout or error) triggers offline switch. Connectivity monitor re-enables online when internet returns |
| Parakeet STT fails | Check gRPC connectivity to `grpc.nvcf.nvidia.com:443`; some keys may lack ASR access |
| LLM returns 503 | The NIM model may be overloaded; try a different model or switch to offline mode |
| `rustls` CryptoProvider panic | Already fixed — `aws_lc_rs` provider initialized via `Once` guard in `src/online/stt.rs` |

---

## License

MIT

---

## Roadmap

- [x] Wake word detection with Levenshtein fallback
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
- [x] Online mode with NVIDIA NIM (Parakeet STT + LLM reasoning)
- [x] Personality-driven system prompts (IGRIS calm / Alita energetic)
- [x] 18 LLM tool functions (weather, jokes, facts, screenshot, system info, clipboard, etc.)
- [x] Conversation context for follow-up understanding
- [x] Configurable NIM model via environment variable
- [x] Automatic fallback from online to local models
- [x] Auto-detect internet on startup, skip offline model loading when online
- [x] Background connectivity monitor (auto-switch online/offline every 15s)
- [x] Online reasoning timeout (15s) with fallback to offline
- [x] On-demand NLU loading when switching from online to offline
- [x] Automatic offline switch after 3 consecutive online failures
- [ ] Voice-activated file sharing
- [ ] Transfer history persistence
- [ ] Multi-language support
- [ ] Custom wake word training
- [ ] Voice command history & analytics
- [ ] Enhanced camera features (filters, zoom)

---

*Say **"hello"** to begin.*
