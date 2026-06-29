// src/setup_manager/platforms/linux.rs
// Linux-specific setup: apt/dnf/pacman install + PATH configuration

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use crate::setup_manager::SetupEvent;
use super::{PlatformSetup, command_exists, run_command};

/// Detected Linux package manager
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
    Apt,      // Debian/Ubuntu
    Dnf,      // Fedora/RHEL
    Pacman,   // Arch
    Zypper,   // openSUSE
    Unknown,
}

impl PackageManager {
    pub fn detect() -> Self {
        if command_exists("apt") {
            PackageManager::Apt
        } else if command_exists("dnf") {
            PackageManager::Dnf
        } else if command_exists("pacman") {
            PackageManager::Pacman
        } else if command_exists("zypper") {
            PackageManager::Zypper
        } else {
            PackageManager::Unknown
        }
    }
    
    pub fn install_cmd(&self) -> (&str, &[&str]) {
        match self {
            PackageManager::Apt => ("sudo", &["apt", "install", "-y"]),
            PackageManager::Dnf => ("sudo", &["dnf", "install", "-y"]),
            PackageManager::Pacman => ("sudo", &["pacman", "-S", "--noconfirm"]),
            PackageManager::Zypper => ("sudo", &["zypper", "install", "-y"]),
            PackageManager::Unknown => ("echo", &["No package manager found"]),
        }
    }
    
    pub fn ffmpeg_package(&self) -> &str {
        "ffmpeg"
    }
    
    pub fn llvm_package(&self) -> &str {
        match self {
            PackageManager::Apt => "clang",
            PackageManager::Dnf => "clang",
            PackageManager::Pacman => "clang",
            PackageManager::Zypper => "clang",
            PackageManager::Unknown => "clang",
        }
    }
}

pub struct LinuxSetup {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
    package_manager: PackageManager,
}

impl LinuxSetup {
    pub fn new(pkg_dir: PathBuf, event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>) -> Self {
        Self {
            pkg_dir,
            event_sender,
            package_manager: PackageManager::detect(),
        }
    }
    
    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
    
    /// Install a package using the detected package manager
    fn install_package(&self, package: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: package.to_string(),
            progress: 0,
        });
        
        let (cmd, base_args) = self.package_manager.install_cmd();
        
        let mut args: Vec<&str> = base_args.to_vec();
        args.push(package);
        
        let output = std::process::Command::new(cmd)
            .args(&args)
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
    
    /// Download a file using curl or wget
    async fn download_file(&self, url: &str, dest: &PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let output = if command_exists("curl") {
            std::process::Command::new("curl")
                .args(["-L", "-o", &dest.to_string_lossy(), url])
                .output()?
        } else if command_exists("wget") {
            std::process::Command::new("wget")
                .args(["-O", &dest.to_string_lossy(), url])
                .output()?
        } else {
            return Err("Neither curl nor wget found".into());
        };
        
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
            home.join(".bashrc"),     // Most common on Linux
            home.join(".zshrc"),
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
        
        // Create .bashrc if no profile exists
        let bashrc = home.join(".bashrc");
        std::fs::write(&bashrc, format!("{}\n", export_line))?;
        
        Ok(())
    }
}

impl PlatformSetup for LinuxSetup {
    fn pkg_dir(&self) -> &PathBuf {
        &self.pkg_dir
    }
    
    fn is_setup_complete(&self) -> bool {
        let system_packages_ok = command_exists("ffmpeg") && command_exists("clang");
        
        let models_ok = self.pkg_dir.join("models/ggml-base.bin").exists()
            && self.pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx").exists()
            && self.pkg_dir.join("models/sbert/pytorch_model.bin").exists();
        
        // Check piper (either system or local)
        let piper_ok = command_exists("piper") || self.pkg_dir.join("piper/piper").exists();
        
        system_packages_ok && models_ok && piper_ok
    }
    
    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dirs = vec![
            self.pkg_dir.join("audio"),
            self.pkg_dir.join("piper"),
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
        
        self.install_package(self.package_manager.ffmpeg_package())?;
        Ok(())
    }
    
    async fn install_piper(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if command_exists("piper") {
            self.send_event(SetupEvent::Completed {
                name: "Piper (already installed)".to_string(),
            });
            return Ok(());
        }
        
        // Download piper binary for Linux
        self.send_event(SetupEvent::Downloading {
            name: "Piper TTS".to_string(),
            progress: 0,
        });
        
        let arch = if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x86_64"
        };
        
        let piper_url = format!(
            "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_{}.tar.gz",
            arch
        );
        
        let download_path = self.pkg_dir.join("downloads/piper.tar.gz");
        self.download_file(&piper_url, &download_path).await?;
        
        // Extract
        let piper_dir = self.pkg_dir.join("piper");
        std::fs::create_dir_all(&piper_dir)?;
        
