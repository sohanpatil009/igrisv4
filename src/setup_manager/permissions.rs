// src/setup_manager/permissions.rs - Runtime permissions system for IGRIS modules
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Result};

/// Module permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    Pending,
}

/// Individual module permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModulePermission {
    pub name: String,
    pub description: String,
    pub status: PermissionStatus,
    pub required: bool,
    pub download_size_mb: f32,
}

/// All module permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    pub modules: HashMap<String, ModulePermission>,
    pub config_path: PathBuf,
}

impl PermissionsConfig {
    /// Create default permissions configuration
    pub fn default_config() -> Self {
        let mut modules = HashMap::new();

        modules.insert(
            "sensevoice_stt".to_string(),
            ModulePermission {
                name: "Speech Recognition (SenseVoice)".to_string(),
                description: "Converts speech to text using SenseVoice f32 model via sherpa-onnx (~940MB)".to_string(),
                status: PermissionStatus::Pending,
                required: true,
                download_size_mb: 940.0,
            },
        );

        modules.insert(
            "sbert_nlu".to_string(),
            ModulePermission {
                name: "Semantic NLU (SBERT)".to_string(),
                description: "Understands intent from text using SBERT model (~80MB)".to_string(),
                status: PermissionStatus::Pending,
                required: true,
                download_size_mb: 80.0,
            },
        );

        modules.insert(
            "piper_tts".to_string(),
            ModulePermission {
                name: "Text-to-Speech (Piper)".to_string(),
                description: "Converts text to speech using Piper TTS (~50MB)".to_string(),
                status: PermissionStatus::Pending,
                required: true,
                download_size_mb: 50.0,
            },
        );

        modules.insert(
            "vad_detection".to_string(),
            ModulePermission {
                name: "Voice Activity Detection".to_string(),
                description: "Detects speech in audio stream (no download required)".to_string(),
                status: PermissionStatus::Pending,
                required: true,
                download_size_mb: 0.0,
            },
        );

        modules.insert(
            "camera_module".to_string(),
            ModulePermission {
                name: "Camera Access".to_string(),
                description: "Enables camera capture and image processing (~20MB)".to_string(),
                status: PermissionStatus::Pending,
                required: false,
                download_size_mb: 20.0,
            },
        );

        modules.insert(
            "file_operations".to_string(),
            ModulePermission {
                name: "File Operations".to_string(),
                description: "Allows file creation, deletion, and management (no download)".to_string(),
                status: PermissionStatus::Pending,
                required: false,
                download_size_mb: 0.0,
            },
        );

        modules.insert(
            "app_launcher".to_string(),
            ModulePermission {
                name: "Application Launcher".to_string(),
                description: "Enables launching and closing applications (no download)".to_string(),
                status: PermissionStatus::Pending,
                required: false,
                download_size_mb: 0.0,
            },
        );

        Self {
            modules,
            config_path: PathBuf::from("./pkg/permissions.json"),
        }
    }

    /// Load permissions from file or create default
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from("./pkg/permissions.json");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Self = serde_json::from_str(&content)?;
            config.config_path = config_path;
            Ok(config)
        } else {
            let config = Self::default_config();
            config.save()?;
            Ok(config)
        }
    }

    /// Save permissions to file
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(self.config_path.parent().unwrap())?;
        let content = serde_json::to_string_pretty(&self.modules)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Grant permission for a module
    pub fn grant_permission(&mut self, module: &str) -> Result<()> {
        self.modules
            .get_mut(module)
            .ok_or_else(|| anyhow!("Module not found: {}", module))?
            .status = PermissionStatus::Granted;
        self.save()?;
        Ok(())
    }

    /// Deny permission for a module
    pub fn deny_permission(&mut self, module: &str) -> Result<()> {
        self.modules
            .get_mut(module)
            .ok_or_else(|| anyhow!("Module not found: {}", module))?
            .status = PermissionStatus::Denied;
        self.save()?;
        Ok(())
    }

    /// Check if module is permitted
    pub fn is_permitted(&self, module: &str) -> bool {
        self.modules
            .get(module)
            .map(|p| p.status == PermissionStatus::Granted)
            .unwrap_or(false)
    }

    /// Get all pending permissions
    pub fn get_pending(&self) -> Vec<(&str, &ModulePermission)> {
        self.modules
            .iter()
            .filter(|(_, p)| p.status == PermissionStatus::Pending)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Get all granted permissions
    pub fn get_granted(&self) -> Vec<(&str, &ModulePermission)> {
        self.modules
            .iter()
            .filter(|(_, p)| p.status == PermissionStatus::Granted)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Get all denied permissions
    pub fn get_denied(&self) -> Vec<(&str, &ModulePermission)> {
        self.modules
            .iter()
            .filter(|(_, p)| p.status == PermissionStatus::Denied)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Calculate total download size for granted modules
    pub fn total_download_size(&self) -> f32 {
        self.modules
            .values()
            .filter(|p| p.status == PermissionStatus::Granted)
            .map(|p| p.download_size_mb)
            .sum()
    }

    /// Get all required modules that are not granted
    pub fn get_missing_required(&self) -> Vec<(&str, &ModulePermission)> {
        self.modules
            .iter()
            .filter(|(_, p)| p.required && p.status != PermissionStatus::Granted)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Check if all required modules are granted
    pub fn all_required_granted(&self) -> bool {
        self.get_missing_required().is_empty()
    }
}

/// Permission request dialog state
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub module_id: String,
    pub module_name: String,
    pub description: String,
    pub required: bool,
    pub download_size_mb: f32,
}

impl PermissionRequest {
    pub fn from_permission(id: &str, perm: &ModulePermission) -> Self {
        Self {
            module_id: id.to_string(),
            module_name: perm.name.clone(),
            description: perm.description.clone(),
            required: perm.required,
            download_size_mb: perm.download_size_mb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permissions() {
        let config = PermissionsConfig::default_config();
        assert!(!config.modules.is_empty());
        assert!(config.modules.contains_key("sensevoice_stt"));
        assert!(config.modules.contains_key("sbert_nlu"));
    }

    #[test]
    fn test_grant_permission() {
        let mut config = PermissionsConfig::default_config();
        config.grant_permission("sensevoice_stt").unwrap();
        assert!(config.is_permitted("sensevoice_stt"));
    }

    #[test]
    fn test_pending_permissions() {
        let config = PermissionsConfig::default_config();
        let pending = config.get_pending();
        assert!(!pending.is_empty());
    }

    #[test]
    fn test_total_download_size() {
        let mut config = PermissionsConfig::default_config();
        config.grant_permission("sensevoice_stt").unwrap();
        config.grant_permission("sbert_nlu").unwrap();
        let size = config.total_download_size();
        assert!(size > 0.0);
    }
}
