use crate::eco::constants::*;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::events::{EcoEvent, EventBus};
use crate::platform::ecosystem::notifications::{create_platform_notification, PlatformNotification, RawNotification};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// A notification synced from any device in the ecosystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationData {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub device_name: String,
    pub device_id: String,
    pub timestamp: i64,
    pub read: bool,
    pub replied: bool,
}

/// Reply to send back through the originating app.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationReply {
    pub notification_id: String,
    pub reply_text: String,
    pub source_device_id: String,
}

/// Persistent notification history stored under pkg/ecosystem/.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NotificationStore {
    pub notifications: Vec<NotificationData>,
}

impl NotificationStore {
    fn load(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// Manages local notification polling, storage, and cross-device sync.
pub struct NotificationManager {
    platform: Box<dyn PlatformNotification>,
    event_bus: Arc<EventBus>,
    storage_path: PathBuf,
    store: NotificationStore,
    local_device_id: String,
    local_device_name: String,
}

impl NotificationManager {
    pub fn new(
        event_bus: Arc<EventBus>,
        pkg_dir: &PathBuf,
        local_device_id: String,
        local_device_name: String,
    ) -> Self {
        let storage_path = pkg_dir
            .join(ECO_STORAGE_DIR)
            .join(NOTIFICATION_HISTORY_FILE);
        let store = NotificationStore::load(&storage_path);
        let platform = create_platform_notification();

        Self {
            platform,
            event_bus,
            storage_path,
            store,
            local_device_id,
            local_device_name,
        }
    }

    /// Poll local notifications and emit events for new ones.
    pub fn poll_local(&mut self) -> Vec<NotificationData> {
        let raw = match self.platform.read_notifications() {
            Ok(n) => n,
            Err(_) => return Vec::new(),
        };

        let mut new_notifications = Vec::new();

        for raw_notif in raw {
            // Deduplicate by checking if we already have this notification
            let already_exists = self.store.notifications.iter().any(|n| {
                n.app_name == raw_notif.app_name
                    && n.title == raw_notif.title
                    && n.body == raw_notif.body
                    && n.timestamp > chrono::Utc::now().timestamp_millis() - 60_000
            });

            if !already_exists {
                let notif = NotificationData {
                    id: raw_notif.id,
                    app_name: raw_notif.app_name,
                    title: raw_notif.title,
                    body: raw_notif.body,
                    device_name: self.local_device_name.clone(),
                    device_id: self.local_device_id.clone(),
                    timestamp: raw_notif.timestamp,
                    read: false,
                    replied: false,
                };

                self.store.notifications.insert(0, notif.clone());
                new_notifications.push(notif.clone());

                self.event_bus
                    .emit(EcoEvent::NotificationReceived(notif, self.local_device_id.clone()));
            }
        }

        // Trim history to max
        if self.store.notifications.len() > NOTIFICATION_HISTORY_MAX {
            self.store
                .notifications
                .truncate(NOTIFICATION_HISTORY_MAX);
        }

        self.store.save(&self.storage_path);
        new_notifications
    }

    /// Receive a notification from a remote device.
    pub fn receive_remote(&mut self, notif: NotificationData) {
        let already_exists = self.store.notifications.iter().any(|n| n.id == notif.id);

        if !already_exists {
            self.store.notifications.insert(0, notif.clone());
            self.event_bus
                .emit(EcoEvent::NotificationReceived(notif, "remote".to_string()));
        }

        if self.store.notifications.len() > NOTIFICATION_HISTORY_MAX {
            self.store
                .notifications
                .truncate(NOTIFICATION_HISTORY_MAX);
        }

        self.store.save(&self.storage_path);
    }

    /// Reply to a notification locally through the source app.
    pub fn reply_to_notification(&self, notification_id: &str, reply_text: &str) -> EcoResult<()> {
        self.platform.reply_to_notification(notification_id, reply_text)
    }

    /// Get all notifications (from UI).
    pub fn get_notifications(&self) -> Vec<NotificationData> {
        self.store.notifications.clone()
    }

    /// Get unread count.
    pub fn unread_count(&self) -> usize {
        self.store.notifications.iter().filter(|n| !n.read).count()
    }

    /// Mark a notification as read.
    pub fn mark_read(&mut self, notification_id: &str) {
        if let Some(n) = self
            .store
            .notifications
            .iter_mut()
            .find(|n| n.id == notification_id)
        {
            n.read = true;
        }
        self.store.save(&self.storage_path);
    }

    /// Clear all notifications.
    pub fn clear_all(&mut self) {
        self.store.notifications.clear();
        self.store.save(&self.storage_path);
    }

    /// Check if platform notification access is available.
    pub fn has_permission(&self) -> bool {
        self.platform.has_permission()
    }

    /// Request platform notification access.
    pub fn request_permission(&self) -> EcoResult<()> {
        self.platform.request_permission()
    }

    /// Set local device identity (called after initialization).
    pub fn set_device_info(&mut self, id: String, name: String) {
        self.local_device_id = id;
        self.local_device_name = name;
    }
}

lazy_static::lazy_static! {
    pub static ref NOTIFICATION_HISTORY: std::sync::Mutex<Option<Vec<NotificationData>>> =
        std::sync::Mutex::new(None);
}

/// Get notifications from the global store.
pub fn get_notifications() -> Vec<NotificationData> {
    NOTIFICATION_HISTORY
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_default()
}

/// Set notifications in the global store (for UI access).
pub fn set_notifications(notifs: Vec<NotificationData>) {
    if let Ok(mut guard) = NOTIFICATION_HISTORY.lock() {
        *guard = Some(notifs);
    }
}