        let output = std::process::Command::new("tar")
            .args(["-xzf", &download_path.to_string_lossy(), "-C", &piper_dir.to_string_lossy(), "--strip-components=1"])
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to extract piper: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        // Make executable
        let piper_bin = piper_dir.join("piper");
        if piper_bin.exists() {
            std::process::Command::new("chmod")
                .args(["+x", &piper_bin.to_string_lossy()])
                .output()?;
        }
        
        self.send_event(SetupEvent::Completed {
            name: "Piper TTS".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_whisper_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let model_path = self.pkg_dir.join("models/ggml-base-q8_0.bin");
        
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "Whisper Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        self.send_event(SetupEvent::Downloading {
            name: "Whisper Model (q8_0)".to_string(),
            progress: 0,
        });
        
        std::fs::create_dir_all(model_path.parent().unwrap())?;
        
        // Using quantized q8_0 model for faster inference
        let url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q8_0.bin";
        self.download_file(url, &model_path).await?;
        
        self.send_event(SetupEvent::Completed {
            name: "Whisper Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_sbert_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let sbert_dir = self.pkg_dir.join("models/sbert");
        let model_path = sbert_dir.join("pytorch_model.bin");
        
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "SBERT Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        self.send_event(SetupEvent::Downloading {
            name: "SBERT Model".to_string(),
            progress: 0,
        });
        
        std::fs::create_dir_all(&sbert_dir)?;
        
        let base_url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main";
        let files = vec![
            ("pytorch_model.bin", "pytorch_model.bin"),
            ("config.json", "config.json"),
            ("tokenizer.json", "tokenizer.json"),
            ("vocab.txt", "vocab.txt"),
        ];
        
        for (remote, local) in files {
            let url = format!("{}/{}", base_url, remote);
            let dest = sbert_dir.join(local);
            self.download_file(&url, &dest).await?;
        }
        
        self.send_event(SetupEvent::Completed {
            name: "SBERT Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_voice_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let voice_dir = self.pkg_dir.join("models/bold_voice");
        let model_path = voice_dir.join("en_US-libritts_r-medium.onnx");
        
        if model_path.exists() {
            self.send_event(SetupEvent::Completed {
                name: "Voice Model (already exists)".to_string(),
            });
            return Ok(());
        }
        
        std::fs::create_dir_all(&voice_dir)?;
        
        self.send_event(SetupEvent::Downloading {
            name: "Voice Model".to_string(),
            progress: 0,
        });
        
        let base_url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium";
        
        let onnx_url = format!("{}/en_US-libritts_r-medium.onnx", base_url);
        let json_url = format!("{}/en_US-libritts_r-medium.onnx.json", base_url);
        
        self.download_file(&onnx_url, &model_path).await?;
        self.download_file(&json_url, &voice_dir.join("en_US-libritts_r-medium.onnx.json")).await?;
        
        self.send_event(SetupEvent::Completed {
            name: "Voice Model".to_string(),
        });
        
        Ok(())
    }
    
    async fn install_llvm(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if command_exists("clang") {
            self.send_event(SetupEvent::Completed {
                name: "LLVM/Clang (already installed)".to_string(),
            });
            return Ok(());
        }
        
        self.install_package(self.package_manager.llvm_package())?;
        Ok(())
    }
    
    fn setup_path(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Add pkg/piper to PATH if it exists
        let piper_dir = self.pkg_dir.join("piper");
        if piper_dir.exists() {
            let piper_path = std::fs::canonicalize(&piper_dir)?;
            self.add_to_profile(&format!("export PATH=\"{}:$PATH\"", piper_path.to_string_lossy()))?;
        }
        
        Ok(())
    }
    
    fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !command_exists("ffmpeg") {
            return Err("FFmpeg not found".into());
        }
        
        if !command_exists("clang") {
            return Err("Clang/LLVM not found in PATH".into());
        }
        
        // Check piper
        let piper_ok = command_exists("piper") || self.pkg_dir.join("piper/piper").exists();
        if !piper_ok {
            return Err("Piper not found".into());
        }
        
        // Check models
        if !self.pkg_dir.join("models/ggml-base-q8_0.bin").exists() {
            return Err("Whisper model not found".into());
        }
        
        if !self.pkg_dir.join("models/sbert/pytorch_model.bin").exists() {
            return Err("SBERT model not found".into());
        }
        
        if !self.pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx").exists() {
            return Err("Voice model not found".into());
        }
        
        Ok(())
    }
    
    async fn run_setup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_setup_complete() {
            self.send_event(SetupEvent::AllComplete);
            return Ok(());
        }
        
        self.send_event(SetupEvent::Started);
        
        println!("📦 Detected package manager: {:?}", self.package_manager);
        
        // Create directories
        self.create_directories()?;
        
        // Install system packages
        self.install_ffmpeg().await?;
        self.install_llvm().await?;
        self.install_piper().await?;
        
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
