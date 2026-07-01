use crate::eco::constants::*;
use crate::eco::device::EcoDevice;
use crate::eco::errors::{EcoError, EcoResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub source_device: String,
    pub timestamp: i64,
    pub content_hash: String,
    pub image_data: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: String,
    pub notification_id: String,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub source_device: String,
    pub source_device_name: String,
    pub timestamp: i64,
    pub reply_allowed: bool,
    pub reply_id: Option<String>,
    pub replied: bool,
    pub read: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcoStore {
    pub known_devices: HashMap<String, EcoDevice>,
    pub trusted_device_ids: Vec<String>,
    pub clipboard_history: Vec<ClipboardEntry>,
    pub notification_history: Vec<NotificationEntry>,
}

impl EcoStore {
    pub fn new() -> Self {
        Self {
            known_devices: HashMap::new(),
            trusted_device_ids: Vec::new(),
            clipboard_history: Vec::new(),
            notification_history: Vec::new(),
        }
    }
}

pub struct EcoStorage {
    storage_dir: PathBuf,
    config_path: PathBuf,
    trusted_devices_path: PathBuf,
    clipboard_history_path: PathBuf,
    notification_history_path: PathBuf,
    store: EcoStore,
}

impl EcoStorage {
    pub fn new(pkg_dir: &PathBuf) -> Self {
        let storage_dir = pkg_dir.join(ECO_STORAGE_DIR);
        let config_path = storage_dir.join(ECO_CONFIG_FILE);
        let trusted_devices_path = storage_dir.join(TRUSTED_DEVICES_FILE);
        let clipboard_history_path = storage_dir.join(CLIPBOARD_HISTORY_FILE);
        let notification_history_path = storage_dir.join(NOTIFICATION_HISTORY_FILE);

        let store = Self::load_or_default(&config_path, &trusted_devices_path, &clipboard_history_path, &notification_history_path);

        Self {
            storage_dir,
            config_path,
            trusted_devices_path,
            clipboard_history_path,
            notification_history_path,
            store,
        }
    }

    fn load_or_default(
        config_path: &PathBuf,
        trusted_path: &PathBuf,
        clipboard_path: &PathBuf,
        notification_path: &PathBuf,
    ) -> EcoStore {
        let mut store = EcoStore::new();

        if let Ok(data) = std::fs::read_to_string(trusted_path) {
            if let Ok(ids) = serde_json::from_str::<Vec<String>>(&data) {
                store.trusted_device_ids = ids;
            }
        }

        if let Ok(data) = std::fs::read_to_string(clipboard_path) {
            if let Ok(history) = serde_json::from_str::<Vec<ClipboardEntry>>(&data) {
                store.clipboard_history = history;
            }
        }

        if let Ok(data) = std::fs::read_to_string(notification_path) {
            if let Ok(history) = serde_json::from_str::<Vec<NotificationEntry>>(&data) {
                store.notification_history = history;
            }
        }

        store
    }

    pub fn init_dirs(&self) -> EcoResult<()> {
        std::fs::create_dir_all(&self.storage_dir).map_err(EcoError::Io)
    }

    pub fn get_known_devices(&self) -> &HashMap<String, EcoDevice> {
        &self.store.known_devices
    }

    pub fn get_known_devices_mut(&mut self) -> &mut HashMap<String, EcoDevice> {
        &mut self.store.known_devices
    }

    pub fn is_device_trusted(&self, device_id: &str) -> bool {
        self.store.trusted_device_ids.contains(&device_id.to_string())
    }

    pub fn trust_device(&mut self, device_id: &str) -> EcoResult<()> {
        if !self.store.trusted_device_ids.contains(&device_id.to_string()) {
            self.store.trusted_device_ids.push(device_id.to_string());
            self.save_trusted_devices()?;
        }

        if let Some(device) = self.store.known_devices.get_mut(device_id) {
            device.mark_trusted();
        }

        Ok(())
    }

    pub fn untrust_device(&mut self, device_id: &str) -> EcoResult<()> {
        self.store.trusted_device_ids.retain(|id| id != device_id);
        self.save_trusted_devices()
    }

    pub fn add_clipboard_entry(&mut self, entry: ClipboardEntry) -> EcoResult<()> {
        self.store.clipboard_history.push(entry);
        if self.store.clipboard_history.len() > CLIPBOARD_HISTORY_MAX {
            self.store.clipboard_history.remove(0);
        }
        self.save_clipboard_history()
    }

    pub fn get_clipboard_history(&self) -> &Vec<ClipboardEntry> {
        &self.store.clipboard_history
    }

    fn save_trusted_devices(&self) -> EcoResult<()> {
        let data = serde_json::to_string_pretty(&self.store.trusted_device_ids)
            .map_err(EcoError::Serde)?;
        std::fs::write(&self.trusted_devices_path, &data).map_err(EcoError::Io)
    }

    fn save_clipboard_history(&self) -> EcoResult<()> {
        let data = serde_json::to_string_pretty(&self.store.clipboard_history)
            .map_err(EcoError::Serde)?;
        std::fs::write(&self.clipboard_history_path, &data).map_err(EcoError::Io)
    }

    pub fn update_device(&mut self, device: EcoDevice) {
        let id = device.id.to_string();
        self.store.known_devices.insert(id, device);
    }

    pub fn remove_device(&mut self, device_id: &str) {
        self.store.known_devices.remove(device_id);
    }

    pub fn get_trusted_device_ids(&self) -> &Vec<String> {
        &self.store.trusted_device_ids
    }

    pub fn add_notification_entry(&mut self, entry: NotificationEntry) -> EcoResult<()> {
        self.store.notification_history.push(entry);
        if self.store.notification_history.len() > NOTIFICATION_HISTORY_MAX {
            self.store.notification_history.remove(0);
        }
        self.save_notification_history()
    }

    pub fn get_notification_history(&self) -> &Vec<NotificationEntry> {
        &self.store.notification_history
    }

    pub fn get_notification_history_mut(&mut self) -> &mut Vec<NotificationEntry> {
        &mut self.store.notification_history
    }

    fn save_notification_history(&self) -> EcoResult<()> {
        let data = serde_json::to_string_pretty(&self.store.notification_history)
            .map_err(EcoError::Serde)?;
        std::fs::write(&self.notification_history_path, &data).map_err(EcoError::Io)
    }
}
