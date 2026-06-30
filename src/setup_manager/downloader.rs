// src/setup_manager/downloader.rs
// Handles downloading all required files with progress tracking
// Platform-specific downloads for Windows, Linux, and macOS

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use reqwest::Client;
use std::fs::File;
use std::io::Write;
use crate::setup_manager::SetupEvent;
use regex::Regex;
use std::process::Command;

#[derive(Clone)]
pub struct DownloadFile {
    pub name: &'static str,
    pub url: String,
    pub filename: String,
}

/// Platform-specific download URLs
#[derive(Clone)]
pub struct PlatformUrls {
    pub windows: &'static str,
    pub linux: &'static str,
    pub macos: &'static str,
}

impl PlatformUrls {
    /// Get the URL for the current platform
    pub fn get_current(&self) -> &'static str {
        if cfg!(target_os = "windows") {
            self.windows
        } else if cfg!(target_os = "linux") {
            self.linux
        } else if cfg!(target_os = "macos") {
            self.macos
        } else {
            // Fallback to Linux for other Unix-like systems
            self.linux
        }
    }
}

pub struct FileDownloader {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
}

impl FileDownloader {
    pub fn new(pkg_dir: PathBuf, event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>) -> Self {
        Self {
            pkg_dir,
            event_sender,
        }
    }

    pub async fn download_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let files = self.get_download_list().await?;

        for file in files {
            self.download_file(&file).await?;
        }

        Ok(())
    }

    async fn download_file(&self, file: &DownloadFile) -> Result<(), Box<dyn std::error::Error>> {
        self.send_event(SetupEvent::Downloading {
            name: file.name.to_string(),
            progress: 0,
        });

        let download_path = self.pkg_dir.join("downloads");
        let file_path = download_path.join(&file.filename);

        // Skip if already downloaded
        if file_path.exists() {
            self.send_event(SetupEvent::Downloading {
                name: file.name.to_string(),
                progress: 100,
            });
            return Ok(());
        }

        let client = Client::new();
        let response = client.get(&file.url).send().await?;

        let total_size = response.content_length().unwrap_or(0);
        let mut file_handle = File::create(&file_path)?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file_handle.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let progress = if total_size > 0 {
                ((downloaded as f64 / total_size as f64) * 100.0) as u32
            } else {
                0
            };

            self.send_event(SetupEvent::Downloading {
                name: file.name.to_string(),
                progress,
            });
        }

        Ok(())
    }

    async fn get_download_list(&self) -> Result<Vec<DownloadFile>, Box<dyn std::error::Error>> {
        // Piper TTS URLs for different platforms
        let piper_urls = PlatformUrls {
            windows: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip",
            linux: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz",
            macos: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_x64.tar.gz",
        };

        let piper_filename = if cfg!(target_os = "windows") {
            "piper.zip"
        } else {
            "piper.tar.gz" // Linux and macOS
        };

        let client = Client::new();

        // Check if LLVM is installed
        let llvm_installed = is_llvm_installed();

        let mut files = vec![
            DownloadFile {
                name: "Piper TTS",
                url: piper_urls.get_current().to_string(),
                filename: piper_filename.to_string(),
            },
            // FFmpeg for camera photo/video with audio
            DownloadFile {
                name: "FFmpeg",
                url: get_ffmpeg_url(),
                filename: get_ffmpeg_filename(),
            },
            // SenseVoice STT model files (direct HuggingFace downloads)
            DownloadFile {
                name: "SenseVoice Model",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.onnx".to_string(),
                filename: "sense-voice-model.onnx".to_string(),
            },
            DownloadFile {
                name: "SenseVoice Tokens",
                url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt".to_string(),
                filename: "sense-voice-tokens.txt".to_string(),
            },
            DownloadFile {
                name: "Voice Model",
                url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx".to_string(),
                filename: "en_US-libritts_r-medium.onnx".to_string(),
            },
            DownloadFile {
                name: "Voice Model JSON",
                url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx.json".to_string(),
                filename: "en_US-libritts_r-medium.onnx.json".to_string(),
            },
            // SBERT model files for NLU semantic understanding
            DownloadFile {
                name: "SBERT Model",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/pytorch_model.bin".to_string(),
                filename: "sbert-pytorch_model.bin".to_string(),
            },
            DownloadFile {
                name: "SBERT Config",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json".to_string(),
                filename: "sbert-config.json".to_string(),
            },
            DownloadFile {
                name: "SBERT Tokenizer",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json".to_string(),
                filename: "sbert-tokenizer.json".to_string(),
            },
        ];

        // Local reasoning LLM (Qwen 2.5 1.5B GGUF) — optional, enables smart fallback
        files.push(DownloadFile {
            name: "Reasoning LLM",
            url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
        });

        // Only add LLVM if not installed (skip on macOS - use brew instead)
        #[cfg(not(target_os = "macos"))]
        if !llvm_installed {
            let llvm_url = resolve_llvm_download_url(&client).await?;
            let llvm_filename = if llvm_url.ends_with(".zip") {
                "llvm.zip"
            } else {
                "llvm.tar.xz"
            };
            files.push(DownloadFile {
                name: "LLVM",
                url: llvm_url,
                filename: llvm_filename.to_string(),
            });
        }
        
        // macOS: LLVM will be installed via brew in validator/extractor
        #[cfg(target_os = "macos")]
        if !llvm_installed {
            println!("[Setup] macOS detected - LLVM will be installed via Homebrew");
        }

        files.retain(|f| !f.url.is_empty());
        Ok(files)
    }

    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
}

