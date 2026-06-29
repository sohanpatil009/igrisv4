// src/setup_manager/extractor.rs
// Handles extracting and organizing downloaded files

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use std::fs;
use crate::setup_manager::SetupEvent;
use dirs;

pub struct ExtractTask {
    pub name: &'static str,
    pub archive: String,
    pub extract_to: PathBuf,
    pub flatten: bool, // Flatten directory structure
}

pub struct FileExtractor {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
}

impl FileExtractor {
    pub fn new(pkg_dir: PathBuf, event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>) -> Self {
        Self {
            pkg_dir,
            event_sender,
        }
    }

    pub async fn extract_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        // On macOS, install LLVM via Homebrew instead of extracting
        #[cfg(target_os = "macos")]
        {
            self.install_llvm_via_brew().await?;
        }
        
        let tasks = self.get_extract_tasks();

        for task in tasks {
            self.extract_file(&task).await?;
        }
        
        // Cleanup downloads folder after extraction - no longer needed
        self.cleanup_downloads()?;

        Ok(())
    }
    
    /// Remove downloads folder after extraction to save space
    fn cleanup_downloads(&self) -> Result<(), Box<dyn std::error::Error>> {
        let downloads_dir = self.pkg_dir.join("downloads");
        if downloads_dir.exists() {
            println!("[Setup] Cleaning up downloads folder to save space...");
            fs::remove_dir_all(&downloads_dir)?;
            println!("[Setup] Downloads folder removed");
        }
        Ok(())
    }
    
    /// Install LLVM via Homebrew on macOS and set environment variables
    #[cfg(target_os = "macos")]
    async fn install_llvm_via_brew(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;
        
        // Check if LLVM is already installed
        let llvm_check = Command::new("llvm-config").arg("--version").output();
        if llvm_check.is_ok() && llvm_check.unwrap().status.success() {
            println!("[Setup] LLVM already installed");
            return Ok(());
        }
        
        self.send_event(SetupEvent::Extracting {
            name: "LLVM (Homebrew)".to_string(),
            progress: 0,
        });
        
        // Check if Homebrew is installed
        let brew_check = Command::new("brew").arg("--version").output();
        if brew_check.is_err() || !brew_check.unwrap().status.success() {
            return Err("Homebrew is not installed. Please install Homebrew first: https://brew.sh".into());
        }
        
        println!("[Setup] Installing LLVM via Homebrew...");
        
        self.send_event(SetupEvent::Extracting {
            name: "LLVM (Homebrew)".to_string(),
            progress: 30,
        });
        
        // Install LLVM via brew
        let install_result = Command::new("brew")
            .args(["install", "llvm"])
            .output()?;
        
        if !install_result.status.success() {
            let stderr = String::from_utf8_lossy(&install_result.stderr);
            return Err(format!("Failed to install LLVM via Homebrew: {}", stderr).into());
        }
        
        self.send_event(SetupEvent::Extracting {
            name: "LLVM (Homebrew)".to_string(),
            progress: 80,
        });
        
        // Get LLVM path from brew
        let llvm_prefix = Command::new("brew")
            .args(["--prefix", "llvm"])
            .output()?;
        
        let llvm_path = String::from_utf8_lossy(&llvm_prefix.stdout).trim().to_string();
        
        if !llvm_path.is_empty() {
            println!("[Setup] LLVM installed at: {}", llvm_path);
            
            // Set environment variables in shell profile
            self.setup_llvm_env_macos(&llvm_path)?;
        }
        
        self.send_event(SetupEvent::Extracting {
            name: "LLVM (Homebrew)".to_string(),
            progress: 100,
        });
        
        println!("[Setup] LLVM installation complete");
        Ok(())
    }
    
    /// Setup LLVM environment variables on macOS
    #[cfg(target_os = "macos")]
    fn setup_llvm_env_macos(&self, llvm_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        
        let home = std::env::var("HOME")?;
        
        // Environment variable exports
        let env_exports = format!(
            r#"
# LLVM Configuration (added by IGRIS)
export LLVM_SYS_180_PREFIX="{llvm_path}"
export PATH="{llvm_path}/bin:$PATH"
export LDFLAGS="-L{llvm_path}/lib"
export CPPFLAGS="-I{llvm_path}/include"
"#
        );
        
        // Try to add to .zshrc (default on modern macOS)
        let zshrc_path = format!("{}/.zshrc", home);
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&zshrc_path)
        {
            // Check if already added
            let content = fs::read_to_string(&zshrc_path).unwrap_or_default();
            if !content.contains("LLVM_SYS_180_PREFIX") {
                writeln!(file, "{}", env_exports)?;
                println!("[Setup] Added LLVM env vars to ~/.zshrc");
            }
        }
        
        // Also try .bash_profile for older systems
        let bash_profile_path = format!("{}/.bash_profile", home);
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&bash_profile_path)
        {
            let content = fs::read_to_string(&bash_profile_path).unwrap_or_default();
            if !content.contains("LLVM_SYS_180_PREFIX") {
                writeln!(file, "{}", env_exports)?;
                println!("[Setup] Added LLVM env vars to ~/.bash_profile");
            }
        }
        
        // Set for current process
        std::env::set_var("LLVM_SYS_180_PREFIX", llvm_path);
        std::env::set_var("PATH", format!("{}/bin:{}", llvm_path, std::env::var("PATH").unwrap_or_default()));
        
        Ok(())
    }

    fn llvm_install_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from("C:/LLVM")
        } else {
            // Avoid writing into /usr/local without elevation.
            // Use a user-writable directory by default.
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".igris")
                .join("llvm")
        }
    }

    async fn extract_file(&self, task: &ExtractTask) -> Result<(), Box<dyn std::error::Error>> {
        self.send_event(SetupEvent::Extracting {
            name: task.name.to_string(),
            progress: 0,
        });

        let archive_path = self.pkg_dir.join("downloads").join(&task.archive);
        let extract_dir = if task.extract_to.is_absolute() {
            task.extract_to.clone()
        } else {
            self.pkg_dir.join(&task.extract_to)
        };

        // Skip if already extracted
        if extract_dir.exists() && !self.is_empty_dir(&extract_dir) {
            self.send_event(SetupEvent::Extracting {
                name: task.name.to_string(),
                progress: 100,
            });
            return Ok(());
        }

        // Determine archive type and extract accordingly
        let extension = archive_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        match extension {
            "zip" => {
                fs::create_dir_all(&extract_dir)?;
                self.extract_zip(&archive_path, &extract_dir, task).await?;
                if task.flatten {
                    self.flatten_directory(&extract_dir)?;
                }
            }
            "gz" => {
                // Handle .tar.gz files
                fs::create_dir_all(&extract_dir)?;
                self.extract_tar_gz(&archive_path, &extract_dir).await?;
                if task.flatten {
                    self.flatten_directory(&extract_dir)?;
                }
            }
            "xz" => {
                // Handle .tar.xz files
                fs::create_dir_all(&extract_dir)?;
                self.extract_tar_xz(&archive_path, &extract_dir).await?;
                if task.flatten {
                    self.flatten_directory(&extract_dir)?;
                }
            }
            _ => {
                // Direct copy for .bin, .onnx, and other files
                if archive_path.exists() {
                    fs::create_dir_all(extract_dir.parent().unwrap())?;
                    fs::copy(&archive_path, &extract_dir)?;
                }
            }
        }

        self.send_event(SetupEvent::Extracting {
            name: task.name.to_string(),
            progress: 100,
        });

        Ok(())
    }

    /// Extract zip file (cross-platform)
    async fn extract_zip(
        &self,
        archive_path: &PathBuf,
        extract_dir: &PathBuf,
        _task: &ExtractTask,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if cfg!(target_os = "windows") {
            // Use PowerShell to extract zip (built-in on Windows)
            let ps_cmd = format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                archive_path.display(),
                extract_dir.display()
            );

            std::process::Command::new("powershell")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(ps_cmd)
                .output()?;
        } else {
            // Use unzip command on Unix-like systems
            std::process::Command::new("unzip")
                .arg("-o")
                .arg(archive_path)
                .arg("-d")
                .arg(extract_dir)
                .output()?;
        }

        Ok(())
    }

    /// Extract tar.gz file (Linux/macOS)
    async fn extract_tar_gz(
        &self,
        archive_path: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::process::Command::new("tar")
            .arg("-xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(extract_dir)
            .output()?;

        Ok(())
    }

    /// Extract tar.xz file (Linux)
    async fn extract_tar_xz(
        &self,
        archive_path: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::process::Command::new("tar")
            .arg("-xJf")
            .arg(archive_path)
            .arg("-C")
            .arg(extract_dir)
            .output()?;

        Ok(())
    }

    fn flatten_directory(&self, dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();

        // If there's only one subdirectory, move its contents up
        if entries.len() == 1 {
            let entry = &entries[0];
            let path = entry.path();
            if path.is_dir() {
                // Get all files from nested directory
                let nested_entries: Vec<_> = fs::read_dir(&path)?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect();

                // Move them to parent
                for nested_path in nested_entries {
                    let filename = nested_path.file_name().unwrap();
                    let dest = dir.join(&filename);

                    if nested_path.is_dir() {
                        fs::create_dir_all(&dest).ok();
                        copy_dir_recursive(&nested_path, &dest)?;
                    } else {
                        fs::copy(&nested_path, &dest)?;
                    }
                }

                // Remove the empty nested directory
                fs::remove_dir_all(&path)?;
            }
        }

        Ok(())
    }

    fn is_empty_dir(&self, dir: &PathBuf) -> bool {
        fs::read_dir(dir).map_or(true, |mut entries| entries.next().is_none())
    }

    fn get_extract_tasks(&self) -> Vec<ExtractTask> {
        let piper_archive = if cfg!(target_os = "windows") {
            "piper.zip"
        } else {
            "piper.tar.gz" // Linux and macOS
        };

        #[allow(unused_mut)]
        let mut tasks = vec![
            ExtractTask {
                name: "Piper TTS",
                archive: piper_archive.to_string(),
                extract_to: PathBuf::from("piper"),
                flatten: true,
            },
            ExtractTask {
                name: "Whisper Model",
                archive: "ggml-base-q8_0.bin".to_string(),
                extract_to: PathBuf::from("models/ggml-base-q8_0.bin"),
                flatten: false,
            },
            ExtractTask {
                name: "SBERT Model Files",
                archive: "sbert-pytorch_model.bin".to_string(),
                extract_to: PathBuf::from("models/sbert/pytorch_model.bin"),
                flatten: false,
            },
            ExtractTask {
                name: "SBERT Config",
                archive: "sbert-config.json".to_string(),
                extract_to: PathBuf::from("models/sbert/config.json"),
                flatten: false,
            },
            ExtractTask {
                name: "SBERT Tokenizer",
                archive: "sbert-tokenizer.json".to_string(),
                extract_to: PathBuf::from("models/sbert/tokenizer.json"),
                flatten: false,
            },
            ExtractTask {
                name: "Voice Model",
                archive: "en_US-libritts_r-medium.onnx".to_string(),
                extract_to: PathBuf::from("models/bold_voice/en_US-libritts_r-medium.onnx"),
                flatten: false,
            },
            ExtractTask {
                name: "Voice Model Config",
                archive: "en_US-libritts_r-medium.onnx.json".to_string(),
                extract_to: PathBuf::from("models/bold_voice/en_US-libritts_r-medium.onnx.json"),
                flatten: false,
            },
        ];
        
        // Add FFmpeg extraction task (Windows and Linux only, macOS uses brew)
        #[cfg(not(target_os = "macos"))]
        {
            let ffmpeg_archive = if cfg!(target_os = "windows") {
                "ffmpeg.zip"
            } else {
                "ffmpeg.tar.xz"
            };
            
            if self.pkg_dir.join("downloads").join(ffmpeg_archive).exists() {
                tasks.push(ExtractTask {
                    name: "FFmpeg",
                    archive: ffmpeg_archive.to_string(),
                    extract_to: PathBuf::from("ffmpeg"),
                    flatten: true,
                });
            }
        }
        
        // Only add LLVM extraction task on Windows and Linux (macOS uses brew)
        #[cfg(not(target_os = "macos"))]
        {
            let llvm_archive = if self.pkg_dir.join("downloads/llvm.zip").exists() {
                "llvm.zip"
            } else {
                "llvm.tar.xz"
            };
            
            tasks.insert(0, ExtractTask {
                name: "LLVM",
                archive: llvm_archive.to_string(),
                extract_to: Self::llvm_install_dir(),
                flatten: true,
            });
        }
        
        tasks
    }

    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}