// src/setup_manager/platforms/windows.rs
// Windows-specific setup: Download binaries and extract

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use crate::setup_manager::{SetupEvent, FileDownloader, FileExtractor, SetupValidator};
use super::PlatformSetup;

pub struct WindowsSetup {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
}

impl WindowsSetup {
    pub fn new(pkg_dir: PathBuf, event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>) -> Self {
        Self { pkg_dir, event_sender }
    }
    
    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
}

impl PlatformSetup for WindowsSetup {
    fn pkg_dir(&self) -> &PathBuf {
        &self.pkg_dir
    }
    
    fn is_setup_complete(&self) -> bool {
        let required_paths = vec![
            self.pkg_dir.join("audio"),
            self.pkg_dir.join("ffmpeg"),
            self.pkg_dir.join("piper"),
            self.pkg_dir.join("models/ggml-base-q8_0.bin"),
            self.pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx"),
        ];
        
        let llvm_ok = PathBuf::from("C:/LLVM/bin/clang.exe").exists();
        
        required_paths.iter().all(|p| p.exists()) && llvm_ok
    }
    
    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dirs = vec![
            self.pkg_dir.join("audio"),
            self.pkg_dir.join("ffmpeg"),
            self.pkg_dir.join("piper"),
            self.pkg_dir.join("models/bold_voice"),
            self.pkg_dir.join("downloads"),
        ];
        
        for dir in dirs {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
    
    async fn install_ffmpeg(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "FFmpeg".to_string(),
            progress: 0,
        });
        
        // FFmpeg is downloaded and extracted via FileDownloader/FileExtractor
        // This is handled in run_setup() which calls download_all and extract_all
        Ok(())
    }
    
    async fn install_piper(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "Piper TTS".to_string(),
            progress: 0,
        });
        Ok(())
    }
    
    async fn install_whisper_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "Whisper Model".to_string(),
            progress: 0,
        });
        Ok(())
    }
    
    async fn install_sbert_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "SBERT Model".to_string(),
            progress: 0,
        });
        Ok(())
    }
    
    async fn install_voice_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "Voice Model".to_string(),
            progress: 0,
        });
        Ok(())
    }
    
    async fn install_llvm(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_event(SetupEvent::Downloading {
            name: "LLVM".to_string(),
            progress: 0,
        });
        Ok(())
    }
    
    fn setup_path(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let llvm_bin = PathBuf::from("C:/LLVM/bin");
        let llvm_bin_str = llvm_bin.to_string_lossy().replace('/', "\\");
        
        let ps_script = format!(r#"
$llvmBin = '{}';
$targets = @('Machine','User');
foreach ($t in $targets) {{
  try {{
    $cur = [Environment]::GetEnvironmentVariable('Path', $t);
    if ($null -eq $cur) {{ $cur = '' }}
    $parts = $cur -split ';' | Where-Object {{ $_ -and $_.Trim() -ne '' }}
    if ($parts -notcontains $llvmBin) {{
      $new = ($parts + $llvmBin) -join ';';
      [Environment]::SetEnvironmentVariable('Path', $new, $t);
    }}
    break;
  }} catch {{ }}
}}
"#, llvm_bin_str);
        
        let output = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&ps_script)
            .output()?;
        
        if !output.status.success() {
            return Err(format!(
                "Failed to update PATH: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into());
        }
        
        Ok(())
    }
    
    fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let validator = SetupValidator::new(self.pkg_dir.clone());
        validator.validate_all().map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
            format!("{}", e).into() 
        })
    }
    
    async fn run_setup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_setup_complete() {
            self.send_event(SetupEvent::AllComplete);
            return Ok(());
        }
        
        self.send_event(SetupEvent::Started);
        
        // Create directories
        self.create_directories()?;
        
        // Download all files (Windows uses binary downloads)
        let downloader = FileDownloader::new(self.pkg_dir.clone(), self.event_sender.clone());
        downloader.download_all().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("{}", e).into()
        })?;
        
        // Extract archives
        let extractor = FileExtractor::new(self.pkg_dir.clone(), self.event_sender.clone());
        extractor.extract_all().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("{}", e).into()
        })?;
        
        // Setup PATH for LLVM
        if let Err(e) = self.setup_path() {
            self.send_event(SetupEvent::Error {
                task: "PATH".to_string(),
                message: format!("Failed to update PATH: {}", e),
            });
        }
        
        // Validate
        self.validate()?;
        
        self.send_event(SetupEvent::AllComplete);
        Ok(())
    }
}
