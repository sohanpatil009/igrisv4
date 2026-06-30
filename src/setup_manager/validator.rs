// src/setup_manager/validator.rs
// Validates that all required files are present and properly structured

use std::path::PathBuf;
use dirs;

pub struct SetupValidator {
    pkg_dir: PathBuf,
}

impl SetupValidator {
    pub fn new(pkg_dir: PathBuf) -> Self {
        Self { pkg_dir }
    }

    fn llvm_install_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from("C:/LLVM")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".igris")
                .join("llvm")
        }
    }

    pub fn validate_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_llvm()?;
        self.validate_piper()?;
        self.validate_sensevoice_model()?;
        self.validate_voice_model()?;
        self.validate_directories()?;

        println!("✅ All setup validations passed!");
        Ok(())
    }

    fn validate_llvm(&self) -> Result<(), Box<dyn std::error::Error>> {
        let llvm_dir = Self::llvm_install_dir();
        let bin_dir = llvm_dir.join("bin");

        let clang_name = if cfg!(target_os = "windows") {
            "clang.exe"
        } else {
            "clang"
        };

        let clang_bin = bin_dir.join(clang_name);
        if !clang_bin.exists() {
            return Err(format!(
                "LLVM clang not found. Checked: {}",
                clang_bin.display()
            )
            .into());
        }

        println!("  ✓ LLVM clang found at: {}", clang_bin.display());
        println!("✅ LLVM validated");
        Ok(())
    }

    fn validate_piper(&self) -> Result<(), Box<dyn std::error::Error>> {
        let piper_dir = self.pkg_dir.join("piper");

        if !piper_dir.exists() {
            return Err("Piper directory not found".into());
        }

        let piper_name = if cfg!(target_os = "windows") {
            "piper.exe"
        } else {
            "piper"
        };

        let piper_bin = piper_dir.join(piper_name);
        if !piper_bin.exists() {
            return Err(format!("{} not found", piper_name).into());
        }
        println!("  ✓ Piper found at: {}", piper_bin.display());

        println!("✅ Piper TTS validated");
        Ok(())
    }

    fn validate_sensevoice_model(&self) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = self.pkg_dir.join("models/sense-voice/model.onnx");
        let tokens_path = self.pkg_dir.join("models/sense-voice/tokens.txt");

        if !model_path.exists() {
            return Err("SenseVoice model not found (models/sense-voice/model.onnx)".into());
        }
        if !tokens_path.exists() {
            return Err("SenseVoice tokens not found (models/sense-voice/tokens.txt)".into());
        }

        let metadata = std::fs::metadata(&model_path)?;
        let size = metadata.len();

        if size < 1_000_000 {
            return Err("SenseVoice model file is too small (corrupted?)".into());
        }

        println!(
            "✅ SenseVoice model validated: {:.2} MB",
            size as f64 / 1_000_000.0
        );
        Ok(())
    }

    fn validate_voice_model(&self) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = self
            .pkg_dir
            .join("models/bold_voice/en_US-libritts_r-medium.onnx");
        let config_path = self
            .pkg_dir
            .join("models/bold_voice/en_US-libritts_r-medium.onnx.json");

        if !model_path.exists() {
            return Err("Voice model (en_US-libritts_r-medium.onnx) not found".into());
        }

        if !config_path.exists() {
            return Err("Voice model config (.onnx.json) not found".into());
        }

        let metadata = std::fs::metadata(&model_path)?;
        let size = metadata.len();

        if size < 10_000_000 {
            // Less than 10MB = invalid
            return Err("Voice model file is too small (corrupted?)".into());
        }

        println!(
            "✅ Voice model validated ({:.2} MB)",
            size as f64 / 1_000_000.0
        );
        Ok(())
    }

    fn validate_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dirs = vec![
            self.pkg_dir.join("audio"),
            self.pkg_dir.join("models"),
            self.pkg_dir.join("models/bold_voice"),
            self.pkg_dir.join("models/sense-voice"),
        ];

        for dir in dirs {
            if !dir.exists() {
                std::fs::create_dir_all(&dir)?;
            }
        }

        println!("✅ All required directories validated");
        Ok(())
    }

    /// Get the validated Piper path (cross-platform)
    pub fn get_piper_path(&self) -> Option<PathBuf> {
        let binary_name = if cfg!(target_os = "windows") {
            "piper.exe"
        } else {
            "piper"
        };

        let path = self.pkg_dir.join("piper").join(binary_name);
        if path.exists() {
            return Some(path);
        }
        None
    }
}
