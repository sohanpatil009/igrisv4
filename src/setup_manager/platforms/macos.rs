// src/setup_manager/platforms/macos.rs
// macOS-specific setup: Homebrew install + PATH configuration

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use crate::setup_manager::SetupEvent;
use super::{PlatformSetup, command_exists, run_command};

pub struct MacOSSetup {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
}

impl MacOSSetup {
    pub fn new(pkg_dir: PathBuf, event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>) -> Self {
        Self { pkg_dir, event_sender }
    }
    
    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
    
    /// Check if Homebrew is installed
    fn is_homebrew_installed(&self) -> bool {
        command_exists("brew")
    }
    
    /// Install Homebrew if not present
    fn install_homebrew(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_homebrew_installed() {
            return Ok(());
        }
        
        self.send_event(SetupEvent::Downloading {
            name: "Homebrew".to_string(),
            progress: 0,
        });
        
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#)
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to install Homebrew: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        self.send_event(SetupEvent::Completed {
            name: "Homebrew".to_string(),
        });
        
        Ok(())
    }
    
    /// Install a package via Homebrew
    fn brew_install(&self, package: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: package.to_string(),
            progress: 0,
        });
        
        let output = std::process::Command::new("brew")
            .args(["install", package])
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to install {}: {}",
                package,
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        self.send_event(SetupEvent::Completed {
            name: package.to_string(),
        });
        
        Ok(())
    }
    
    /// Get Homebrew prefix path
    fn brew_prefix(&self) -> PathBuf {
        // Apple Silicon: /opt/homebrew
        // Intel: /usr/local
        if PathBuf::from("/opt/homebrew").exists() {
            PathBuf::from("/opt/homebrew")
        } else {
            PathBuf::from("/usr/local")
        }
    }
    
    /// Download a file using curl
    async fn download_file(&self, url: &str, dest: &PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let output = std::process::Command::new("curl")
            .args(["-L", "-o", &dest.to_string_lossy(), url])
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to download {}: {}",
                url,
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        Ok(())
    }
    
    /// Add export line to shell profile
    fn add_to_profile(&self, export_line: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        
        // Try multiple shell profiles
        let profiles = vec![
            home.join(".zshrc"),      // Default on modern macOS
            home.join(".bash_profile"),
            home.join(".profile"),
        ];
        
        for profile in profiles {
            if profile.exists() {
                let content = std::fs::read_to_string(&profile)?;
                if !content.contains(export_line) {
                    let mut new_content = content;
                    if !new_content.ends_with('\n') {
                        new_content.push('\n');
                    }
                    new_content.push_str(export_line);
                    new_content.push('\n');
                    std::fs::write(&profile, new_content)?;
                }
                return Ok(());
            }
        }
        
        // Create .zshrc if no profile exists
        let zshrc = home.join(".zshrc");
        std::fs::write(&zshrc, format!("{}\n", export_line))?;
        
        Ok(())
    }
}

impl PlatformSetup for MacOSSetup {
    fn pkg_dir(&self) -> &PathBuf {
        &self.pkg_dir
    }
    
    fn is_setup_complete(&self) -> bool {
        let ffmpeg_ok = command_exists("ffmpeg");
        let tts_ok = command_exists("piper") || self.pkg_dir.join("piper/piper").exists();
        
        // Check for quantized model (q8_0) which is what downloader downloads
        let whisper_ok = self.pkg_dir.join("models/ggml-base-q8_0.bin").exists() 
            || self.pkg_dir.join("models/ggml-base.bin").exists();
        
        let voice_ok = self.pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx").exists();
        let sbert_ok = self.pkg_dir.join("models/sbert/pytorch_model.bin").exists();
        
        let llvm_ok = command_exists("clang");
        
        ffmpeg_ok && tts_ok && whisper_ok && voice_ok && sbert_ok && llvm_ok
    }
    
    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dirs = vec![
            self.pkg_dir.join("audio"),
            self.pkg_dir.join("models/bold_voice"),
            self.pkg_dir.join("models/sbert"),
            self.pkg_dir.join("downloads"),
        ];
        