const LLVM_RELEASE_TAG: &str = "llvmorg-21.1.8";

async fn resolve_llvm_download_url(client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    let release_page = format!(
        "https://github.com/llvm/llvm-project/releases/tag/{LLVM_RELEASE_TAG}"
    );
    let html = client.get(release_page).send().await?.text().await?;

    let href_re = Regex::new(&format!(
        r#"href=\"(?P<href>[^\"]*releases/download/{}/[^\"]+)\""#,
        regex::escape(LLVM_RELEASE_TAG)
    ))?;

    let mut candidates: Vec<String> = href_re
        .captures_iter(&html)
        .filter_map(|cap| cap.name("href").map(|m| m.as_str().to_string()))
        .map(|href| {
            if href.starts_with("http") {
                href
            } else {
                format!("https://github.com{href}")
            }
        })
        .filter(|url| {
            (url.contains("clang%2Bllvm-") || url.contains("clang+llvm-"))
                && (url.ends_with(".tar.xz") || url.ends_with(".zip"))
                && !url.ends_with(".sig")
                && !url.ends_with(".jsonl")
        })
        .collect();

    candidates.sort();

    // Platform and architecture detection
    let platform_predicate: Box<dyn Fn(&str) -> bool> = if cfg!(target_os = "windows") {
        // Windows: detect 32-bit vs 64-bit
        if cfg!(target_arch = "x86_64") {
            // 64-bit Windows
            Box::new(|u| {
                (u.contains("windows") || u.contains("pc-windows"))
                    && (u.contains("x86_64") || u.contains("win64") || u.contains("amd64") || u.contains("X64"))
                    && !u.contains("i686") && !u.contains("win32") && !u.contains("i386")
            })
        } else if cfg!(target_arch = "x86") {
            // 32-bit Windows
            Box::new(|u| {
                (u.contains("windows") || u.contains("pc-windows"))
                    && (u.contains("i686") || u.contains("win32") || u.contains("i386") || u.contains("x86"))
                    && !u.contains("x86_64") && !u.contains("win64") && !u.contains("amd64")
            })
        } else if cfg!(target_arch = "aarch64") {
            // ARM64 Windows
            Box::new(|u| {
                (u.contains("windows") || u.contains("pc-windows"))
                    && (u.contains("aarch64") || u.contains("arm64"))
            })
        } else {
            // Fallback to 64-bit
            Box::new(|u| u.contains("windows") || u.contains("pc-windows"))
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            Box::new(|u| u.contains("linux") && (u.contains("aarch64") || u.contains("arm64")))
        } else if cfg!(target_arch = "x86_64") {
            Box::new(|u| u.contains("linux") && (u.contains("x86_64") || u.contains("amd64")))
        } else if cfg!(target_arch = "x86") {
            Box::new(|u| u.contains("linux") && (u.contains("i686") || u.contains("i386")))
        } else {
            Box::new(|u| u.contains("linux") && u.contains("x86_64"))
        }
    } else {
        // macOS - shouldn't reach here as we skip LLVM download on macOS
        if cfg!(target_arch = "aarch64") {
            Box::new(|u| u.contains("apple-darwin") && (u.contains("arm64") || u.contains("aarch64")))
        } else {
            Box::new(|u| u.contains("apple-darwin") && u.contains("x86_64"))
        }
    };

    // Prefer .tar.xz over .zip
    let tar_xz = candidates
        .iter()
        .find(|u| platform_predicate(u.as_str()) && u.ends_with(".tar.xz"))
        .cloned();
    if let Some(u) = tar_xz {
        return Ok(u);
    }

    let zip = candidates
        .iter()
        .find(|u| platform_predicate(u.as_str()) && u.ends_with(".zip"))
        .cloned();
    if let Some(u) = zip {
        return Ok(u);
    }

    Err(format!(
        "Failed to find LLVM binary archive for this platform in {LLVM_RELEASE_TAG}"
    )
    .into())
}

// Add this function to check if LLVM is installed
fn is_llvm_installed() -> bool {
    if let Ok(output) = Command::new("llvm-config").arg("--version").output() {
        output.status.success()
    } else {
        false
    }
}

/// Get FFmpeg download URL for current platform
fn get_ffmpeg_url() -> String {
    if cfg!(target_os = "windows") {
        // Windows - use gyan.dev builds (reliable, up-to-date)
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip".to_string()
    } else if cfg!(target_os = "linux") {
        // Linux - static build
        "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz".to_string()
    } else if cfg!(target_os = "macos") {
        // macOS - will use brew instead
        String::new()
    } else {
        String::new()
    }
}

/// Get FFmpeg filename for current platform
fn get_ffmpeg_filename() -> String {
    if cfg!(target_os = "windows") {
        "ffmpeg.zip".to_string()
    } else if cfg!(target_os = "linux") {
        "ffmpeg.tar.xz".to_string()
    } else {
        String::new()
    }
}