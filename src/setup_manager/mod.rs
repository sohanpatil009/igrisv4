// src/setup_manager/mod.rs - Setup system module exports

pub mod downloader;
pub mod extractor;
pub mod gui;
pub mod validator;
pub mod permissions;
pub mod permissions_ui;
pub mod platforms;

// Re-exports for convenience
pub use downloader::FileDownloader;
pub use extractor::FileExtractor;
pub use gui::{SetupUI, SetupGui, is_setup_complete};
pub use validator::SetupValidator;
pub use permissions::PermissionsConfig;
pub use platforms::{PlatformSetup, current_platform, command_exists};

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use dirs;

#[derive(Clone, Debug)]
pub enum SetupEvent {
    Started,
    Downloading { name: String, progress: u32 },
    Extracting { name: String, progress: u32 },
    Validating { name: String },
    Completed { name: String },
    Error { task: String, message: String },
    AllComplete,
}

pub struct SetupManager {
    pkg_dir: PathBuf,
    event_sender: Arc<Mutex<mpsc::UnboundedSender<SetupEvent>>>,
}

impl SetupManager {
    pub fn new(pkg_dir: PathBuf, event_sender: mpsc::UnboundedSender<SetupEvent>) -> Self {
        Self {
            pkg_dir,
            event_sender: Arc::new(Mutex::new(event_sender)),
        }
    }

    pub async fn run_setup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use platform-specific setup
        let platform_setup = platforms::get_platform_setup(
            self.pkg_dir.clone(),
            self.event_sender.clone(),
        );
        
        platform_setup.run_setup().await
    }

    fn send_event(&self, event: SetupEvent) {
        if let Ok(sender) = self.event_sender.lock() {
            let _ = sender.send(event);
        }
    }
}

/// Uninstall function to clean up all setup files
pub fn uninstall_igris() -> Result<String, Box<dyn std::error::Error>> {
    let pkg_dir = PathBuf::from("./pkg");

    if !pkg_dir.exists() {
        return Ok("IGRIS package folder not found. Already uninstalled.".to_string());
    }

    println!("🗑️ Starting IGRIS uninstall...");

    // Remove the entire pkg directory
    match fs::remove_dir_all(&pkg_dir) {
        Ok(_) => {
            println!("✅ Successfully removed: {}", pkg_dir.display());
            Ok(format!(
                "IGRIS has been successfully uninstalled. Folder removed: {}",
                pkg_dir.display()
            ))
        }
        Err(e) => {
            eprintln!("❌ Error removing directory: {}", e);
            Err(format!("Failed to uninstall IGRIS: {}", e).into())
        }
    }
}

/// Clean up temporary files (called during uninstall or maintenance)
pub fn cleanup_temp_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp_files = vec!["./pkg/downloads", "./.last_camera_recording"];

    for file_path in temp_files {
        let path = PathBuf::from(file_path);
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
                println!("🗑️ Removed temporary directory: {}", file_path);
            } else {
                fs::remove_file(&path)?;
                println!("🗑️ Removed temporary file: {}", file_path);
            }
        }
    }

    Ok(())
}

/// Verify installation integrity
pub fn verify_installation() -> Result<bool, Box<dyn std::error::Error>> {
    let pkg_dir = PathBuf::from("./pkg");

    let required_paths = vec![
        pkg_dir.join("piper"),
        pkg_dir.join("models/ggml-base.bin"),
        pkg_dir.join("models/sbert/pytorch_model.bin"),
        pkg_dir.join("models/sbert/config.json"),
        pkg_dir.join("models/sbert/tokenizer.json"),
        pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx"),
        pkg_dir.join("models/bold_voice/en_US-libritts_r-medium.onnx.json"),
    ];

    // The reasoning LLM model is optional — downloaded separately if user wants smart fallback.

    let all_exist = required_paths.iter().all(|p| p.exists());

    if all_exist {
        println!("✅ Installation verified successfully");
        Ok(true)
    } else {
        println!("❌ Installation incomplete. Missing files detected.");
        for path in &required_paths {
            if !path.exists() {
                println!("  Missing: {}", path.display());
            }
        }
        Ok(false)
    }
}

fn llvm_install_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        Some(PathBuf::from("C:/LLVM"))
    } else {
        dirs::home_dir().map(|h| h.join(".igris").join("llvm"))
    }
}

fn llvm_bin_dir() -> Option<PathBuf> {
    llvm_install_dir().map(|p| p.join("bin"))
}

fn ensure_llvm_in_path() -> io::Result<()> {
    let llvm_bin = llvm_bin_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Unable to resolve LLVM bin directory")
    })?;

    if cfg!(target_os = "windows") {
        ensure_llvm_in_path_windows(&llvm_bin)
    } else {
        ensure_llvm_in_path_unix(&llvm_bin)
    }
}

fn ensure_llvm_in_path_unix(llvm_bin: &PathBuf) -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Unable to resolve home directory")
    })?;

    // Best-effort: update a broadly-supported profile file.
    let profile_path = home.join(".profile");
    let marker = llvm_bin.to_string_lossy();
    let export_line = format!("export PATH=\"{}:$PATH\"\n", marker);

    let existing = fs::read_to_string(&profile_path).unwrap_or_default();
    if existing.contains(&marker.to_string()) {
        return Ok(());
    }

    let mut new_contents = existing;
    if !new_contents.ends_with('\n') && !new_contents.is_empty() {
        new_contents.push('\n');
    }
    new_contents.push_str(&export_line);
    fs::write(profile_path, new_contents)
}

fn ensure_llvm_in_path_windows(llvm_bin: &PathBuf) -> io::Result<()> {
    let llvm_bin_str = llvm_bin.to_string_lossy().replace('/', "\\");

    // Attempt Machine PATH first (may require admin), then User PATH.
    let ps_template = r#"$llvmBin = '__LLVM_BIN__';
$targets = @('Machine','User');
foreach ($t in $targets) {
  try {
    $cur = [Environment]::GetEnvironmentVariable('Path', $t);
    if ($null -eq $cur) { $cur = '' }
    $parts = $cur -split ';' | Where-Object { $_ -and $_.Trim() -ne '' }
    if ($parts -notcontains $llvmBin) {
      $new = ($parts + $llvmBin) -join ';';
      [Environment]::SetEnvironmentVariable('Path', $new, $t);
    }
    break;
  } catch { }
}
"#;
    let ps_script = ps_template.replace("__LLVM_BIN__", &llvm_bin_str);

    let out = std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(ps_script)
        .output()?;

    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("PowerShell failed: {}", String::from_utf8_lossy(&out.stderr)),
        ));
    }

    Ok(())
}