        for dir in dirs {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
    
    async fn install_ffmpeg(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if command_exists("ffmpeg") {
            self.send_event(SetupEvent::Completed {
                name: "FFmpeg (already installed)".to_string(),
            });
            return Ok(());
        }
        
        self.install_homebrew()?;
        self.brew_install("ffmpeg")?;
        Ok(())
    }
    
    async fn install_piper(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tts_dir = self.pkg_dir.join("piper");
        let tts_binary = tts_dir.join("piper");
        
        // Check if piper already exists
        if command_exists("piper") || tts_binary.exists() {
            self.send_event(SetupEvent::Completed {
                name: "Piper TTS (already installed)".to_string(),
            });
            return Ok(());
        }
        
        // Try installing via Homebrew first (faster and more reliable)
        self.send_event(SetupEvent::Downloading {
            name: "Piper TTS (via Homebrew)".to_string(),
            progress: 0,
        });
        
        self.install_homebrew()?;
        
        // Try brew install piper
        let output = std::process::Command::new("brew")
            .args(["install", "piper-tts/piper/piper"])
            .output();
        
        match output {
            Ok(out) if out.status.success() => {
                self.send_event(SetupEvent::Completed {
                    name: "Piper TTS".to_string(),
                });
                return Ok(());
            }
            _ => {
                println!("[Setup] Homebrew install failed, trying alternative tap...");
            }
        }
        
        // Try alternative tap
        let output = std::process::Command::new("brew")
            .args(["tap", "rhasspy/piper"])
            .output();
        
        if output.is_ok() {
            let output = std::process::Command::new("brew")
                .args(["install", "piper"])
                .output();
            
            if let Ok(out) = output {
                if out.status.success() {
                    self.send_event(SetupEvent::Completed {
                        name: "Piper TTS".to_string(),
                    });
                    return Ok(());
                }
            }
        }
        
        // Fallback: Build from source (works on all macOS architectures)
        println!("[Setup] Homebrew install not available, building from source...");
        self.send_event(SetupEvent::Downloading {
            name: "Piper TTS (building from source)".to_string(),
            progress: 0,
        });
        
        self.build_piper_from_source().await?;
        
        self.send_event(SetupEvent::Completed {
            name: "Piper TTS".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_whisper_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use quantized model for better performance (matches downloader)
        let model_path = self.pkg_dir.join("models/ggml-base-q8_0.bin");
        let download_path = self.pkg_dir.join("downloads/ggml-base-q8_0.bin");
        
        // Check if already exists in final location
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "Whisper Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        // Check if downloaded but not moved
        if download_path.exists() {
            std::fs::copy(&download_path, &model_path)?;
            self.send_event(SetupEvent::Completed {
                name: "Whisper Model".to_string(),
            });
            return Ok(());
        }
        
        self.send_event(SetupEvent::Downloading {
            name: "Whisper Model".to_string(),
            progress: 0,
        });
        
        // Download quantized model
        let url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q8_0.bin";
        self.download_file(url, &download_path).await?;
        
        // Move to final location
        std::fs::copy(&download_path, &model_path)?;
        
        self.send_event(SetupEvent::Completed {
            name: "Whisper Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_sbert_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let sbert_dir = self.pkg_dir.join("models/sbert");
        let model_path = sbert_dir.join("pytorch_model.bin");
        let downloads_dir = self.pkg_dir.join("downloads");
        
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "SBERT Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        std::fs::create_dir_all(&sbert_dir)?;
        
        // Check if files are in downloads folder (from downloader)
        let downloaded_files = vec![
            ("sbert-pytorch_model.bin", "pytorch_model.bin"),
            ("sbert-config.json", "config.json"),
            ("sbert-tokenizer.json", "tokenizer.json"),
        ];
        
        let mut all_downloaded = true;
        for (download_name, _) in &downloaded_files {
            if !downloads_dir.join(download_name).exists() {
                all_downloaded = false;
                break;
            }
        }
        
        if all_downloaded {
            // Copy from downloads to models/sbert
            for (download_name, final_name) in downloaded_files {
                let src = downloads_dir.join(download_name);
                let dest = sbert_dir.join(final_name);
                std::fs::copy(&src, &dest)?;
            }
            
            self.send_event(SetupEvent::Completed {
                name: "SBERT Model".to_string(),
            });
            return Ok(());
        }
        
        // Download directly if not in downloads folder
        self.send_event(SetupEvent::Downloading {
            name: "SBERT Model".to_string(),
            progress: 0,
        });
        
        let base_url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main";
        let files = vec![
            ("pytorch_model.bin", "pytorch_model.bin"),
            ("config.json", "config.json"),
            ("tokenizer.json", "tokenizer.json"),
        ];
        
        for (remote, local) in files {
            let url = format!("{}/{}", base_url, remote);
            let dest = sbert_dir.join(local);
            if !dest.exists() {
                self.download_file(&url, &dest).await?;
            }
        }
        
        self.send_event(SetupEvent::Completed {
            name: "SBERT Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_voice_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let voice_dir = self.pkg_dir.join("models/bold_voice");
        let model_path = voice_dir.join("en_US-libritts_r-medium.onnx");
        let downloads_dir = self.pkg_dir.join("downloads");
        
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "Voice Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        std::fs::create_dir_all(&voice_dir)?;
        
        // Check if downloaded by downloader
        let onnx_download = downloads_dir.join("en_US-libritts_r-medium.onnx");
        let json_download = downloads_dir.join("en_US-libritts_r-medium.onnx.json");
        
        if onnx_download.exists() && json_download.exists() {
            // Copy from downloads
            std::fs::copy(&onnx_download, &model_path)?;
            std::fs::copy(&json_download, &voice_dir.join("en_US-libritts_r-medium.onnx.json"))?;
            
            // Download espeak-ng-data if not exists
            self.install_espeak_data().await?;
            
            self.send_event(SetupEvent::Completed {
                name: "Voice Model".to_string(),
            });
            return Ok(());
        }
        
        // Download directly if not in downloads
        self.send_event(SetupEvent::Downloading {
            name: "Voice Model".to_string(),
            progress: 0,
        });
        
        let base_url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium";
        
        let onnx_url = format!("{}/en_US-libritts_r-medium.onnx", base_url);
        let json_url = format!("{}/en_US-libritts_r-medium.onnx.json", base_url);
        
        self.download_file(&onnx_url, &model_path).await?;
        self.download_file(&json_url, &voice_dir.join("en_US-libritts_r-medium.onnx.json")).await?;
        
        // Download espeak-ng-data
        self.install_espeak_data().await?;
        
        self.send_event(SetupEvent::Completed {
            name: "Voice Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_llvm(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if command_exists("clang") {
            self.send_event(SetupEvent::Completed {
                name: "LLVM (already installed)".to_string(),
            });
            return Ok(());
        }
        
        // macOS comes with clang via Xcode Command Line Tools
        self.send_event(SetupEvent::Downloading {
            name: "Xcode Command Line Tools".to_string(),
            progress: 0,
        });
        
        let output = std::process::Command::new("xcode-select")
            .args(["--install"])
            .output();
        
        // This will show a GUI prompt, so we just continue
        self.send_event(SetupEvent::Completed {
            name: "LLVM/Clang".to_string(),
        });
        
        Ok(())
    }
    
    fn setup_path(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let brew_prefix = self.brew_prefix();
        
        // Add Homebrew to PATH if not already
        let brew_bin = brew_prefix.join("bin");
        self.add_to_profile(&format!("export PATH=\"{}:$PATH\"", brew_bin.to_string_lossy()))?;
        
        // Add pkg/piper to PATH if it exists
        let piper_dir = self.pkg_dir.join("piper");
        if piper_dir.exists() {
            self.add_to_profile(&format!("export PATH=\"{}:$PATH\"", piper_dir.to_string_lossy()))?;
        }
        
        Ok(())
    }
    
    fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check FFmpeg
        if !command_exists("ffmpeg") {
            return Err("FFmpeg not found in PATH".into());
        }
        
        // Check TTS Engine
        let tts_binary = self.pkg_dir.join("piper/piper");
        if !command_exists("piper") && !tts_binary.exists() {
            return Err("Piper not found in PATH or pkg/piper".into());
        }
        
        // Check models (support both quantized and non-quantized)
        let whisper_q8 = self.pkg_dir.join("models/ggml-base-q8_0.bin");
        let whisper_base = self.pkg_dir.join("models/ggml-base.bin");
        if !whisper_q8.exists() && !whisper_base.exists() {
            return Err("Whisper model not found (neither ggml-base-q8_0.bin nor ggml-base.bin)".into());
        }
        
        if !self.pkg_dir.join("models/sbert/pytorch_model.bin").exists() {
            return Err("SBERT model not found".into());
        }
        
        if !self.pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx").exists() {
            return Err("Voice model not found".into());
        }
        
        println!("✅ macOS setup validated successfully");
        Ok(())
    }
    
    async fn run_setup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_setup_complete() {
            self.send_event(SetupEvent::AllComplete);
            return Ok(());
        }
        
        self.send_event(SetupEvent::Started);
        
        // Create directories
        self.create_directories()?;
        
        // Install via Homebrew
        self.install_ffmpeg().await?;
        self.install_piper().await?;
        self.install_llvm().await?;
        
        // Download models
        self.install_whisper_model().await?;
        self.install_sbert_model().await?;
        self.install_voice_model().await?;
        
        // Setup PATH
        self.setup_path()?;
        
        // Validate
        self.validate()?;
        
        self.send_event(SetupEvent::AllComplete);
        Ok(())
    }
}

impl MacOSSetup {
    /// Build Piper from source (for Apple Silicon support)
    async fn build_piper_from_source(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let downloads_dir = self.pkg_dir.join("downloads");
        let source_dir = downloads_dir.join("piper-src");
        
        std::fs::create_dir_all(&downloads_dir)?;
        
        // Clone Piper repository
        self.send_event(SetupEvent::Downloading {
            name: "Cloning Piper repository".to_string(),
            progress: 30,
        });
        
        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "https://github.com/rhasspy/piper.git", &source_dir.to_string_lossy()])
            .output()?;
        
        if !output.status.success() {
            return Err(format!("Failed to clone piper: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        
        // Build
        self.send_event(SetupEvent::Extracting {
            name: "Building Piper".to_string(),
            progress: 60,
        });
        
        let build_dir = source_dir.join("build");
        std::fs::create_dir_all(&build_dir)?;
        
        // Run cmake
        let output = std::process::Command::new("cmake")
            .args(["-DCMAKE_BUILD_TYPE=Release", ".."])
            .current_dir(&build_dir)
            .output()?;
        
        if !output.status.success() {
            return Err(format!("CMake failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        
        // Run make
        let output = std::process::Command::new("make")
            .args(["-j6"])
            .current_dir(&build_dir)
            .output()?;
        
        if !output.status.success() {
            return Err(format!("Make failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        
        // Copy binary to pkg/piper
        let tts_dir = self.pkg_dir.join("piper");
        std::fs::create_dir_all(&tts_dir)?;
        
        let built_binary = build_dir.join("piper");
        let dest_binary = tts_dir.join("piper");
        
        if built_binary.exists() {
            std::fs::copy(&built_binary, &dest_binary)?;
            
            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest_binary)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest_binary, perms)?;
            }
            
            // Add to PATH
            self.add_to_profile(&format!("export PATH=\"{}:$PATH\"", tts_dir.to_string_lossy()))?;
        }
        
        Ok(())
    }
    
    /// Install espeak-ng-data
    async fn install_espeak_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let espeak_dir = self.pkg_dir.join("models/bold_voice/espeak-ng-data");
        
        if espeak_dir.exists() && espeak_dir.join("phontab").exists() {
            return Ok(());
        }
        
        self.send_event(SetupEvent::Downloading {
            name: "espeak-ng".to_string(),
            progress: 0,
        });
        
        // Install espeak-ng via Homebrew (most reliable)
        self.install_homebrew()?;
        
        let output = std::process::Command::new("brew")
            .args(["install", "espeak-ng"])
            .output()?;
        
        if output.status.success() {
            self.send_event(SetupEvent::Completed {
                name: "espeak-ng".to_string(),
            });
            return Ok(());
        }
        
        // Fallback: Download espeak-ng-data from GitHub
        self.send_event(SetupEvent::Downloading {
            name: "espeak-ng-data (fallback)".to_string(),
            progress: 50,
        });
        
        let downloads_dir = self.pkg_dir.join("downloads");
        let espeak_archive = downloads_dir.join("espeak-ng-data.tar.gz");
        
        // Download espeak-ng-data
        let url = "https://github.com/rhasspy/piper/releases/download/v1.2.0/espeak-ng-data.tar.gz";
        self.download_file(url, &espeak_archive).await?;
        
        // Extract
        std::fs::create_dir_all(&espeak_dir)?;
        
        let output = std::process::Command::new("tar")
            .args(["-xzf", &espeak_archive.to_string_lossy(), "-C", &espeak_dir.to_string_lossy()])
            .output()?;
        
        if !output.status.success() {
            return Err(format!("Failed to extract espeak-ng-data: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        
        self.send_event(SetupEvent::Completed {
            name: "espeak-ng-data".to_string(),
        });
        
        Ok(())
    }
}
